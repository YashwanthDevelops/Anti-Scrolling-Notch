//! Backend-only receive path for the verified Codex CLI observing adapter.
//!
//! This pipe is intentionally separate from `pipe.rs`: Codex lifecycle events
//! must not enter Claude's frontend hook handlers or its permission decision
//! flow. Stage 5 will connect this validated event boundary to the authoritative
//! reducer; until then, only the event discriminator is recorded locally.

use std::time::Duration;

use codex_hook_contract::{parse_wire_message, MAX_WIRE_BYTES};
use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

use crate::log;

const READ_TIMEOUT: Duration = Duration::from_secs(2);

pub fn pipe_name() -> String {
    let key = crate::win_user::current_user_sid()
        .unwrap_or_else(|| std::env::var("USERNAME").unwrap_or_else(|_| "user".into()));
    format!(r"\\.\pipe\{}-{key}", crate::identity::CODEX_PIPE_PREFIX)
}

pub fn start() {
    tauri::async_runtime::spawn(async move {
        let name = pipe_name();
        let mut server = match ServerOptions::new().first_pipe_instance(true).create(&name) {
            Ok(server) => server,
            Err(err) => {
                log::line(format!("cannot open Codex observer pipe: {err}"));
                return;
            }
        };

        loop {
            if server.connect().await.is_err() {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
            let next = match ServerOptions::new().create(&name) {
                Ok(server) => server,
                Err(err) => {
                    log::line(format!("cannot reopen Codex observer pipe: {err}"));
                    return;
                }
            };
            let connected = std::mem::replace(&mut server, next);
            tauri::async_runtime::spawn(async move { receive(connected).await });
        }
    });
}

async fn receive(mut pipe: NamedPipeServer) {
    let read = tokio::time::timeout(READ_TIMEOUT, read_message(&mut pipe)).await;
    if let Ok(Some(bytes)) = read {
        if let Ok(observation) = parse_wire_message(&bytes) {
            log::line(format!("Codex CLI hook {}", observation.event.as_str()));
        }
    }
    let _ = pipe.disconnect();
}

async fn read_message(pipe: &mut NamedPipeServer) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(MAX_WIRE_BYTES);
    let mut chunk = [0u8; 128];
    loop {
        match pipe.read(&mut chunk).await {
            Ok(0) => return Some(bytes),
            Ok(n) => {
                bytes.extend_from_slice(&chunk[..n]);
                if bytes.len() > MAX_WIRE_BYTES {
                    return None;
                }
                if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                    bytes.truncate(end);
                    return Some(bytes);
                }
            }
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use codex_hook_contract::{CodexHookEvent, CodexHookObservation};

    use super::*;

    #[test]
    fn receiver_accepts_only_versioned_observations_from_the_contract() {
        let wire = serde_json::to_vec(&CodexHookObservation::new(CodexHookEvent::Stop)).unwrap();
        let received = parse_wire_message(&wire).unwrap();
        assert_eq!(received.event, CodexHookEvent::Stop);
    }
}
