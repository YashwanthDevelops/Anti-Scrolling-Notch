//! Anti-Scrolling-Notch relay — inherited Claude hooks plus a separate Codex observer mode.
//!
//! Reads the hook JSON on stdin, adds a little terminal context, and hands it to
//! Anti-Scrolling-Notch over `\\.\pipe\anti-scrolling-notch-<sid>`.
//!
//! Hard rule (docs/CLAUDE.md): **never block Claude Code.**
//! * If the pipe does not exist — Anti-Scrolling-Notch is closed — we exit 0 immediately with
//!   nothing on stdout, and the session carries on untouched.
//! * Every step runs under a deadline enforced by the main thread, so a pipe that
//!   accepts the connection and then stops reading cannot wedge the session
//!   either: we abandon the worker and exit.
//! * Only `PermissionRequest` waits for an answer, because approving from the
//!   island is the whole point. No answer means empty stdout, and Claude Code
//!   asks in the terminal exactly as if Anti-Scrolling-Notch were not installed.
//!
//! Usage: `anti-scrolling-notch-hook <EventName>` for the inherited Claude Code
//! hooks, or `anti-scrolling-notch-hook --codex-observer` for the verified Codex
//! lifecycle subset.

use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anti_scrolling_notch_runtime_identity::{APP_PIPE_PREFIX, CODEX_PIPE_PREFIX};
use codex_hook_contract::{parse_hook_payload, CodexHookObservation, MAX_HOOK_INPUT_BYTES};

/// Budget for getting a pipe connection. Beyond this Claude Code wins, always.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
/// Whole-run budget for an event nobody waits on: connect and write, no more.
const FIRE_AND_FORGET_BUDGET: Duration = Duration::from_secs(2);
/// How long a permission prompt may stay on screen before the terminal takes over.
const DECISION_BUDGET: Duration = Duration::from_secs(110);

/// `ERROR_PIPE_BUSY` — every instance is serving someone else right now. This is
/// the one error worth retrying: the server exists and a slot will free up.
const ERROR_PIPE_BUSY: i32 = 231;

/// Fields that are pointless to forward and can be enormous (a whole file read,
/// a full command output). The island never shows them.
const DROPPED_FIELDS: &[&str] = &["tool_response", "transcript_path"];
/// Longest string forwarded for any single field; the island truncates to far
/// less than this anyway.
const MAX_FIELD_LEN: usize = 2_000;

mod win;

/// `\\.\pipe\anti-scrolling-notch-<sid>`. The SID keeps two accounts on the same machine from
/// ever meeting on the same pipe; no username fallback is used when SID lookup
/// fails.
fn pipe_path() -> Option<String> {
    pipe_path_from_sid(win::current_user_sid())
}

fn pipe_path_for_sid(sid: &str) -> String {
    format!(r"\\.\pipe\{APP_PIPE_PREFIX}-{sid}")
}

fn pipe_path_from_sid(sid: Option<String>) -> Option<String> {
    sid.map(|sid| pipe_path_for_sid(&sid))
}

/// Codex's observing adapter uses a separate backend-only pipe. It must never
/// enter the inherited Claude event listener or its approval/UI handlers.
fn codex_pipe_path() -> Option<String> {
    codex_pipe_path_from_sid(win::current_user_sid())
}

fn codex_pipe_path_for_sid(sid: &str) -> String {
    format!(r"\\.\pipe\{CODEX_PIPE_PREFIX}-{sid}")
}

fn codex_pipe_path_from_sid(sid: Option<String>) -> Option<String> {
    sid.map(|sid| codex_pipe_path_for_sid(&sid))
}

/// Opens the pipe. Retries only while the server is busy: any other error means
/// there is nothing to talk to, and waiting would only delay Claude Code.
fn connect() -> Option<std::fs::File> {
    connect_to(&pipe_path()?)
}

fn connect_to(path: &str) -> Option<std::fs::File> {
    use std::os::windows::io::AsRawHandle;
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    loop {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
        {
            Ok(file) => {
                let handle = windows::Win32::Foundation::HANDLE(file.as_raw_handle());
                // Somebody else's server on our pipe name gets nothing from us.
                return win::pipe_server_is_same_user(handle).then_some(file);
            }
            Err(err) => {
                if err.raw_os_error() != Some(ERROR_PIPE_BUSY) || Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(15));
            }
        }
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--codex-observer") {
        run_codex_observer();
        return;
    }

    let Some((payload, event)) = read_event() else {
        std::process::exit(0)
    };

    let waits_for_answer = event == "PermissionRequest";
    let budget = if waits_for_answer {
        DECISION_BUDGET
    } else {
        FIRE_AND_FORGET_BUDGET
    };

    // The worker owns every blocking call. If it overruns the budget we simply
    // stop listening and exit: the process dying takes the pipe handle with it.
    // (No catch_unwind here — the release profile is panic = "abort", so it would
    // be dead code. `talk` is written to have nothing to panic on instead.)
    let (tx, rx) = mpsc::channel::<Option<String>>();
    std::thread::spawn(move || {
        let _ = tx.send(talk(&payload, waits_for_answer));
    });

    if let Ok(Some(decision)) = rx.recv_timeout(budget) {
        if let Some(json) = decision_json(&decision) {
            let mut out = std::io::stdout();
            let _ = writeln!(out, "{json}");
            let _ = out.flush();
        }
    }
    // Nothing printed: Claude Code asks in the terminal, as if we were not here.
    std::process::exit(0);
}

