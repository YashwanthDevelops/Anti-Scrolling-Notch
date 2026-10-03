// Bounded named-pipe server for anti-scrolling-notch-hook.
//
// The SID-scoped pipe uses a fixed listener pool. Every hook event is forwarded
// to the island as a `hook` event. `PermissionRequest` is the only one that keeps
// its connection open: it waits for the island's decision and writes it back on
// the same pipe, which is how approving from the island works.
//
// Claude Code is never blocked by us. Three things guarantee it:
//   * anti-scrolling-notch-hook gives the connection 300 ms and exits cleanly if we are closed;
//   * we only wait for a human once the island has *confirmed* the card is on
//     screen, so a paused island or a webview that is not listening costs a few
//     hundred milliseconds, not two minutes;
//   * whatever happens we drop the connection after the decision timeout, and
//     the terminal takes over.
//
// What we write back is the bare word `allow` or `deny`. Turning that into the
// documented hookSpecificOutput JSON is anti-scrolling-notch-hook's job, so the wire format
// Claude Code expects lives in exactly one place.

use std::collections::HashMap;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use codex_hook_contract::MAX_HOOK_INPUT_BYTES;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};
use windows::Win32::Foundation::HANDLE;

use crate::island::WINDOW_LABEL;
use crate::log;

/// Slightly under anti-scrolling-notch-hook's own 110 s wait, so we always answer first.
const DECISION_TIMEOUT: Duration = Duration::from_secs(108);
/// How long the island gets to say "the card is up". This is the whole of B4:
/// without it, an island that is paused, hidden behind a crashed webview or
/// simply not listening would leave Claude Code staring at a prompt nobody can
/// see for nearly two minutes.
const ACK_TIMEOUT: Duration = Duration::from_millis(800);
/// Bound pipe input independently from the long permission decision wait.
const READ_TIMEOUT: Duration = Duration::from_secs(2);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_PAYLOAD: usize = MAX_HOOK_INPUT_BYTES;
const LISTENER_RETRY: Duration = Duration::from_millis(200);
const MAX_LISTENERS: usize = 8;
const MAX_ACTIVE_CLIENTS: usize = 8;
const MAX_PIPE_INSTANCES: usize = MAX_LISTENERS + MAX_ACTIVE_CLIENTS;

/// What the island can say about a permission request.
pub enum Reply {
    /// The card is on screen and a human can act on it.
    Ack,
    /// A human clicked: `allow` or `deny`.
    Decision(String),
    /// Nobody can act on it — paused, or another request already holds the card.
    Decline,
}

/// Permission requests the island has been told about.
#[derive(Default)]
pub struct Pending(pub Mutex<HashMap<String, mpsc::Sender<Reply>>>);

static COUNTER: AtomicU64 = AtomicU64::new(1);

/// `\\.\pipe\anti-scrolling-notch-<sid>` — must match the relay's `pipe_path()` exactly.
fn pipe_name_for_sid(sid: &str) -> String {
    format!(r"\\.\pipe\{}-{sid}", crate::identity::APP_PIPE_PREFIX)
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Some(sid) = crate::win_user::current_user_sid() else {
            log::line("hook relay disabled: could not resolve current-user SID");
            return;
        };
        let name = pipe_name_for_sid(&sid);
        let listeners = match create_listener_pool(&name, &sid) {
            Ok(listeners) => listeners,
            Err(err) => {
                log::line(format!("cannot open private hook relay pipe: {err}"));
                return;
            }
        };

        let permits = Arc::new(Semaphore::new(MAX_ACTIVE_CLIENTS));
        for server in listeners {
            let name = name.clone();
            let sid = sid.clone();
            let permits = Arc::clone(&permits);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                accept_loop(server, name, sid, permits, app).await;
            });
        }
    });
}

fn create_listener_pool(name: &str, sid: &str) -> io::Result<Vec<NamedPipeServer>> {
    (0..MAX_LISTENERS)
        .map(|index| create_server(name, sid, index == 0))
        .collect()
}

