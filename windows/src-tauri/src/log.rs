// Small append-only log at %LOCALAPPDATA%\Anti-Scrolling-Notch\anti-scrolling-notch.log — the Windows
// equivalent of nbLog() in HookServer.swift. Nothing leaves the machine.

use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use regex::Regex;

use windows::Win32::System::SystemInformation::GetLocalTime;

use crate::settings;

const MAX_LOG_BYTES: u64 = 1_000_000;
const MAX_LOG_LINE_BYTES: usize = 4_096;
const MAX_LOG_INPUT_BYTES: usize = 64 * 1024;
static LOG_LOCK: Mutex<()> = Mutex::new(());
static REDACTION_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

pub fn line(message: impl AsRef<str>) {
    let t = unsafe { GetLocalTime() };
    let stamp = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
    );
    let dir = settings::local_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join(crate::identity::LOG_FILE_NAME);
    let _guard = LOG_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    append_line(&path, &stamp, message.as_ref());
}

fn append_line(path: &Path, stamp: &str, message: &str) {
    let clean = sanitize_line(message);
    let line = format!("{stamp} {clean}\n");
    if std::fs::metadata(path)
        .map(|metadata| metadata.len().saturating_add(line.len() as u64) > MAX_LOG_BYTES)
        .unwrap_or(false)
    {
        let rotated = path.with_extension("log.1");
        let _ = std::fs::remove_file(&rotated);
        if std::fs::rename(path, &rotated).is_err() {
            return;
        }
        if std::fs::metadata(&rotated)
            .map(|metadata| metadata.len() > MAX_LOG_BYTES)
            .unwrap_or(false)
        {
            let _ = std::fs::remove_file(&rotated);
        }
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(line.as_bytes());
    }
}

pub(crate) fn redact_sensitive_text(message: &str) -> String {
    let patterns = REDACTION_PATTERNS.get_or_init(|| {
        [
            r#"(?i)(authorization[\"']?\s*[:=]\s*[\"']?(?:bearer|basic)\s+)[^\s,;\"']+"#,
            r#"(?i)([\"']?(?:api[_-]?key|access[_-]?(?:token|key)|refresh[_-]?token|client[_-]?secret|private[_-]?key|aws_secret_access_key|aws_access_key_id|credential|token|secret|password)[\"']?\s*[:=]\s*[\"']?)[^\"'\s,;]+"#,
            r"(?i)\b(?:sk-ant-[A-Za-z0-9_-]+|sk-[A-Za-z0-9_-]{20,}|(?:sk|rk)_(?:live|test)_[A-Za-z0-9_-]+|whsec_[A-Za-z0-9_-]+|gh[pousr]_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]+|xox[baprs]-[A-Za-z0-9-]+|re_[A-Za-z0-9_-]+|ntn_[A-Za-z0-9_-]+|cal_[A-Za-z0-9_-]+|AKIA[A-Z0-9]{16})\b",
            r#"(?i)\b[A-Z]:\\(?:[^\\/:*?"<>|\r\n]+\\)*[^\\\s/:*?"<>|\r\n]*"#,
            r"\\\\[^\\\s]+\\(?:[^\\\r\n]+\\)*[^\\\s\\\r\n]*",
        ]
        .into_iter()
        .map(|pattern| Regex::new(pattern).expect("static log redaction regex"))
        .collect()
    });

    let mut redacted = message.to_owned();
    for (index, pattern) in patterns.iter().enumerate() {
        let replacement = if index <= 1 {
            "$1[REDACTED]"
        } else {
            "[REDACTED]"
        };
        redacted = pattern.replace_all(&redacted, replacement).into_owned();
    }
    redacted
}

fn sanitize_line(message: &str) -> String {
    let input = truncate_utf8(message, MAX_LOG_INPUT_BYTES);
    let redacted = redact_sensitive_text(input);
    let single_line = redacted
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    if single_line.len() <= MAX_LOG_LINE_BYTES {
        return single_line;
    }
    format!(
        "{}…",
        truncate_utf8(&single_line, MAX_LOG_LINE_BYTES.saturating_sub("…".len()))
    )
}

fn truncate_utf8(text: &str, maximum_bytes: usize) -> &str {
    if text.len() <= maximum_bytes {
        return text;
    }
    let mut end = maximum_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn test_path(label: &str) -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("log-tests")
            .join(format!(
                "{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&directory).unwrap();
        directory.join("app.log")
    }

    #[test]
    fn redacts_credentials_paths_and_line_breaks() {
        let message = r#"Authorization: Bearer abc.def {"api_key":"sk-secret-value"} and C:\Users\Alice\private\repo
next"#;
        let sanitized = sanitize_line(message);
        assert!(!sanitized.contains("abc.def"));
        assert!(!sanitized.contains("sk-secret-value"));
        assert!(!sanitized.contains("C:\\Users\\Alice"));
        assert!(!sanitized.contains('\n'));
        assert!(sanitized.contains("[REDACTED]"));
        let spaced_path =
            redact_sensitive_text(r"at C:\Users\Alice\Windows Notch\app\settings.json");
        assert!(!spaced_path.contains("Windows Notch"));
        assert!(!spaced_path.contains("settings.json"));
        let spaced_unc = redact_sensitive_text(r"\\server\share\Windows Notch\settings.json");
        assert!(!spaced_unc.contains("Windows Notch"));
        assert!(!spaced_unc.contains("settings.json"));
        let json_bearer = redact_sensitive_text(r#"{"authorization":"Bearer token-value"}"#);
        assert!(!json_bearer.contains("token-value"));
        for secret in [
            "sk_live_secret",
            "whsec_secret",
            "ghp_012345678901234567890123",
        ] {
            assert!(!redact_sensitive_text(secret).contains(secret));
        }
    }

    #[test]
    fn rotates_one_bounded_backup_and_truncates_large_lines() {
        let path = test_path("rotate");
        std::fs::write(&path, "x".repeat(MAX_LOG_BYTES as usize - 10)).unwrap();
        append_line(&path, "2026-10-02", "rotate now");
        let current = std::fs::metadata(&path).unwrap().len();
        assert!(current <= MAX_LOG_BYTES);
        let backup = path.with_extension("log.1");
        assert!(std::fs::metadata(&backup).unwrap().len() <= MAX_LOG_BYTES);
        append_line(&path, "2026-10-02", &"y".repeat(MAX_LOG_LINE_BYTES * 2));
        assert!(std::fs::metadata(&path).unwrap().len() <= MAX_LOG_BYTES);
        assert!(std::fs::read_to_string(&path).unwrap().contains('…'));
        assert!(std::fs::metadata(&backup).unwrap().len() <= MAX_LOG_BYTES);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
