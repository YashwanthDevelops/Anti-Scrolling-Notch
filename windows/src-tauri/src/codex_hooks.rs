//! Previewable, user-consented Codex CLI observing-hook configuration.
//!
//! This installer writes only the user-level `hooks.json` file. It registers
//! the four event names captured for CLI 0.157.1 and invokes the existing
//! observer relay. Codex's own trust/review flow remains authoritative.

use std::ffi::OsStr;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use codex_hook_contract::{CodexHookEvent, CODEX_HOOK_SCHEMA_CLI_VERSION};
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::{hooks, settings};

const OBSERVER_ARGUMENT: &str = "--codex-observer";
const HOOK_TIMEOUT_SECONDS: u64 = 2;
const CLI_VERSION_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_VERSION_OUTPUT_BYTES: usize = 128;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Events installed by this packet come directly from the accepted runtime
/// capture. Schema-only events are deliberately not included.
const OBSERVED_EVENTS: [CodexHookEvent; 4] = CodexHookEvent::VERIFIED;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexHookStatus {
    pub configured_events: Vec<String>,
    pub settings_path: String,
    pub hook_path: String,
    pub hook_ready: bool,
    pub cli_version: Option<String>,
    pub cli_supported: bool,
    pub supported_cli_version: String,
}

fn codex_home_from(
    code_home: Option<&OsStr>,
    user_profile: Option<&OsStr>,
) -> Result<PathBuf, String> {
    let home = if let Some(code_home) = code_home {
        let path = PathBuf::from(code_home);
        if !path.is_absolute() {
            return Err("CODEX_HOME must be an absolute path.".into());
        }
        path
    } else {
        let user_profile = user_profile
            .ok_or_else(|| "USERPROFILE is unavailable and CODEX_HOME is not set.".to_string())?;
        PathBuf::from(user_profile).join(".codex")
    };
    Ok(home)
}

fn codex_home() -> Result<PathBuf, String> {
    codex_home_from(
        std::env::var_os("CODEX_HOME").as_deref(),
        std::env::var_os("USERPROFILE").as_deref(),
    )
}

fn hooks_path_for(home: &Path) -> PathBuf {
    home.join("hooks.json")
}

fn config_toml_path(home: &Path) -> PathBuf {
    home.join("config.toml")
}

fn has_inline_hooks(config_path: &Path) -> Result<bool, String> {
    let text = match std::fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!("Can't inspect {}: {error}", config_path.display()));
        }
    };
    has_inline_hook_declaration(&text).map_err(|error| {
        format!(
            "Can't parse Codex config {}: {error}",
            config_path.display()
        )
    })
}

/// Detects Codex's inline hook tables without editing its TOML configuration.
/// The complete file is parsed first so malformed config fails closed.
fn has_inline_hook_declaration(text: &str) -> Result<bool, String> {
    let root: toml::Value = toml::from_str(text).map_err(|error| error.to_string())?;
    Ok(match root.get("hooks") {
        None => false,
        Some(toml::Value::Table(hooks)) => hooks.keys().any(|key| key != "state"),
        Some(_) => true,
    })
}

fn command_for(relay_exe: &Path, windows: bool) -> String {
    let mut executable = relay_exe.to_string_lossy().into_owned();
    if !windows {
        executable = executable.replace('\\', "/");
    }
    format!("\"{executable}\" {OBSERVER_ARGUMENT}")
}

fn codex_entry_for(relay_exe: &Path, windows: bool) -> Value {
    json!({
        "type": "command",
        "command": command_for(relay_exe, false),
        "commandWindows": command_for(relay_exe, windows),
        "async": true,
        "timeout": HOOK_TIMEOUT_SECONDS,
    })
}

