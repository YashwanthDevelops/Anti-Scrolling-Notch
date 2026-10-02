//! Backend-only receive path for the verified Codex CLI observing adapter.
//!
//! This pipe is intentionally separate from `pipe.rs`: Codex lifecycle events
//! must not enter Claude's frontend hook handlers or its permission decision
//! flow. Only the event discriminator crosses this bounded, current-user-only
//! boundary. Stage 5's reducer will consume it in a later packet.

use std::ffi::c_void;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::sync::Arc;
use std::time::Duration;

use codex_hook_contract::{parse_wire_message, CodexHookObservation, MAX_WIRE_BYTES};
use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};

use crate::log;

const READ_TIMEOUT: Duration = Duration::from_secs(2);
const LISTENER_RETRY: Duration = Duration::from_millis(200);
const MAX_LISTENERS: usize = 8;
const MAX_ACTIVE_CLIENTS: usize = 8;
const MAX_PIPE_INSTANCES: usize = MAX_LISTENERS + MAX_ACTIVE_CLIENTS;

fn pipe_name_for_sid(sid: &str) -> String {
    format!(r"\\.\pipe\{}-{sid}", crate::identity::CODEX_PIPE_PREFIX)
}

pub fn start() {
    tauri::async_runtime::spawn(async move {
        let Some(sid) = crate::win_user::current_user_sid() else {
            log::line("Codex observer disabled: could not resolve current-user SID");
            return;
        };
        let name = pipe_name_for_sid(&sid);
        let listeners = match create_listener_pool(&name, &sid) {
            Ok(listeners) => listeners,
            Err(err) => {
                // In particular, first_pipe_instance rejects a pipe already
                // owned by another process. Never attach to an existing name.
                log::line(format!("cannot open private Codex observer pipe: {err}"));
                return;
            }
        };

        let permits = Arc::new(Semaphore::new(MAX_ACTIVE_CLIENTS));
        for server in listeners {
            let name = name.clone();
            let sid = sid.clone();
            let permits = Arc::clone(&permits);
            tauri::async_runtime::spawn(async move {
                accept_loop(server, name, sid, permits).await;
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
    // A protected DACL containing one allow ACE for the current user's SID.
    // The relay opens the pipe for both reading and writing; no other account
    // or inherited broad group is granted access.
    let sddl = format!("D:P(A;;GRGW;;;{sid})");
    let wide_sddl = sddl.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(wide_sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
        .map_err(win_error)?;
    }
    let _descriptor = LocalAllocation(descriptor.0);
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };

    let mut options = ServerOptions::new();
    options
        .first_pipe_instance(first)
        .max_instances(MAX_PIPE_INSTANCES)
        .reject_remote_clients(true);
    // The Tokio API passes these attributes directly to CreateNamedPipeW.
    // The descriptor remains alive until that synchronous call has returned.
    unsafe {
        options.create_with_security_attributes_raw(
            name,
            (&mut attributes as *mut SECURITY_ATTRIBUTES).cast::<c_void>(),
        )
    }
}

async fn accept_loop(
    mut server: NamedPipeServer,
    name: String,
    sid: String,
    permits: Arc<Semaphore>,
) {
    loop {
        if let Err(err) = server.connect().await {
            let _ = server.disconnect();
            log::line(format!("Codex observer listener reconnecting: {err}"));
            tokio::time::sleep(LISTENER_RETRY).await;
            continue;
        }

        let Some(permit) = acquire_client_permit(&permits) else {
            // Saturation is neutral: drop this best-effort observer event and
            // reuse the bounded listener rather than creating another task.
            let _ = server.disconnect();
            continue;
        };

        let next = match create_server(&name, &sid, false) {
            Ok(next) => next,
            Err(err) => {
                // Keep this listener instance and recover after a short delay;
                // the connected event is deliberately dropped on this path.
                log::line(format!("Codex observer listener recovering: {err}"));
                let _ = server.disconnect();
                drop(permit);
                tokio::time::sleep(LISTENER_RETRY).await;
                continue;
            }
        };

        let connected = std::mem::replace(&mut server, next);
        tauri::async_runtime::spawn(async move {
            if let Some(observation) = receive(connected).await {
                log::line(format!("Codex CLI hook {}", observation.event.as_str()));
            }
            drop(permit);
        });
    }
}

fn acquire_client_permit(permits: &Arc<Semaphore>) -> Option<OwnedSemaphorePermit> {
    Arc::clone(permits).try_acquire_owned().ok()
}

async fn receive(mut pipe: NamedPipeServer) -> Option<CodexHookObservation> {
    // Read only a tiny framed message before checking the client identity.
    // Impersonation is based on the last message read, and no payload is
    // parsed, logged or acted on until the SID has been verified.
    let bytes = tokio::time::timeout(READ_TIMEOUT, read_message(&mut pipe))
        .await
        .ok()??;
    if !crate::win_user::pipe_client_is_same_user(windows::Win32::Foundation::HANDLE(
        pipe.as_raw_handle(),
    )) {
        return None;
    }
    parse_wire_message(&bytes).ok()
}

async fn read_message(pipe: &mut NamedPipeServer) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(MAX_WIRE_BYTES + 1);
    let mut chunk = [0u8; 128];
    let mut line_end = None;
    loop {
        match pipe.read(&mut chunk).await {
            Ok(0) => return line_end.map(|end| bytes[..end].to_vec()),
            Ok(n) => {
                if line_end.is_some() {
                    return None;
                }
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                    if end > MAX_WIRE_BYTES || end + 1 != bytes.len() {
                        return None;
                    }
                    line_end = Some(end);
                } else if bytes.len() > MAX_WIRE_BYTES {
                    return None;
                }
            }
            Err(_) => return None,
        }
    }
}

fn win_error(error: windows::core::Error) -> io::Error {
    io::Error::other(error.to_string())
}

struct LocalAllocation(*mut c_void);

impl Drop for LocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::windows::io::AsRawHandle;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    use codex_hook_contract::{CodexHookEvent, CodexHookObservation};
    use tokio::io::AsyncWriteExt;
    use tokio::net::windows::named_pipe::ClientOptions;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW,
        ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
        SE_KERNEL_OBJECT,
    };
    use windows::Win32::Security::{DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR};

    use super::*;

    fn test_pipe_name(sid: &str) -> String {
        static NEXT_PIPE: AtomicU64 = AtomicU64::new(0);
        format!(
            r"\\.\pipe\{}-test-{}-{}-{sid}",
            crate::identity::CODEX_PIPE_PREFIX,
            std::process::id(),
            NEXT_PIPE.fetch_add(1, Ordering::Relaxed)
        )
    }

    fn descriptor_dacl_sddl(descriptor: PSECURITY_DESCRIPTOR) -> io::Result<String> {
        let _descriptor = LocalAllocation(descriptor.0);
        let mut text = PWSTR::null();
        unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descriptor,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut text,
                None,
            )
            .map_err(win_error)?;
        }
        let _text = LocalAllocation(text.0.cast());
        unsafe {
            text.to_string()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        }
    }

    fn actual_dacl_sddl(pipe: &NamedPipeServer) -> io::Result<String> {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        let status = unsafe {
            GetSecurityInfo(
                HANDLE(pipe.as_raw_handle()),
                SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                None,
                None,
                Some(&mut descriptor),
            )
        };
        if status.0 != 0 {
            return Err(io::Error::from_raw_os_error(status.0 as i32));
        }
        descriptor_dacl_sddl(descriptor)
    }

    fn expected_dacl_sddl(sid: &str) -> io::Result<String> {
        // The pipe's generic read/write rights map to this concrete mask.
        // Use the mask spelling because Windows may serialize GRGW as either
        // symbolic rights or a numeric mask depending on the descriptor path.
        let sddl = format!("D:P(A;;0x12019f;;;{sid})");
        let wide_sddl = sddl.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(wide_sddl.as_ptr()),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
            .map_err(win_error)?;
        }
        descriptor_dacl_sddl(descriptor)
    }

    async fn open_client(
        name: String,
    ) -> io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
        tokio::task::spawn_blocking(move || {
            let mut options = ClientOptions::new();
            options.read(true).write(true).open(name)
        })
        .await
        .map_err(|error| io::Error::other(error.to_string()))?
    }

    async fn exchange(sid: &str, bytes: &[u8]) -> Option<CodexHookObservation> {
        let name = test_pipe_name(sid);
        let server = create_server(&name, sid, true).unwrap();
        let receiver = tokio::spawn(async move {
            server.connect().await.unwrap();
            receive(server).await
        });
        let mut client = open_client(name).await.unwrap();
        let _ = client.write_all(bytes).await;
        drop(client);
        tokio::time::timeout(Duration::from_secs(3), receiver)
            .await
            .expect("pipe receiver returned before its deadline")
            .unwrap()
    }

    #[test]
    fn private_pipe_acl_collision_identity_and_wire_are_checked_on_windows() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let sid = crate::win_user::current_user_sid().expect("current-user SID");
            let name = test_pipe_name(&sid);
            let mut listeners = create_listener_pool(&name, &sid).unwrap();
            assert_eq!(listeners.len(), MAX_LISTENERS);
            // Windows may serialize a well-known SID using its SDDL alias
            // (for example, the built-in Administrator account as `LA`).
            // Canonicalize the expected DACL through the same OS serializer so
            // the test compares the actual trustee, not its textual spelling.
            let expected_dacl = expected_dacl_sddl(&sid).unwrap();
            for listener in &listeners {
                assert_eq!(actual_dacl_sddl(listener).unwrap(), expected_dacl);
            }
            assert!(create_server(&name, &sid, true).is_err());

            let server = listeners.remove(0);
            drop(listeners);
            let server_task = tokio::spawn(async move {
                server.connect().await.unwrap();
                receive(server).await
            });
            let mut client = open_client(name).await.unwrap();
            let wire =
                serde_json::to_vec(&CodexHookObservation::new(CodexHookEvent::Stop)).unwrap();
            client.write_all(&wire).await.unwrap();
            client.write_all(b"\n").await.unwrap();
            client.flush().await.unwrap();
            drop(client);

            let observation = tokio::time::timeout(Duration::from_secs(3), server_task)
                .await
                .expect("pipe receiver returned before its deadline")
                .unwrap();
            assert_eq!(observation.unwrap().event, CodexHookEvent::Stop);
        });
    }

    #[test]
    fn inactive_connected_client_is_closed_by_the_read_deadline() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let sid = crate::win_user::current_user_sid().expect("current-user SID");
            let name = test_pipe_name(&sid);
            let server = create_server(&name, &sid, true).unwrap();
            let server_task = tokio::spawn(async move {
                server.connect().await.unwrap();
                let started = Instant::now();
                let observation = receive(server).await;
                (observation, started.elapsed())
            });
            let _client = open_client(name).await.unwrap();
            let (observation, elapsed) =
                tokio::time::timeout(READ_TIMEOUT + Duration::from_secs(1), server_task)
                    .await
                    .expect("slow client exceeded the bounded receive deadline")
                    .unwrap();
            assert!(observation.is_none());
            assert!(elapsed >= READ_TIMEOUT);
        });
    }

    #[test]
    fn observer_client_concurrency_is_bounded() {
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
    fn malformed_oversized_unknown_version_unterminated_and_extra_frames_are_rejected() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let sid = crate::win_user::current_user_sid().expect("current-user SID");
            let cases: [&[u8]; 5] = [
                b"not-json\n",
                br#"{"wire_version":2,"event":"Stop"}"#,
                br#"{"wire_version":1,"event":"Stop","prompt":"hidden"}"#,
                b"{\"wire_version\":1,\"event\":\"Stop\"}\n{}\n",
                br#"{"wire_version":1,"event":"Stop"}"#,
            ];
            for payload in cases {
                assert!(exchange(&sid, payload).await.is_none());
            }

            let oversized = vec![b'x'; MAX_WIRE_BYTES + 1];
            assert!(exchange(&sid, &oversized).await.is_none());
        });
    }

    #[test]
    fn receiver_still_uses_the_two_field_contract_and_only_verified_event_names() {
        let wire = serde_json::to_vec(&CodexHookObservation::new(CodexHookEvent::Stop)).unwrap();
        let received = parse_wire_message(&wire).unwrap();
        assert_eq!(received.event, CodexHookEvent::Stop);
        assert_eq!(wire, br#"{"wire_version":1,"event":"Stop"}"#.to_vec());
        assert!(parse_wire_message(br#"{"wire_version":1,"event":"Stop","prompt":"x"}"#).is_err());
        assert!(parse_wire_message(&vec![b' '; MAX_WIRE_BYTES + 1]).is_err());
    }
}
