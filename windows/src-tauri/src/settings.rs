// Versioned preferences in %APPDATA%\Anti-Scrolling-Notch\settings.json.
// No secret ever lands here — API keys live in the Windows Credential Manager.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const SETTINGS_SCHEMA_VERSION: u32 = 1;
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Settings {
    pub sound_enabled: bool,
    pub sound_volume: f64,
    pub reduced_motion: bool,
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
    /// Retain redacted turn summaries/plans in local history. Off by default.
    pub retain_history_content: bool,
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
            reduced_motion: false,
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
            retain_history_content: false,
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

pub fn history_path() -> PathBuf {
    local_dir().join("history.sqlite3")
}

pub fn load() -> Settings {
    let path = settings_path();
    match load_from(&path) {
        Ok(Some((settings, migrated))) => {
            if migrated {
                if let Err(error) = save_to(&path, &settings) {
                    crate::log::line(format!("settings migration could not be saved: {error}"));
                }
            }
            settings
        }
        Ok(None) => Settings::default(),
        Err(error) => {
            crate::log::line(format!("settings could not be loaded: {error}"));
            Settings::default()
        }
    }
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    save_to(&settings_path(), settings)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VersionedSettings {
    schema_version: u32,
    settings: Settings,
}

fn load_from(path: &std::path::Path) -> std::io::Result<Option<(Settings, bool)>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;

    let Some(version) = value.get("schemaVersion") else {
        let settings = serde_json::from_value(value)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        return Ok(Some((settings, true)));
    };
    let version = version.as_u64().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "settings schemaVersion must be an unsigned integer",
        )
    })?;
    if version != u64::from(SETTINGS_SCHEMA_VERSION) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unsupported settings schema version {version}"),
        ));
    }

    let settings: VersionedSettings = serde_json::from_value(value)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok(Some((settings.settings, false)))
}

fn save_to(path: &std::path::Path, settings: &Settings) -> std::io::Result<()> {
    // Do not let a save from the default in-memory settings overwrite a file
    // this build cannot interpret. In particular, a newer schema must remain
    // available to the version that wrote it.
    let _existing = load_from(path)?;
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "settings path has no parent",
        )
    })?;
    std::fs::create_dir_all(dir)?;
    let persisted = VersionedSettings {
        schema_version: SETTINGS_SCHEMA_VERSION,
        settings: settings.clone(),
    };
    let json = serde_json::to_vec_pretty(&persisted)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "settings path has no file name",
        )
    })?;
    let mut temp_name = file_name.to_os_string();
    temp_name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let temp_path = dir.join(temp_name);
    let result = (|| {
        use std::io::Write;

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(&json)?;
        file.sync_all()?;
        drop(file);
        // The temporary file shares the destination directory, so Windows
        // performs an atomic same-volume replacement on supported Windows 10+.
        std::fs::rename(&temp_path, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
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
        assert!(!settings.reduced_motion);
        assert!(!settings.retain_history_content);
    }

    #[test]
    fn placement_and_shortcut_settings_round_trip_through_json() {
        let settings = Settings {
            screen: "monitor:\\\\.\\DISPLAY2".into(),
            edge_offset: 48.0,
            toggle_shortcut: "Control+Alt+Shift+Space".into(),
            sound_enabled: false,
            sound_volume: 0.075,
            reduced_motion: true,
            ..Settings::default()
        };

        let serialized = serde_json::to_vec(&settings).unwrap();
        let restored: Settings = serde_json::from_slice(&serialized).unwrap();

        assert_eq!(restored.screen, settings.screen);
        assert_eq!(restored.edge_offset, settings.edge_offset);
        assert_eq!(restored.toggle_shortcut, settings.toggle_shortcut);
        assert_eq!(restored.sound_enabled, settings.sound_enabled);
        assert_eq!(restored.sound_volume, settings.sound_volume);
        assert!(restored.reduced_motion);
    }

    #[test]
    fn settings_write_atomically_and_migrate_legacy_json() {
        let directory = test_directory("settings-migration");
        let path = directory.join("settings.json");
        std::fs::write(
            &path,
            br#"{"soundEnabled":false,"soundVolume":0.08,"model":"claude-sonnet-5"}"#,
        )
        .unwrap();

        let (settings, migrated) = load_from(&path).unwrap().unwrap();
        assert!(migrated);
        assert!(!settings.sound_enabled);
        assert!(!settings.retain_history_content);
        save_to(&path, &settings).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["schemaVersion"], SETTINGS_SCHEMA_VERSION);
        assert_eq!(saved["settings"]["model"], "claude-sonnet-5");
        assert!(!load_from(&path).unwrap().unwrap().1);
        assert_eq!(
            std::fs::read_dir(&directory).unwrap().count(),
            1,
            "temporary replacement files are removed"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn future_settings_schema_is_preserved_for_a_newer_build() {
        let directory = test_directory("settings-future");
        let path = directory.join("settings.json");
        let contents = br#"{"schemaVersion":99,"settings":{}}"#;
        std::fs::write(&path, contents).unwrap();

        assert!(load_from(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), contents);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_settings_are_not_replaced_by_a_default_save() {
        let directory = test_directory("settings-malformed");
        let path = directory.join("settings.json");
        let contents = br#"{"soundEnabled": "not-a-boolean"}"#;
        std::fs::write(&path, contents).unwrap();

        assert!(save_to(&path, &Settings::default()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), contents);
        std::fs::remove_dir_all(directory).unwrap();
    }

    fn test_directory(label: &str) -> PathBuf {
        static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("storage-tests")
            .join(format!(
                "{label}-{}-{}",
                std::process::id(),
                NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
