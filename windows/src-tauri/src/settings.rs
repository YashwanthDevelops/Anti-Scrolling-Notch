// Preferences, stored as plain JSON in %APPDATA%\Anti-Scrolling-Notch\settings.json.
// No secret ever lands here — API keys live in the Windows Credential Manager.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Settings {
    pub sound_enabled: bool,
    pub sound_volume: f64,
    pub auto_close_interval: f64,
    pub absence_interval: f64,
    pub active_integrations: Vec<String>,
    /// "primary", "cursor", or "monitor:<Windows display device name>".
    pub screen: String,
    /// Distance from the selected display's top edge, in logical pixels.
    pub edge_offset: f64,
    /// Tauri accelerator syntax; kept as text so older settings remain readable.
    pub toggle_shortcut: String,
    pub autostart: bool,
    pub hooks_installed: bool,
    /// Claude model used by the chat. Changeable in the settings window.
    /// Defaulted explicitly so a settings.json written by an older build still loads.
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_model() -> String {
    crate::claude::DEFAULT_MODEL.to_string()
}

fn default_toggle_shortcut() -> String {
    "CommandOrControl+Alt+Shift+Space".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            sound_volume: 0.12,
            auto_close_interval: 15.0,
            absence_interval: 180.0,
            active_integrations: vec![
                "integration_resend".into(),
                "integration_n8n".into(),
                "integration_vercel".into(),
                "integration_github".into(),
            ],
            screen: "primary".into(),
            edge_offset: 0.0,
            toggle_shortcut: default_toggle_shortcut(),
            autostart: false,
            hooks_installed: false,
            model: default_model(),
        }
    }
}

/// %APPDATA%\Anti-Scrolling-Notch
pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    app_data_dir(base)
}

/// %LOCALAPPDATA%\Anti-Scrolling-Notch — where the relay, inbox and log live.
pub fn local_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    app_data_dir(base)
}

fn app_data_dir(base: PathBuf) -> PathBuf {
    base.join(crate::identity::STORAGE_DIRECTORY)
}

pub fn hook_exe_path() -> PathBuf {
    local_dir()
        .join("bin")
        .join(crate::identity::RELAY_EXECUTABLE)
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load() -> Settings {
    match std::fs::read(settings_path()) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let json = serde_json::to_vec_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(settings_path(), json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_and_relay_paths_are_isolated_from_coucou() {
        let appdata = PathBuf::from(r"C:\Users\Tester\AppData\Roaming");
        let local = PathBuf::from(r"C:\Users\Tester\AppData\Local");
        assert_eq!(
            app_data_dir(appdata),
            PathBuf::from(r"C:\Users\Tester\AppData\Roaming\Anti-Scrolling-Notch")
        );
        let local_dir = app_data_dir(local);
        assert!(!local_dir.ends_with("Coucou"));
        assert_eq!(
            local_dir
                .join("bin")
                .join(crate::identity::RELAY_EXECUTABLE),
            PathBuf::from(
                r"C:\Users\Tester\AppData\Local\Anti-Scrolling-Notch\bin\anti-scrolling-notch-hook.exe"
            )
        );
    }

    #[test]
    fn old_settings_receive_new_shell_defaults() {
        let old = r#"{
            "soundEnabled": true,
            "soundVolume": 0.12,
            "autoCloseInterval": 15.0,
            "absenceInterval": 180.0,
            "activeIntegrations": [],
            "screen": "primary",
            "autostart": false,
            "hooksInstalled": false,
            "model": "claude-opus-5"
        }"#;
        let settings: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(settings.edge_offset, 0.0);
        assert_eq!(settings.toggle_shortcut, default_toggle_shortcut());
    }

    #[test]
    fn placement_and_shortcut_settings_round_trip_through_json() {
        let settings = Settings {
            screen: "monitor:\\\\.\\DISPLAY2".into(),
            edge_offset: 48.0,
            toggle_shortcut: "Control+Alt+Shift+Space".into(),
            ..Settings::default()
        };

        let serialized = serde_json::to_vec(&settings).unwrap();
        let restored: Settings = serde_json::from_slice(&serialized).unwrap();

        assert_eq!(restored.screen, settings.screen);
        assert_eq!(restored.edge_offset, settings.edge_offset);
        assert_eq!(restored.toggle_shortcut, settings.toggle_shortcut);
    }
}