fn exact_observer_command(command: &str, relay_exe: &Path) -> bool {
    let command = command.trim_start();
    let (executable, arguments) = if let Some(quoted) = command.strip_prefix('"') {
        let Some((executable, remaining)) = quoted.split_once('"') else {
            return false;
        };
        (executable, remaining.trim())
    } else {
        let mut parts = command.split_whitespace();
        let Some(executable) = parts.next() else {
            return false;
        };
        let arguments = parts.collect::<Vec<_>>().join(" ");
        // Compare below without allocating a special representation for the
        // executable path; the unquoted path cannot contain spaces.
        if arguments != OBSERVER_ARGUMENT {
            return false;
        }
        (executable, OBSERVER_ARGUMENT)
    };

    if arguments != OBSERVER_ARGUMENT {
        return false;
    }
    executable
        .replace('\\', "/")
        .eq_ignore_ascii_case(&relay_exe.to_string_lossy().replace('\\', "/"))
}

fn entry_is_ours(entry: &Value, relay_exe: &Path) -> bool {
    let Some(handlers) = entry.get("hooks").and_then(Value::as_array) else {
        return false;
    };
    handlers.iter().any(|handler| {
        if handler.get("type").and_then(Value::as_str) != Some("command") {
            return false;
        }
        let command = handler
            .get("commandWindows")
            .and_then(Value::as_str)
            .or_else(|| handler.get("command").and_then(Value::as_str));
        command.is_some_and(|command| exact_observer_command(command, relay_exe))
    })
}

fn remove_owned_handlers(value: &Value, relay_exe: &Path) -> (Value, bool) {
    let Some(object) = value.as_object() else {
        return (value.clone(), false);
    };
    let Some(handlers) = object.get("hooks").and_then(Value::as_array) else {
        return (value.clone(), false);
    };

    let kept_handlers: Vec<Value> = handlers
        .iter()
        .filter(|handler| !entry_is_ours(&json!({ "hooks": [handler] }), relay_exe))
        .cloned()
        .collect();
    let removed = kept_handlers.len() != handlers.len();
    if !removed {
        return (value.clone(), false);
    }

    let mut next = object.clone();
    if kept_handlers.is_empty() && next.len() == 1 {
        return (Value::Null, true);
    }
    next.insert("hooks".into(), Value::Array(kept_handlers));
    (Value::Object(next), true)
}

fn strip_ours_from_event(groups: &mut Vec<Value>, relay_exe: &Path) -> bool {
    let mut removed_any = false;
    let mut kept_groups = Vec::with_capacity(groups.len());
    for group in groups.drain(..) {
        let (next, removed) = remove_owned_handlers(&group, relay_exe);
        removed_any |= removed;
        if !next.is_null() {
            kept_groups.push(next);
        }
    }
    *groups = kept_groups;
    removed_any
}

fn hooks_object(root: &Value) -> Result<Map<String, Value>, String> {
    match root.get("hooks") {
        None => Ok(Map::new()),
        Some(Value::Object(hooks)) => Ok(hooks.clone()),
        Some(_) => Err("The hooks.json `hooks` value is not an object; it was not changed.".into()),
    }
}

fn merged(existing: &Value, relay_exe: &Path) -> Result<Value, String> {
    let mut root = existing
        .as_object()
        .cloned()
        .ok_or_else(|| "Codex hooks.json must contain a JSON object.".to_string())?;
    let mut hook_events = hooks_object(existing)?;

    for event in OBSERVED_EVENTS {
        let name = event.as_str();
        let mut groups = match hook_events.get(name) {
            None => Vec::new(),
            Some(Value::Array(groups)) => groups.clone(),
            Some(_) => {
                return Err(format!(
                    "Codex hooks.json event `{name}` is not an array; it was not changed."
                ));
            }
        };
        strip_ours_from_event(&mut groups, relay_exe);
        groups.push(json!({ "hooks": [codex_entry_for(relay_exe, true)] }));
        hook_events.insert(name.to_string(), Value::Array(groups));
    }

    root.insert("hooks".into(), Value::Object(hook_events));
    Ok(Value::Object(root))
}

