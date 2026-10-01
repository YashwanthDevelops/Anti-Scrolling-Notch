//! Best-effort navigation to the registered stable OpenAI Codex desktop app.
//!
//! Codex's observed hook events do not identify a session or provide a verified
//! destination. This module therefore opens only the app's generic launch
//! surface. It does not invent a chat URI, pass arguments, or inspect Codex data.

use std::thread;

use windows::core::PCWSTR;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    ApplicationActivationManager, IApplicationActivationManager, AO_NOERRORUI,
};

/// Stable package family and application id observed in the installed stable
/// Codex desktop app. App identity is versioned and verified in
/// `docs/codex-compatibility.md`; this is not a protocol/deep link.
pub const CODEX_APP_USER_MODEL_ID: &str = "OpenAI.Codex_2p2nqsd0c76g0!App";

/// Activate the generic OpenAI Codex desktop launch surface for this user.
pub fn activate_codex() -> Result<u32, String> {
    activate_aumid(CODEX_APP_USER_MODEL_ID)
}

fn activate_aumid(aumid: &str) -> Result<u32, String> {
    let aumid = aumid.to_owned();
    thread::Builder::new()
        .name("codex-app-activation".into())
        .spawn(move || activate_on_sta(&aumid))
        .map_err(|err| format!("Could not start Codex app activation: {err}"))?
        .join()
        .map_err(|_| "Codex app activation worker stopped unexpectedly".to_string())?
}

fn activate_on_sta(aumid: &str) -> Result<u32, String> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .map_err(|err| {
            format_hresult("Could not initialize Windows app activation", err.code().0)
        })?;
    let _apartment = ComApartment;

    let manager: IApplicationActivationManager =
        unsafe { CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER) }
            .map_err(|err| {
                format_hresult(
                    "Windows app activation manager is unavailable",
                    err.code().0,
                )
            })?;

    let aumid: Vec<u16> = aumid.encode_utf16().chain(std::iter::once(0)).collect();
    let process_id = unsafe {
        manager.ActivateApplication(PCWSTR(aumid.as_ptr()), PCWSTR::null(), AO_NOERRORUI)
    }
    .map_err(|err| {
        format_hresult(
            "Could not activate the OpenAI Codex desktop app",
            err.code().0,
        )
    })?;

    if process_id == 0 {
        return Err("Windows did not return a process for the OpenAI Codex app".into());
    }
    Ok(process_id)
}

fn format_hresult(message: &str, code: i32) -> String {
    format!("{message} (HRESULT 0x{:08X})", code as u32)
}

struct ComApartment;

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_app_id_is_a_package_identity_not_a_deep_link() {
        assert!(CODEX_APP_USER_MODEL_ID.starts_with("OpenAI.Codex_"));
        assert!(CODEX_APP_USER_MODEL_ID.ends_with("!App"));
        assert!(!CODEX_APP_USER_MODEL_ID.contains("://"));
    }

    #[test]
    fn unknown_app_identity_returns_an_activation_error() {
        let error = activate_aumid("AntiScrollingNotch.NotInstalled_0000000000000!Missing")
            .expect_err("an unknown package must not be treated as openable");
        assert!(error.contains("HRESULT"), "{error}");
    }

    #[test]
    #[ignore = "requires the installed stable Codex desktop app and activates its generic launch surface"]
    fn activates_the_installed_stable_codex_app() {
        assert!(activate_codex().unwrap() > 0);
    }
}
