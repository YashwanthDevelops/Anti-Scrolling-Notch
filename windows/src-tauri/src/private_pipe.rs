//! Shared Windows named-pipe server creation with a current-user-only ACL.
//!
//! The Codex observer and inherited hook relay use this helper to keep the
//! DACL, first-instance, and remote-client policies aligned.

use std::ffi::c_void;
use std::io;

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};

pub(crate) fn create_server(
    name: &str,
    sid: &str,
    first: bool,
    max_instances: usize,
) -> io::Result<NamedPipeServer> {
    // A protected DACL containing one allow ACE for the current user's SID.
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
        .max_instances(max_instances)
        .reject_remote_clients(true);
    // Tokio passes these attributes directly to CreateNamedPipeW. Keep the
    // descriptor alive until that synchronous call returns.
    unsafe {
        options.create_with_security_attributes_raw(
            name,
            (&mut attributes as *mut SECURITY_ATTRIBUTES).cast::<c_void>(),
        )
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
    use std::io;
    use std::os::windows::io::AsRawHandle;
    use std::sync::atomic::{AtomicU64, Ordering};

    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW,
        ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
        SE_KERNEL_OBJECT,
    };
    use windows::Win32::Security::{DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR};

    use super::*;

    fn pipe_name(sid: &str) -> String {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        format!(
            r"\\.\pipe\anti-scrolling-notch-private-test-{}-{}-{sid}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
    }

    fn descriptor_sddl(descriptor: PSECURITY_DESCRIPTOR) -> io::Result<String> {
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

    fn actual_dacl(server: &NamedPipeServer) -> io::Result<String> {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        let status = unsafe {
            GetSecurityInfo(
                HANDLE(server.as_raw_handle()),
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
        descriptor_sddl(descriptor)
    }

    fn expected_dacl(sid: &str) -> io::Result<String> {
        // Canonicalize the expected generic read/write mask through Windows.
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
        descriptor_sddl(descriptor)
    }

    #[test]
    fn current_user_acl_and_first_instance_collision_are_enforced() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let sid = crate::win_user::current_user_sid().expect("current-user SID");
            let name = pipe_name(&sid);
            let server = create_server(&name, &sid, true, 2).unwrap();
            assert_eq!(actual_dacl(&server).unwrap(), expected_dacl(&sid).unwrap());
            assert!(create_server(&name, &sid, true, 2).is_err());
        });
    }
}