fn without_ours(existing: &Value, relay_exe: &Path) -> Result<Value, String> {
    let mut root = existing
        .as_object()
        .cloned()
        .ok_or_else(|| "Codex hooks.json must contain a JSON object.".to_string())?;
    let mut hook_events = hooks_object(existing)?;
    for event in OBSERVED_EVENTS {
        let name = event.as_str();
        if let Some(Value::Array(groups)) = hook_events.get_mut(name) {
            strip_ours_from_event(groups, relay_exe);
            if groups.is_empty() {
                hook_events.remove(name);
            }
        }
    }
    if hook_events.is_empty() {
        root.remove("hooks");
    } else if root.contains_key("hooks") {
        root.insert("hooks".into(), Value::Object(hook_events));
    }
    Ok(Value::Object(root))
}

fn read_current(path: &Path) -> Result<(Value, Vec<u8>), String> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let value = hooks::parse_settings(&bytes, &path.display().to_string())?;
            Ok((value, bytes))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((json!({}), Vec::new())),
        Err(error) => Err(format!("Can't read {}: {error}", path.display())),
    }
}

fn backup_path(path: &Path, original_bytes: &[u8]) -> Result<PathBuf, String> {
    let stem = format!("hooks.json.bak-{}", hooks::fingerprint(original_bytes));
    let mut suffix = 0_u64;
    loop {
        let name = if suffix == 0 {
            stem.clone()
        } else {
            format!("{stem}-{suffix}")
        };
        let candidate = path.with_file_name(name);
        match std::fs::symlink_metadata(&candidate) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(candidate);
            }
            Ok(_) => {
                suffix = suffix.checked_add(1).ok_or_else(|| {
                    "Could not find an available hooks.json backup name.".to_string()
                })?;
            }
            Err(error) => {
                return Err(format!(
                    "Can't inspect backup destination {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
}

fn ensure_regular_file_or_missing(path: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "{} is a symbolic link. It was not changed.",
            path.display()
        )),
        Ok(metadata) if !metadata.is_file() => Err(format!(
            "{} is not a regular file. It was not changed.",
            path.display()
        )),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Can't inspect {}: {error}", path.display())),
    }
}

fn preview_at(
    path: &Path,
    relay_exe: &Path,
    install: bool,
) -> Result<crate::hooks::HookPreview, String> {
    let file_exists = ensure_regular_file_or_missing(path)?;
    let (current, bytes) = read_current(path)?;
    let next = if install {
        merged(&current, relay_exe)?
    } else {
        without_ours(&current, relay_exe)?
    };
    let backup = if file_exists && next != current {
        backup_path(path, &bytes)?.to_string_lossy().to_string()
    } else {
        String::new()
    };
    Ok(crate::hooks::HookPreview {
        diff: hooks::unified_diff(
            &serde_json::to_string_pretty(&current).unwrap_or_default(),
            &serde_json::to_string_pretty(&next).unwrap_or_default(),
        ),
        backup,
        settings_path: path.to_string_lossy().to_string(),
        fingerprint: hooks::fingerprint(&bytes),
    })
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_file_name(format!(
        ".hooks.json.anti-scrolling-notch-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("temporary file create failed: {error}"))?;
    let write_result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("temporary file write/flush failed: {error}"));
    drop(file);
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("atomic replace failed: {error}"));
    }
    Ok(())
}

fn create_backup(path: &Path, backup: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(backup)
        .map_err(|error| format!("Backup failed for {}: {error}", path.display()))?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = std::fs::remove_file(backup);
        return Err(format!("Backup failed for {}: {error}", path.display()));
    }
    Ok(())
}