fn create_server(name: &str, sid: &str, first: bool) -> io::Result<NamedPipeServer> {
    crate::private_pipe::create_server(name, sid, first, MAX_PIPE_INSTANCES)
}

async fn accept_loop(
    mut server: NamedPipeServer,
    name: String,
    sid: String,
    permits: Arc<Semaphore>,
    app: AppHandle,
) {
    loop {
        if server.connect().await.is_err() {
            let _ = server.disconnect();
            tokio::time::sleep(LISTENER_RETRY).await;
            continue;
        }

        let Some(permit) = acquire_client_permit(&permits) else {
            // A saturated relay declines the request so the upstream terminal
            // can handle permission prompts, and drops non-blocking events.
            let _ = server.disconnect();
            continue;
        };

        let next = match create_server(&name, &sid, false) {
            Ok(next) => next,
            Err(err) => {
                log::line(format!("hook relay listener recovering: {err}"));
                let _ = server.disconnect();
                drop(permit);
                tokio::time::sleep(LISTENER_RETRY).await;
                continue;
            }
        };

        let connected = std::mem::replace(&mut server, next);
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            handle(app, connected, permit).await;
        });
    }
}

fn acquire_client_permit(permits: &Arc<Semaphore>) -> Option<OwnedSemaphorePermit> {
    Arc::clone(permits).try_acquire_owned().ok()
}

async fn receive_payload(pipe: &mut NamedPipeServer) -> Option<Value> {
    let bytes = tokio::time::timeout(READ_TIMEOUT, read_message(pipe))
        .await
        .ok()??;
    // Impersonation is synchronous and occurs only after the bounded async
    // read. No untrusted JSON is parsed or logged before the SID check.
    if !crate::win_user::pipe_client_is_same_user(HANDLE(pipe.as_raw_handle())) {
        return None;
    }
    let payload = serde_json::from_slice::<Value>(&bytes).ok()?;
    payload.is_object().then_some(payload)
}

async fn read_message(pipe: &mut NamedPipeServer) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        match pipe.read(&mut chunk).await {
            Ok(0) => return (!bytes.is_empty()).then_some(bytes),
            Ok(n) => {
                let max_framed_size = MAX_PAYLOAD.checked_add(1)?;
                if bytes.len().checked_add(n)? > max_framed_size {
                    return None;
                }
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                    // Keep the established newline-delimited hook message and
                    // reject an extra frame carried in the same pipe write.
                    if end > MAX_PAYLOAD || end + 1 != bytes.len() {
                        return None;
                    }
                    return Some(bytes[..end].to_vec());
                }
                if bytes.len() > MAX_PAYLOAD {
                    return None;
                }
            }
            Err(_) => return None,
        }
    }
}

async fn handle(app: AppHandle, mut pipe: NamedPipeServer, _permit: OwnedSemaphorePermit) {
    let Some(mut payload) = receive_payload(&mut pipe).await else {
        let _ = pipe.disconnect();
        return;
    };

    let event = payload
        .get("hook_event_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    if event != "PermissionRequest" {
        log::line(format!("hook {}", safe_event_name(&event)));
        let _ = app.emit_to(WINDOW_LABEL, "hook", payload);
        let _ = pipe.disconnect();
        return;
    }

    let id = format!(
        "{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let (tx, mut rx) = mpsc::channel::<Reply>(4);
    {
        let pending = app.state::<Pending>();
        pending.0.lock().unwrap().insert(id.clone(), tx);
    }
    payload["request_id"] = json!(id);
    log::line(format!("hook PermissionRequest id={id}"));
    let _ = app.emit_to(WINDOW_LABEL, "hook", payload);

    let decision = wait_for_decision(&id, &mut rx).await;
    app.state::<Pending>().0.lock().unwrap().remove(&id);

    // No decision: say nothing at all. The relay then writes nothing to stdout
    // and Claude Code asks in the terminal, exactly as if the app were closed.
    if let Some(d) = decision {
        let response = format!("{d}\n");
        let result = tokio::time::timeout(RESPONSE_TIMEOUT, async {
            pipe.write_all(response.as_bytes()).await?;
            pipe.flush().await
        })
        .await;
        if !matches!(result, Ok(Ok(()))) {
            log::line(format!("hook id={id} response delivery failed"));
        }
    }
    let _ = pipe.disconnect();
}

fn safe_event_name(event: &str) -> String {
    event
        .chars()
        .take(64)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
                character
            } else {
                '?'
            }
        })
        .collect()
}

