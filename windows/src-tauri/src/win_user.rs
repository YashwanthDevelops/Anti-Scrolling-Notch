// Who we are, for the relay pipe name.
//
// Named pipes share one machine-wide namespace. The SID in the name and the
// explicit pipe DACL keep other accounts away; the relay verifies the server
// SID and the app verifies the SID behind each received message.

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{
    GetTokenInformation, RevertToSelf, TokenUser, TOKEN_QUERY, TOKEN_USER,
};
use windows::Win32::System::Pipes::ImpersonateNamedPipeClient as ImpersonatePipeClient;
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, OpenProcessToken, OpenThreadToken,
};

/// The SID of the account this process runs as, as `S-1-5-21-…`.
pub fn current_user_sid() -> Option<String> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).ok()?;
        let sid = token_sid(token);
        let _ = CloseHandle(token);
        sid
    }
}

/// Verify the SID that sent the last pipe message. The check runs
/// synchronously after the bounded read, with no `.await` while the Tokio
/// worker thread is impersonating the peer.
pub fn pipe_client_is_same_user(pipe: HANDLE) -> bool {
    let Some(expected) = current_user_sid() else {
        return false;
    };
    unsafe {
        if ImpersonatePipeClient(pipe).is_err() {
            return false;
        }
        let _revert = RevertToSelfGuard;

        let mut token = HANDLE::default();
        if OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token).is_err() {
            return false;
        }
        let actual = token_sid(token);
        let _ = CloseHandle(token);
        same_user_sid(&expected, actual.as_deref())
    }
}

fn same_user_sid(expected: &str, actual: Option<&str>) -> bool {
    actual == Some(expected)
}

struct RevertToSelfGuard;

impl Drop for RevertToSelfGuard {
    fn drop(&mut self) {
        if unsafe { RevertToSelf() }.is_err() {
            // Reusing a Tokio worker with a leaked impersonation token would
            // run unrelated work as the pipe client. Windows directs servers
            // to terminate if they cannot revert successfully.
            std::process::abort();
        }
    }
}

/// Return the account SID from a borrowed token handle.
unsafe fn token_sid(token: HANDLE) -> Option<String> {
    // First call sizes the buffer, second fills it.
    let mut needed = 0u32;
    let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
    if needed == 0 {
        return None;
    }
    let mut buf = vec![0u8; needed as usize];
    GetTokenInformation(
        token,
        TokenUser,
        Some(buf.as_mut_ptr().cast()),
        needed,
        &mut needed,
    )
    .ok()?;

    let user = &*(buf.as_ptr() as *const TOKEN_USER);
    let mut text = PWSTR::null();
    ConvertSidToStringSidW(user.User.Sid, &mut text).ok()?;
    let sid = text.to_string().ok();
    let _ = LocalFree(Some(HLOCAL(text.0 as *mut _)));
    sid
}

#[cfg(test)]
mod tests {
    use super::same_user_sid;

    #[test]
    fn missing_or_mismatched_identity_fails_closed() {
        assert!(same_user_sid("S-1-5-21-current", Some("S-1-5-21-current")));
        assert!(!same_user_sid("S-1-5-21-current", Some("S-1-5-21-other")));
        assert!(!same_user_sid("S-1-5-21-current", None));
    }
}