fn apply_at(
    path: &Path,
    config_path: &Path,
    relay_exe: &Path,
    install: bool,
    expected_fingerprint: &str,
    expected_backup: &str,
) -> Result<String, String> {
    if install && has_inline_hooks(config_path)? {
        return Err(format!(
            "{} already defines inline [hooks]. Codex combines both sources and warns, so no hooks were written. Remove or migrate that table first.",
            config_path.display()
        ));
    }
    let file_exists = ensure_regular_file_or_missing(path)?;
    let (current, original_bytes) = read_current(path)?;
    if hooks::fingerprint(&original_bytes) != expected_fingerprint {
        return Err(format!(
            "{} changed since the preview. Nothing was written; review a fresh diff.",
            path.display()
        ));
    }

    let next = if install {
        merged(&current, relay_exe)?
    } else {
        without_ours(&current, relay_exe)?
    };
    if next == current {
        if !expected_backup.is_empty() {
            return Err(format!(
                "{} no longer needs a backup. Review a fresh preview.",
                path.display()
            ));
        }
        return Ok("No change; Codex hook settings were left untouched.".into());
    }

    let backup = if file_exists {
        let candidate = backup_path(path, &original_bytes)?;
        if candidate.to_string_lossy() != expected_backup {
            return Err(format!(
                "The backup destination for {} changed since the preview. Review a fresh diff.",
                path.display()
            ));
        }
        Some(candidate)
    } else {
        if !expected_backup.is_empty() {
            return Err(format!(
                "{} did not exist in the preview, so no backup destination is valid. Review a fresh diff.",
                path.display()
            ));
        }
        None
    };
    let mut output = serde_json::to_vec_pretty(&next).map_err(|error| error.to_string())?;
    output.push(b'\n');

    let parent = path
        .parent()
        .ok_or_else(|| "Codex hooks path has no parent directory.".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Can't create {}: {error}", parent.display()))?;

    if let Some(backup) = backup.as_deref() {
        create_backup(path, backup, &original_bytes)?;
    }
    atomic_replace(path, &output)?;
    Ok(backup.map_or_else(
        || "Created hooks.json. No previous file existed, so no backup was created.".into(),
        |backup| backup.to_string_lossy().to_string(),
    ))
}

fn parse_cli_version(output: &[u8]) -> Option<String> {
    if output.len() > MAX_VERSION_OUTPUT_BYTES {
        return None;
    }
    let version = String::from_utf8(output.to_vec()).ok()?.trim().to_string();
    let expected = format!("codex-cli {CODEX_HOOK_SCHEMA_CLI_VERSION}");
    (version == expected).then_some(version)
}

fn detect_cli_version() -> Option<String> {
    let mut child = Command::new("codex")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let output = child.wait_with_output().ok()?.stdout;
                return parse_cli_version(&output);
            }
            Ok(None) if started.elapsed() < CLI_VERSION_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn require_supported_cli() -> Result<(), String> {
    match detect_cli_version() {
        Some(_) => Ok(()),
        None => Err(format!(
            "Codex CLI {} could not be verified from `codex --version`; hook installation is disabled.",
            CODEX_HOOK_SCHEMA_CLI_VERSION
        )),
    }
}

fn configured_events(value: &Value, relay_exe: &Path) -> Vec<String> {
    let Some(events) = value.get("hooks").and_then(Value::as_object) else {
        return Vec::new();
    };
    OBSERVED_EVENTS
        .iter()
        .map(|event| event.as_str())
        .filter(|name| {
            events
                .get(*name)
                .and_then(Value::as_array)
                .is_some_and(|groups| groups.iter().any(|group| entry_is_ours(group, relay_exe)))
        })
        .map(str::to_string)
        .collect()
}

pub fn status() -> Result<CodexHookStatus, String> {
    let home = codex_home()?;
    let path = hooks_path_for(&home);
    let (current, _) = read_current(&path)?;
    let hook_path = settings::hook_exe_path();
    let cli_version = detect_cli_version();
    Ok(CodexHookStatus {
        configured_events: configured_events(&current, &hook_path),
        settings_path: path.to_string_lossy().to_string(),
        hook_path: hook_path.to_string_lossy().to_string(),
        hook_ready: hook_path.is_file(),
        cli_supported: cli_version.is_some(),
        cli_version,
        supported_cli_version: CODEX_HOOK_SCHEMA_CLI_VERSION.into(),
    })
}

pub fn preview(install: bool) -> Result<crate::hooks::HookPreview, String> {
    if install {
        require_supported_cli()?;
    }
    let home = codex_home()?;
    let path = hooks_path_for(&home);
    if install && !settings::hook_exe_path().is_file() {
        return Err("The Anti-Scrolling-Notch observer relay is not installed.".into());
    }
    if install && has_inline_hooks(&config_toml_path(&home))? {
        return Err(format!(
            "{} already defines inline [hooks]. Codex combines both sources and warns, so no hooks were written.",
            config_toml_path(&home).display()
        ));
    }
    preview_at(&path, &settings::hook_exe_path(), install)
}

pub fn write(install: bool, fingerprint: &str, backup: &str) -> Result<String, String> {
    if install {
        require_supported_cli()?;
        if !settings::hook_exe_path().is_file() {
            return Err("The Anti-Scrolling-Notch observer relay is not installed.".into());
        }
    }
    let home = codex_home()?;
    let path = hooks_path_for(&home);
    apply_at(
        &path,
        &config_toml_path(&home),
        &settings::hook_exe_path(),
        install,
        fingerprint,
        backup,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "anti-scrolling-notch-codex-hooks-{}-{label}",
            std::process::id()
        ))
    }

    fn relay() -> PathBuf {
        PathBuf::from(
            r"C:\Users\Tester\AppData\Local\Anti-Scrolling-Notch\bin\anti-scrolling-notch-hook.exe",
        )
    }

    fn create_temp_home(home: &Path) {
        let _ = std::fs::remove_dir_all(home);
        std::fs::create_dir_all(home).unwrap();
    }

    #[test]
    fn only_the_runtime_captured_events_are_registered() {
        let relay = relay();
        let merged = merged(&json!({"description":"user hooks"}), &relay).unwrap();
        let names = configured_events(&merged, &relay);
        assert_eq!(
            names,
            vec![
                "SessionStart".to_string(),
                "UserPromptSubmit".to_string(),
                "Stop".to_string(),
                "SessionEnd".to_string()
            ]
        );
        let events = merged["hooks"].as_object().unwrap();
        assert_eq!(events.len(), 4);
        for event in OBSERVED_EVENTS {
            let handler = &events[event.as_str()][0]["hooks"][0];
            assert_eq!(handler["type"], "command");
            assert_eq!(handler["async"], true);
            assert_eq!(handler["timeout"], HOOK_TIMEOUT_SECONDS);
            assert!(handler["commandWindows"]
                .as_str()
                .unwrap()
                .contains(OBSERVER_ARGUMENT));
        }
        assert_eq!(
            OBSERVED_EVENTS.map(CodexHookEvent::as_str),
            ["SessionStart", "UserPromptSubmit", "Stop", "SessionEnd"]
        );
    }

    #[test]
    fn exact_command_ownership_preserves_other_handlers_and_metadata() {
        let relay = relay();
        let owned = codex_entry_for(&relay, true);
        let similar = json!({
            "type":"command",
            "commandWindows": format!("{} --codex-observer-extra", command_for(&relay, true)),
        });
        let group = json!({
            "matcher":"custom",
            "label":"keep",
            "hooks":[owned, {"type":"command", "command":"other.exe"}, similar]
        });
        let (without, removed) = remove_owned_handlers(&group, &relay);
        assert!(removed);
        assert_eq!(without["matcher"], "custom");
        assert_eq!(without["label"], "keep");
        assert_eq!(without["hooks"].as_array().unwrap().len(), 2);
        assert!(!entry_is_ours(&similar, &relay));
    }

    #[test]
    fn repeated_install_does_not_duplicate_and_uninstall_keeps_foreign_data() {
        let relay = relay();
        let original = json!({
            "description":"keep me",
            "hooks": {
                "SessionStart":[{"matcher":"startup", "hooks":[{"type":"command", "command":"foreign.exe"}]}],
                "PrivateEvent":[{"hooks":[{"type":"command", "command":"someone-else.exe"}]}]
            }
        });
        let first = merged(&original, &relay).unwrap();
        let second = merged(&first, &relay).unwrap();
        let start = second["hooks"]["SessionStart"].as_array().unwrap();
        assert_eq!(start.len(), 2);
        assert_eq!(configured_events(&second, &relay).len(), 4);
        let removed = without_ours(&second, &relay).unwrap();
        assert_eq!(removed, original);
    }

    #[test]
    fn preview_apply_backup_and_remove_use_only_the_injected_codex_home() {
        let home = temp_home("round-trip");
        create_temp_home(&home);
        let path = hooks_path_for(&home);
        let config = config_toml_path(&home);
        let original = br#"{ "description": "preserve", "hooks": { "OtherEvent": [{"hooks":[{"type":"command","command":"other.exe"}]}] } }"#;
        std::fs::write(&path, original).unwrap();
        let plan = preview_at(&path, &relay(), true).unwrap();
        assert!(plan.diff.contains("--codex-observer"));
        let backup = apply_at(
            &path,
            &config,
            &relay(),
            true,
            &plan.fingerprint,
            &plan.backup,
        )
        .unwrap();
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        let after: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(after["description"], "preserve");
        assert!(after["hooks"]["OtherEvent"].is_array());
        assert_eq!(configured_events(&after, &relay()).len(), 4);

        let removal = preview_at(&path, &relay(), false).unwrap();
        apply_at(
            &path,
            &config,
            &relay(),
            false,
            &removal.fingerprint,
            &removal.backup,
        )
        .unwrap();
        let restored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            restored["hooks"]["OtherEvent"],
            after["hooks"]["OtherEvent"]
        );
        assert!(configured_events(&restored, &relay()).is_empty());
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn first_install_shows_no_fake_backup_and_creates_hooks_json() {
        let home = temp_home("first-install");
        create_temp_home(&home);
        let path = hooks_path_for(&home);
        let config = config_toml_path(&home);
        let plan = preview_at(&path, &relay(), true).unwrap();
        assert!(plan.backup.is_empty());
        assert!(plan.diff.contains("--codex-observer"));

        let result = apply_at(
            &path,
            &config,
            &relay(),
            true,
            &plan.fingerprint,
            &plan.backup,
        )
        .unwrap();
        assert!(result.contains("No previous file existed"));
        let installed: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(configured_events(&installed, &relay()).len(), 4);
        assert!(!std::fs::read_dir(&home)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("hooks.json.bak-")));

        let no_change = preview_at(&path, &relay(), true).unwrap();
        assert!(no_change.backup.is_empty());
        assert!(apply_at(
            &path,
            &config,
            &relay(),
            true,
            &no_change.fingerprint,
            &no_change.backup,
        )
        .unwrap()
        .starts_with("No change;"));
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn backup_collision_is_previewed_and_never_overwritten() {
        let home = temp_home("backup-collision");
        create_temp_home(&home);
        let path = hooks_path_for(&home);
        let config = config_toml_path(&home);
        let original = br#"{"description":"preserve"}"#;
        std::fs::write(&path, original).unwrap();
        let occupied =
            path.with_file_name(format!("hooks.json.bak-{}", hooks::fingerprint(original)));
        std::fs::write(&occupied, b"preexisting backup").unwrap();

        let plan = preview_at(&path, &relay(), true).unwrap();
        assert_eq!(
            plan.backup,
            occupied
                .with_file_name(format!(
                    "{}-1",
                    occupied.file_name().unwrap().to_string_lossy()
                ))
                .to_string_lossy()
        );
        let created = apply_at(
            &path,
            &config,
            &relay(),
            true,
            &plan.fingerprint,
            &plan.backup,
        )
        .unwrap();
        assert_eq!(created, plan.backup);
        assert_eq!(std::fs::read(&occupied).unwrap(), b"preexisting backup");
        assert_eq!(std::fs::read(&plan.backup).unwrap(), original);
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn backup_destination_race_requires_a_fresh_preview() {
        let home = temp_home("backup-race");
        create_temp_home(&home);
        let path = hooks_path_for(&home);
        let config = config_toml_path(&home);
        let original = br#"{"theme":"dark"}"#;
        std::fs::write(&path, original).unwrap();
        let plan = preview_at(&path, &relay(), true).unwrap();
        std::fs::write(&plan.backup, b"created after preview").unwrap();

        let error = apply_at(
            &path,
            &config,
            &relay(),
            true,
            &plan.fingerprint,
            &plan.backup,
        )
        .unwrap_err();
        assert!(error.contains("backup destination"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(
            std::fs::read(&plan.backup).unwrap(),
            b"created after preview"
        );
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn stale_preview_and_invalid_json_leave_the_file_unchanged() {
        let home = temp_home("stale");
        create_temp_home(&home);
        let path = hooks_path_for(&home);
        let config = config_toml_path(&home);
        std::fs::write(&path, br#"{"theme":"dark"}"#).unwrap();
        let plan = preview_at(&path, &relay(), true).unwrap();
        std::fs::write(&path, br#"{"theme":"light"}"#).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(apply_at(
            &path,
            &config,
            &relay(),
            true,
            &plan.fingerprint,
            &plan.backup,
        )
        .unwrap_err()
        .contains("changed since the preview"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::write(&path, b"{ broken").unwrap();
        assert!(preview_at(&path, &relay(), true).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{ broken");
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn inline_hooks_and_conflicting_event_shapes_fail_closed() {
        let home = temp_home("conflict");
        create_temp_home(&home);
        let config = config_toml_path(&home);
        assert!(!has_inline_hook_declaration("[features]\nhooks = false\n").unwrap());
        assert!(!has_inline_hook_declaration(
            "# [[hooks.SessionStart]]\n[hooks.state]\nentry = { enabled = false }\n"
        )
        .unwrap());
        assert!(
            has_inline_hook_declaration("[[hooks.SessionStart]]\nmatcher = \"startup\"\n").unwrap()
        );
        assert!(has_inline_hook_declaration("[hooks]\nSessionStart = [{ hooks = [] }]\n").unwrap());
        assert!(has_inline_hook_declaration("[hooks\n").is_err());
        std::fs::write(&config, "[hooks\n").unwrap();
        assert!(has_inline_hooks(&config)
            .unwrap_err()
            .contains("Can't parse Codex config"));
        let path = hooks_path_for(&home);
        let before = br#"{"description":"preserve"}"#;
        std::fs::write(&path, before).unwrap();
        let plan = preview_at(&path, &relay(), true).unwrap();
        std::fs::write(&config, "[[hooks.SessionStart]]\nmatcher = \"startup\"\n").unwrap();
        assert!(has_inline_hooks(&config).unwrap());
        assert!(apply_at(
            &path,
            &config,
            &relay(),
            true,
            &plan.fingerprint,
            &plan.backup,
        )
        .unwrap_err()
        .contains("already defines inline [hooks]"));
        assert_eq!(std::fs::read(&path).unwrap(), before);

        std::fs::write(&config, "").unwrap();
        let conflict = br#"{"hooks":{"SessionStart":false}}"#;
        std::fs::write(&path, conflict).unwrap();
        assert!(preview_at(&path, &relay(), true).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), conflict);
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn codex_home_selection_and_cli_version_gate_are_exact() {
        let user_profile = OsStr::new(r"C:\Users\Tester");
        assert_eq!(
            codex_home_from(None, Some(user_profile)).unwrap(),
            PathBuf::from(r"C:\Users\Tester\.codex")
        );
        assert_eq!(
            codex_home_from(Some(OsStr::new(r"D:\CodexProfile")), Some(user_profile)).unwrap(),
            PathBuf::from(r"D:\CodexProfile")
        );
        assert!(codex_home_from(Some(OsStr::new("relative")), Some(user_profile)).is_err());
        assert_eq!(
            parse_cli_version(b"codex-cli 0.157.1\r\n").as_deref(),
            Some("codex-cli 0.157.1")
        );
        assert!(parse_cli_version(b"codex-cli 0.158.0").is_none());
        assert!(parse_cli_version(&[b'x'; MAX_VERSION_OUTPUT_BYTES + 1]).is_none());
    }
}