/// Two waits: a short one for "the card is up", then the long one for a human.
async fn wait_for_decision(id: &str, rx: &mut mpsc::Receiver<Reply>) -> Option<String> {
    match tokio::time::timeout(ACK_TIMEOUT, rx.recv()).await {
        Ok(Some(Reply::Ack)) => {}
        // A click that beats the ack is still a click.
        Ok(Some(Reply::Decision(d))) => {
            log::line(format!("hook id={id} answered {d}"));
            return Some(d);
        }
        Ok(Some(Reply::Decline)) => {
            log::line(format!("hook id={id} not shown — terminal takes over"));
            return None;
        }
        Ok(None) => return None,
        Err(_) => {
            log::line(format!(
                "hook id={id} island never acknowledged — terminal takes over"
            ));
            return None;
        }
    }

    match tokio::time::timeout(DECISION_TIMEOUT, rx.recv()).await {
        Ok(Some(Reply::Decision(d))) => {
            log::line(format!("hook id={id} answered {d}"));
            Some(d)
        }
        Ok(Some(Reply::Decline)) => {
            log::line(format!("hook id={id} released without a decision"));
            None
        }
        _ => {
            log::line(format!("hook id={id} timed out — terminal takes over"));
            None
        }
    }
}

fn send(app: &AppHandle, request_id: &str, reply: Reply, keep: bool) {
    let sender = {
        let pending = app.state::<Pending>();
        let mut map = pending.0.lock().unwrap();
        if keep {
            map.get(request_id).cloned()
        } else {
            map.remove(request_id)
        }
    };
    match sender {
        Some(tx) => {
            let _ = tx.try_send(reply);
        }
        None => log::line(format!("reply for id={request_id} — no pending request")),
    }
}

/// The island has the card on screen; the long wait may begin.
pub fn acknowledge(app: &AppHandle, request_id: &str) {
    send(app, request_id, Reply::Ack, true);
}

/// Nobody can act on this one — paused, or another card already holds the view.
pub fn decline(app: &AppHandle, request_id: &str) {
    log::line(format!("decline id={request_id}"));
    send(app, request_id, Reply::Decline, false);
}

