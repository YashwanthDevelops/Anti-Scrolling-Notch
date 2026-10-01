//! Shared Windows runtime identity, generated from the package manifest.

include!(concat!(env!("OUT_DIR"), "/identity.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_identity_is_specific_and_names_startup_independently() {
        assert_eq!(PRODUCT_NAME, "Anti-Scrolling-Notch");
        assert_eq!(APP_IDENTIFIER, "com.yashwanthdevelops.antiscrollingnotch");
        assert_eq!(CREDENTIAL_NAMESPACE, APP_IDENTIFIER);
        assert_eq!(AUTOSTART_VALUE_NAME, PRODUCT_NAME);
        assert_eq!(STORAGE_DIRECTORY, PRODUCT_NAME);
        assert_eq!(RELAY_EXECUTABLE, "anti-scrolling-notch-hook.exe");
    }

    #[test]
    fn relay_pipes_have_a_product_specific_namespace() {
        assert_eq!(APP_PIPE_PREFIX, "anti-scrolling-notch");
        assert_eq!(CODEX_PIPE_PREFIX, "anti-scrolling-notch-codex");
        assert_ne!(APP_PIPE_PREFIX, CODEX_PIPE_PREFIX);
    }
}