/// Codex observing hooks are best-effort and neutral: accept only the verified
/// event discriminator, send a tiny versioned message to the Codex-only backend
/// pipe, and never write hook output to stdout or wait for a decision.
fn run_codex_observer() {
    let mut raw = Vec::new();
    if std::io::stdin()
        .take((MAX_HOOK_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut raw)
        .is_err()
    {
        return;
    }
    let Ok(observation) = parse_hook_payload(&raw) else {
        return;
    };
    send_codex_observation(observation);
}

fn send_codex_observation(observation: CodexHookObservation) {
    let Some(pipe_path) = codex_pipe_path() else {
        return;
    };
    let Ok(mut payload) = serde_json::to_vec(&observation) else {
        return;
    };
    payload.push(b'\n');

    // The named-pipe open/write run on a worker so even a wedged app cannot
    // hold the Codex turn. Closing the process drops the pending pipe handle.
    let (tx, rx) = mpsc::channel::<bool>();
    std::thread::spawn(move || {
        let sent = connect_to(&pipe_path)
            .and_then(|mut pipe| pipe.write_all(&payload).ok().map(|()| pipe.flush().is_ok()))
            .unwrap_or(false);
        let _ = tx.send(sent);
    });
    let _ = rx.recv_timeout(FIRE_AND_FORGET_BUDGET);
}

/// The documented PermissionRequest output. Anything we do not recognise prints
/// nothing at all rather than guessing — silence is the safe answer.
/// See https://code.claude.com/docs/en/hooks
fn decision_json(decision: &str) -> Option<String> {
    let behavior = match decision.trim() {
        // "always" still answers a plain allow; remembering it is the island's
        // business, not Claude Code's.
        "allow" | "always" => r#"{"behavior":"allow"}"#.to_string(),
        "deny" => r#"{"behavior":"deny","message":"Denied from Anti-Scrolling-Notch"}"#.to_string(),
        _ => return None,
    };
    Some(format!(
        r#"{{"hookSpecificOutput":{{"hookEventName":"PermissionRequest","decision":{behavior}}}}}"#
    ))
}

/// Reads stdin and returns the payload to forward plus the event name.
fn read_event() -> Option<(String, String)> {
    let mut raw = read_bounded_input(std::io::stdin())?;
    if raw.is_empty() {
        return None;
    }
    // Some shells hand us a UTF-8 BOM; serde_json would choke on it.
    if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        raw.drain(..3);
    }

    let mut payload = serde_json::from_slice::<serde_json::Value>(&raw).ok()?;
    let map = payload.as_object_mut()?;

    // The event name is passed as argv[1] by the hook command; the JSON usually
    // carries it too. Trust argv when the JSON is missing it.
    let arg_event = std::env::args().nth(1).unwrap_or_default();
    let event = map
        .get("hook_event_name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or(arg_event);
    map.insert(
        "hook_event_name".into(),
        serde_json::Value::String(event.clone()),
    );

    for field in DROPPED_FIELDS {
        map.remove(*field);
    }

    let cwd_missing = map
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(str::is_empty)
        .unwrap_or(true);
    if cwd_missing {
        if let Ok(cwd) = std::env::current_dir() {
            map.insert(
                "cwd".into(),
                serde_json::Value::String(cwd.to_string_lossy().to_string()),
            );
        }
    }

    // Which terminal the session runs in. Unlike macOS, Anti-Scrolling-Notch on Windows accepts
    // events from every terminal, so this is context only — never a filter.
    for (key, var) in [
        ("term_program", "TERM_PROGRAM"),
        ("wt_session", "WT_SESSION"),
        ("term_session_id", "TERM_SESSION_ID"),
        ("vscode_pid", "VSCODE_PID"),
        ("session_pid", "CLAUDE_CODE_SSE_PORT"),
    ] {
        if !map.contains_key(key) {
            let value = std::env::var(var).unwrap_or_default();
            map.insert(key.into(), serde_json::Value::String(value));
        }
    }

    truncate_strings(&mut payload);

    let mut line = payload.to_string();
    line.push('\n');
    Some((line, event))
}

fn read_bounded_input(reader: impl Read) -> Option<Vec<u8>> {
    let mut raw = Vec::new();
    reader
        .take((MAX_HOOK_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut raw)
        .ok()?;
    (raw.len() <= MAX_HOOK_INPUT_BYTES).then_some(raw)
}

/// Caps every string in the payload. A single Write can carry a whole file.
fn truncate_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => {
            if s.len() > MAX_FIELD_LEN {
                // Cut on a char boundary; a lone byte index can split UTF-8.
                let mut end = MAX_FIELD_LEN;
                while end > 0 && !s.is_char_boundary(end) {
                    end -= 1;
                }
                s.truncate(end);
                s.push('…');
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(truncate_strings),
        serde_json::Value::Object(map) => map.values_mut().for_each(truncate_strings),
        _ => {}
    }
}

/// Connect, send, and — for a permission request — wait for the island's word.
fn talk(payload: &str, waits_for_answer: bool) -> Option<String> {
    let mut pipe = connect()?;

    if pipe.write_all(payload.as_bytes()).is_err() {
        return None;
    }
    let _ = pipe.flush();

    if !waits_for_answer {
        return None;
    }

    const MAX_DECISION_RESPONSE_BYTES: usize = 64;
    let mut buf = Vec::new();
    let mut chunk = [0u8; MAX_DECISION_RESPONSE_BYTES];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if buf.len().checked_add(n)? > MAX_DECISION_RESPONSE_BYTES {
                    return None;
                }
                buf.extend_from_slice(&chunk[..n]);
                if let Some(end) = buf.iter().position(|byte| *byte == b'\n') {
                    if end + 1 != buf.len() {
                        return None;
                    }
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let answer = String::from_utf8_lossy(&buf).trim().to_string();
    (!answer.is_empty()).then_some(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_pipe_requires_a_sid_and_uses_the_product_namespace() {
        assert!(pipe_path_from_sid(None).is_none());
        assert_eq!(
            pipe_path_from_sid(Some("S-1-5-21-current".into())).as_deref(),
            Some(r"\\.\pipe\anti-scrolling-notch-S-1-5-21-current")
        );
    }

    #[test]
    fn codex_pipe_requires_a_sid_and_uses_no_username_fallback() {
        assert!(codex_pipe_path_from_sid(None).is_none());
        assert_eq!(
            codex_pipe_path_from_sid(Some("S-1-5-21-current".into())).as_deref(),
            Some(r"\\.\pipe\anti-scrolling-notch-codex-S-1-5-21-current")
        );
    }

    #[test]
    fn inherited_hook_input_is_bounded_at_the_shared_contract_limit() {
        use std::io::Cursor;

        assert_eq!(
            read_bounded_input(Cursor::new(vec![b'x'; MAX_HOOK_INPUT_BYTES]))
                .unwrap()
                .len(),
            MAX_HOOK_INPUT_BYTES
        );
        assert!(read_bounded_input(Cursor::new(vec![b'x'; MAX_HOOK_INPUT_BYTES + 1])).is_none());
    }

    #[test]
    fn absent_app_pipe_returns_without_waiting_for_a_listener() {
        use std::time::Instant;

        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let name = format!(
            r"\\.\pipe\anti-scrolling-notch-no-listener-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let started = Instant::now();
        assert!(connect_to(&name).is_none());
        assert!(started.elapsed() < CONNECT_TIMEOUT);
    }

    #[test]
    fn decision_json_matches_the_documented_shape() {
        assert_eq!(
            decision_json("allow").unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}"#
        );
        assert_eq!(
            decision_json("deny").unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Denied from Anti-Scrolling-Notch"}}}"#
        );
        // "always" is an island concept; Claude Code just gets an allow.
        assert!(decision_json("always")
            .unwrap()
            .contains(r#""behavior":"allow""#));
    }

    #[test]
    fn anything_unrecognised_prints_nothing() {
        assert!(decision_json("").is_none());
        assert!(decision_json("maybe").is_none());
        // The shape the app used to send must not be mistaken for a decision.
        assert!(decision_json(r#"{"permissionDecision":"allow"}"#).is_none());
    }

    #[test]
    fn long_strings_are_cut_on_a_char_boundary() {
        let mut v = serde_json::json!({ "tool_input": { "content": "é".repeat(4000) } });
        truncate_strings(&mut v);
        let s = v["tool_input"]["content"].as_str().unwrap();
        assert!(s.len() <= MAX_FIELD_LEN + 4);
        assert!(s.ends_with('…'));
    }
}