/// Called by the island's Allow / Deny buttons. Only ever a bare word: turning
/// it into Claude Code's JSON is the relay's job.
pub fn answer(app: &AppHandle, request_id: &str, decision: &str) {
    let word = match decision {
        "allow" | "always" => "allow",
        _ => "deny",
    };
    log::line(format!("decision id={request_id} {word}"));
    send(app, request_id, Reply::Decision(word.to_string()), false);
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    use tokio::io::AsyncWriteExt;
    use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

    use super::*;

    #[test]
    fn app_pipe_name_uses_the_product_namespace_and_sid() {
        assert_eq!(
            pipe_name_for_sid("S-1-5-21-current"),
            r"\\.\pipe\anti-scrolling-notch-S-1-5-21-current"
        );
    }

    fn test_pipe_name(sid: &str) -> String {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        format!(
            r"\\.\pipe\{}-test-{}-{}-{sid}",
            crate::identity::APP_PIPE_PREFIX,
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
    }

    async fn open_client(name: String) -> io::Result<NamedPipeClient> {
        tokio::task::spawn_blocking(move || {
            let mut options = ClientOptions::new();
            options.read(true).write(true).open(name)
        })
        .await
        .map_err(|error| io::Error::other(error.to_string()))?
    }

    async fn exchange(sid: &str, bytes: &[u8]) -> Option<Value> {
        let name = test_pipe_name(sid);
        let server = create_server(&name, sid, true).unwrap();
        let receiver = tokio::spawn(async move {
            let mut server = server;
            server.connect().await.unwrap();
            let payload = receive_payload(&mut server).await;
            let _ = server.disconnect();
            payload
        });
        let mut client = open_client(name).await.unwrap();
        client.write_all(bytes).await.unwrap();
        client.flush().await.unwrap();
        drop(client);
        tokio::time::timeout(READ_TIMEOUT + Duration::from_secs(1), receiver)
            .await
            .expect("hook receiver returned before its deadline")
            .unwrap()
    }

    #[test]
    fn app_hook_pipe_uses_the_current_sid_and_rejects_instance_collisions() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let sid = crate::win_user::current_user_sid().expect("current-user SID");
            let name = test_pipe_name(&sid);
            let listeners = create_listener_pool(&name, &sid).unwrap();
            assert_eq!(listeners.len(), MAX_LISTENERS);
            assert!(create_server(&name, &sid, true).is_err());

            let server = listeners.into_iter().next().unwrap();
            let receiver = tokio::spawn(async move {
                let mut server = server;
                server.connect().await.unwrap();
                let payload = receive_payload(&mut server).await;
                let _ = server.disconnect();
                payload
            });
            let mut client = open_client(name).await.unwrap();
            let wire = br#"{"hook_event_name":"SessionStart","cwd":"C:\\project"}"#;
            client.write_all(wire).await.unwrap();
            client.write_all(b"\n").await.unwrap();
            client.flush().await.unwrap();
            drop(client);

            let payload = tokio::time::timeout(READ_TIMEOUT + Duration::from_secs(1), receiver)
                .await
                .expect("same-user hook message was not delivered before its deadline")
                .unwrap()
                .expect("same-user hook message was rejected");
            assert_eq!(payload["hook_event_name"], "SessionStart");
            assert_eq!(payload["cwd"], "C:\\project");
        });
    }

    #[test]
    fn hook_pipe_closes_silent_clients_at_the_read_deadline() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let sid = crate::win_user::current_user_sid().expect("current-user SID");
            let name = test_pipe_name(&sid);
            let server = create_server(&name, &sid, true).unwrap();
            let receiver = tokio::spawn(async move {
                let mut server = server;
                server.connect().await.unwrap();
                let started = Instant::now();
                let payload = receive_payload(&mut server).await;
                (payload, started.elapsed())
            });
            let _client = open_client(name).await.unwrap();
            let (payload, elapsed) =
                tokio::time::timeout(READ_TIMEOUT + Duration::from_secs(1), receiver)
                    .await
                    .expect("silent client exceeded the bounded receive deadline")
                    .unwrap();
            assert!(payload.is_none());
            assert!(elapsed >= READ_TIMEOUT);
        });
    }

    #[test]
    fn malformed_oversized_and_extra_framed_hook_messages_are_rejected() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let sid = crate::win_user::current_user_sid().expect("current-user SID");
            for bytes in [
                b"not-json\n".as_slice(),
                b"[]\n".as_slice(),
                b"{}\n{}\n".as_slice(),
            ] {
                assert!(exchange(&sid, bytes).await.is_none());
            }
            assert!(exchange(&sid, &vec![b'x'; MAX_PAYLOAD + 1]).await.is_none());
        });
    }

    #[test]
    fn active_hook_clients_and_pipe_instances_are_bounded() {
        let permits = Arc::new(Semaphore::new(MAX_ACTIVE_CLIENTS));
        let mut held = (0..MAX_ACTIVE_CLIENTS)
            .map(|_| acquire_client_permit(&permits).expect("permit within bound"))
            .collect::<Vec<_>>();
        assert!(acquire_client_permit(&permits).is_none());
        drop(held.pop());
        assert!(acquire_client_permit(&permits).is_some());
        assert_eq!(MAX_PIPE_INSTANCES, MAX_LISTENERS + MAX_ACTIVE_CLIENTS);
    }

    #[test]
    fn event_names_are_bounded_and_log_safe() {
        assert_eq!(safe_event_name("PermissionRequest"), "PermissionRequest");
        assert_eq!(
            safe_event_name("event\nwith controls"),
            "event?with?controls"
        );
        assert_eq!(safe_event_name(&"x".repeat(100)).len(), 64);
    }
}
