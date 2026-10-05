// Anti-Scrolling-Notch for Windows — app wiring and the commands the island calls.

pub mod broker;
mod capabilities;
mod claude;
mod codex_hooks;
mod codex_navigation;
mod codex_pipe;
mod files;
mod hooks;
mod identity;
mod integrations;
mod island;
mod log;
mod pipe;
mod private_pipe;
mod secrets;
mod settings;
pub mod storage;
mod tray;
mod win_user;

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use codex_hooks::CodexHookStatus;
use files::DroppedFile;
use hooks::{HookPreview, HookStatus};
use island::{PollGate, ScreenInfo};
use pipe::Pending;
use settings::Settings;

/// Keeps spawned helpers from flashing a console window.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Shared {
    pub settings: Mutex<Settings>,
    pub history: Mutex<Option<storage::HistoryStore>>,
    pub broker: broker::service::BrokerService,
    pub gate: Arc<PollGate>,
    shortcut_status: Mutex<ShortcutStatus>,
}

#[tauri::command]
fn broker_sync(
    shared: State<Shared>,
    after_sequence: u64,
) -> Result<broker::stream::BrokerSyncResponse, String> {
    shared
        .broker
        .synchronize_after(after_sequence)
        .map_err(|error| error.to_string())
}

/// Publish an adapter-normalized broker update and notify the view store.
/// Raw hook payloads must never be passed here; each producer must first meet
/// its own compatibility and capability gate.
pub fn publish_broker_update(
    app: &AppHandle,
    shared: &Shared,
    update: broker::reducer::BrokerUpdate,
    observed_at_unix_ms: u64,
    source_event_id: Option<broker::types::OpaqueId>,
    deduplication_key: Option<broker::types::OpaqueId>,
) -> Result<broker::stream::ApplyReceipt, String> {
    let (receipt, notification) = shared
        .broker
        .apply(
            update,
            observed_at_unix_ms,
            source_event_id,
            deduplication_key,
        )
        .map_err(|error| error.to_string())?;
    if let Some(notification) = notification {
        app.emit("broker-sync", notification)
            .map_err(|error| format!("could not notify the frontend broker view store: {error}"))?;
    }
    Ok(receipt)
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShortcutStatus {
    active: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootInfo {
    settings: Settings,
    screen: ScreenInfo,
    monitors: Vec<island::MonitorOption>,
    shortcut_status: ShortcutStatus,
    version: String,
    hook_path: String,
    capabilities: capabilities::CapabilityRegistry,
}

#[tauri::command]
fn boot(app: AppHandle, shared: State<Shared>) -> BootInfo {
    let mut settings = shared.settings.lock().unwrap().clone();
    // The real state of ~/.claude/settings.json wins over whatever we stored.
    settings.hooks_installed = hooks::status().installed;
    let screen = island::screen_info(&app, &settings.screen);
    let monitors = island::monitor_options(&app);
    let shortcut_status = shared.shortcut_status.lock().unwrap().clone();
    BootInfo {
        settings,
        screen,
        monitors,
        shortcut_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        hook_path: settings::hook_exe_path().to_string_lossy().to_string(),
        capabilities: capabilities::CapabilityRegistry::discover(),
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, shared: State<Shared>, settings: Settings) {
    let (screen_changed, autostart_changed, settings) = {
        let mut current = shared.settings.lock().unwrap();
        let autostart_changed = current.autostart != settings.autostart;
        let mut settings = settings;
        // The shortcut has its own fallible command so a failed registration
        // cannot be persisted as if it were active.
        settings.toggle_shortcut = current.toggle_shortcut.clone();
        settings.edge_offset = if settings.edge_offset.is_finite() {
            settings.edge_offset.clamp(0.0, island::MAX_EDGE_OFFSET)
        } else {
            current.edge_offset
        };
        let placement_changed =
            current.screen != settings.screen || current.edge_offset != settings.edge_offset;
        *current = settings.clone();
        (placement_changed, autostart_changed, settings)
    };
    if let Err(err) = settings::save(&settings) {
        eprintln!("[anti-scrolling-notch] could not save settings: {err}");
    }
    if autostart_changed {
        let manager = app.autolaunch();
        let result = if settings.autostart {
            manager.enable()
        } else {
            manager.disable()
        };
        if let Err(err) = result {
            eprintln!("[anti-scrolling-notch] autostart: {err}");
        }
    }
    if screen_changed {
        let collapsed = shared.gate.collapsed.load(Ordering::Relaxed);
        island::apply_geometry(&app, &settings.screen, collapsed, settings.edge_offset);
    }
    // Keep the other window in step (island ⇄ settings window).
    let _ = app.emit("settings-changed", settings);
}

/// Hidden island → shrink the window to the invisible wake strip and park the
/// cursor poll; anything else → full panel and 60 Hz polling.
#[tauri::command]
fn set_collapsed(app: AppHandle, shared: State<Shared>, collapsed: bool) {
    let settings = shared.settings.lock().unwrap().clone();
    shared.gate.collapsed.store(collapsed, Ordering::Relaxed);
    island::apply_geometry(&app, &settings.screen, collapsed, settings.edge_offset);
    // The wake strip must always take the mouse, and a resize invalidates the flag.
    island::set_ignore_cursor(&app, false);
    shared.gate.forget_ignore_state();
    shared.gate.set_active(!collapsed);
}

/// The front end pushes the island shape; Rust decides click-through from it.
#[tauri::command]
fn set_island_rect(shared: State<Shared>, x: f64, y: f64, width: f64, height: f64) {
    shared.gate.set_rect(island::IslandRect {
        x,
        y,
        w: width,
        h: height,
    });
}

#[tauri::command]
fn focus_window(app: AppHandle, focused: bool) {
    let Some(win) = island::window(&app) else {
        return;
    };
    island::set_activating(&win, focused);
    if focused {
        let _ = win.set_focus();
    }
}

#[tauri::command]
fn reposition(app: AppHandle, shared: State<Shared>) {
    let settings = shared.settings.lock().unwrap().clone();
    let collapsed = shared.gate.collapsed.load(Ordering::Relaxed);
    island::apply_geometry(&app, &settings.screen, collapsed, settings.edge_offset);
}

#[tauri::command]
fn monitor_options(app: AppHandle) -> Vec<island::MonitorOption> {
    island::monitor_options(&app)
}

fn parse_shortcut(value: &str) -> Result<Shortcut, String> {
    Shortcut::from_str(value.trim()).map_err(|err| format!("Invalid shortcut: {err}"))
}

#[tauri::command]
fn set_toggle_shortcut(
    app: AppHandle,
    shared: State<Shared>,
    shortcut: String,
) -> Result<ShortcutStatus, String> {
    let shortcut_text = shortcut.trim().to_string();
    if shortcut_text.is_empty() {
        return Err("Enter a global shortcut.".into());
    }
    let new_shortcut = parse_shortcut(&shortcut_text)?;
    let mut settings = shared.settings.lock().unwrap();
    let previous = settings.toggle_shortcut.clone();
    let previous_status = shared.shortcut_status.lock().unwrap().clone();
    if previous == shortcut_text && previous_status.active.as_deref() == Some(&shortcut_text) {
        return Ok(previous_status);
    }

    let manager = app.global_shortcut();
    manager
        .register(new_shortcut)
        .map_err(|err| format!("Could not register shortcut: {err}"))?;

    if let Some(active) = previous_status.active.as_deref() {
        if active != shortcut_text {
            if let Ok(old_shortcut) = parse_shortcut(active) {
                if let Err(err) = manager.unregister(old_shortcut) {
                    if let Ok(new_shortcut) = parse_shortcut(&shortcut_text) {
                        let _ = manager.unregister(new_shortcut);
                    }
                    return Err(format!("Could not replace the active shortcut: {err}"));
                }
            }
        }
    }

    let mut updated = settings.clone();
    updated.toggle_shortcut = shortcut_text.clone();
    if let Err(err) = settings::save(&updated) {
        if let Ok(new_shortcut) = parse_shortcut(&shortcut_text) {
            let _ = manager.unregister(new_shortcut);
        }
        let restore = previous_status
            .active
            .as_deref()
            .and_then(|active| parse_shortcut(active).ok())
            .and_then(|old| manager.register(old).err());
        if let Some(restore_err) = restore {
            *shared.shortcut_status.lock().unwrap() = ShortcutStatus {
                active: None,
                error: Some(format!("Settings could not be saved ({err}); prior shortcut could not be restored ({restore_err}).")),
            };
        }
        return Err(format!("Could not save shortcut setting: {err}"));
    }

    *settings = updated;
    let status = ShortcutStatus {
        active: Some(shortcut_text),
        error: None,
    };
    *shared.shortcut_status.lock().unwrap() = status.clone();
    drop(settings);
    let _ = app.emit("settings-changed", shared.settings.lock().unwrap().clone());
    Ok(status)
}

#[cfg(test)]
mod shortcut_tests {
    use super::parse_shortcut;

    #[test]
    fn default_shortcut_uses_supported_accelerator_syntax() {
        assert!(parse_shortcut("CommandOrControl+Alt+Shift+Space").is_ok());
    }

    #[test]
    fn invalid_shortcut_is_rejected_before_registration() {
        assert!(parse_shortcut("not a shortcut").is_err());
    }
}

#[tauri::command]
fn set_file_drag_active(shared: State<Shared>, active: bool) {
    shared
        .gate
        .file_drag_active
        .store(active, Ordering::Relaxed);
}

#[tauri::command]
fn open_url(url: String) {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return;
    }
    let _ = Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

/// Best-effort return to the registered Codex desktop app's generic surface.
/// Exact conversation/session destinations are not verified by the hook adapter.
#[tauri::command]
fn open_codex(expected_generation: String) -> Result<u32, String> {
    let current = capabilities::CapabilityRegistry::discover();
    capabilities::execute(
        &current,
        &expected_generation,
        capabilities::OPEN_CODEX_CAPABILITY,
        codex_navigation::activate_codex,
    )
}

/// "Open terminal" opens the working folder in VS Code when `code` is on PATH,
/// and falls back to Explorer otherwise.
#[tauri::command]
fn open_in_vscode(path: Option<String>) -> bool {
    // No `cmd /C` anywhere near this. The path is a project folder chosen by
    // whoever is using Claude Code, and cmd would happily read `&`, `^` and `%`
    // in a folder name as syntax. Finding the launcher ourselves and handing the
    // path over as a separate argument keeps it a path.
    if let Some(code) = find_on_path("code") {
        let mut cmd = Command::new(code);
        if let Some(p) = path.as_deref().filter(|p| !p.is_empty()) {
            cmd.arg(p);
        }
        if cmd.creation_flags(CREATE_NO_WINDOW).spawn().is_ok() {
            return true;
        }
    }
    if let Some(p) = path.as_deref().filter(|p| !p.is_empty()) {
        let _ = Command::new("explorer").arg(p).spawn();
    }
    false
}

/// Our own `where`: walks %PATH% against %PATHEXT%, no shell involved.
/// Rust quotes arguments correctly for `.cmd`/`.bat` targets since 1.77, so
/// spawning `code.cmd` directly is safe.
fn find_on_path(stem: &str) -> Option<std::path::PathBuf> {
    let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    let dirs = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&dirs) {
        for ext in exts.split(';').filter(|e| !e.is_empty()) {
            let candidate = dir.join(format!("{stem}{}", ext.to_lowercase()));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Tray → Pause. Paused means paused: the pollers stop talking to the network,
/// not just the island stopping showing things.
#[tauri::command]
fn set_paused(paused: bool) {
    integrations::set_paused(paused);
}

// ── Claude Code hooks ─────────────────────────────────────────────────────────

#[tauri::command]
fn hooks_status() -> HookStatus {
    hooks::status()
}

/// Returns the diff the user has to look at before anything is written.
#[tauri::command]
fn hooks_preview(install: bool) -> Result<HookPreview, String> {
    hooks::preview(install)
}

/// Only ever called from an explicit click in the settings window.
#[tauri::command]
fn hooks_apply(
    app: AppHandle,
    shared: State<Shared>,
    install: bool,
    fingerprint: String,
) -> Result<String, String> {
    // The fingerprint comes from the preview the user actually looked at, so a
    // settings.json that changed in between is refused rather than overwritten.
    let backup = hooks::write(install, &fingerprint)?;
    let updated = {
        let mut current = shared.settings.lock().unwrap();
        current.hooks_installed = install;
        let _ = settings::save(&current);
        current.clone()
    };
    let _ = app.emit("settings-changed", updated);
    Ok(backup)
}

// ── Codex CLI hook configuration ─────────────────────────────────────────────

#[tauri::command]
fn codex_hooks_status() -> Result<CodexHookStatus, String> {
    codex_hooks::status()
}

/// Returns the exact user-level hooks.json diff before any write occurs.
#[tauri::command]
fn codex_hooks_preview(install: bool) -> Result<HookPreview, String> {
    codex_hooks::preview(install)
}

/// Applies or removes only the Codex observer entries shown by the preview.
#[tauri::command]
fn codex_hooks_apply(install: bool, fingerprint: String, backup: String) -> Result<String, String> {
    codex_hooks::write(install, &fingerprint, &backup)
}

#[tauri::command]
fn approval_decision(app: AppHandle, request_id: String, decision: String) {
    pipe::answer(&app, &request_id, &decision);
}

/// The island has the card on screen, so the long wait for a human may begin.
/// Until this arrives the relay only waits a few hundred milliseconds, which is
/// what stops a paused or unresponsive island from freezing Claude Code.
#[tauri::command]
fn approval_ack(app: AppHandle, request_id: String) {
    pipe::acknowledge(&app, &request_id);
}

/// Nobody can act on this request — the island is paused, or another card is
/// already up. Claude Code falls back to asking in the terminal immediately.
#[tauri::command]
fn approval_decline(app: AppHandle, request_id: String) {
    pipe::decline(&app, &request_id);
}

// ── Chat, files and secrets ───────────────────────────────────────────────────

/// Copies a dropped file into the inbox and reports its name back.
#[tauri::command]
fn ingest_file(path: String, capability_generation: String) -> Result<DroppedFile, String> {
    let current = capabilities::CapabilityRegistry::discover();
    capabilities::execute(
        &current,
        &capability_generation,
        "codex.attachmentDelivery",
        || files::ingest(&path),
    )
}

/// The island may only ask whether a key exists — never read it.
#[tauri::command]
fn secret_present(key: String) -> bool {
    secrets::present(&key)
}

#[tauri::command]
fn secret_set(key: String, value: String) -> Result<(), String> {
    secrets::set(&key, &value)
}

#[tauri::command]
fn secret_clear(key: String) -> Result<(), String> {
    secrets::clear(&key)
}

/// Opens the configured n8n instance — the URL lives in the Credential Manager.
#[tauri::command]
fn open_n8n() {
    if let Some(url) = secrets::get("n8n-url") {
        open_url(url);
    }
}

/// Refresh buttons in the integration cards.
#[tauri::command]
async fn refresh_integration(app: AppHandle, id: String) {
    integrations::poll_once(app, &id).await;
}

/// Lets the island write to the same log as the Rust side.
#[tauri::command]
fn log_line(message: String) {
    log::line(format!("ui  {message}"));
}

// ── Settings window ───────────────────────────────────────────────────────────

/// WebView2 allows exactly one browser environment per app, and its options are
/// fixed by whichever webview is created first. Every window must therefore ask
/// for the *same* arguments as the island (see `additionalBrowserArgs` in
/// tauri.conf.json) — a mismatch makes the second window come up blank, with no
/// error anywhere.
const BROWSER_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI --autoplay-policy=no-user-gesture-required";

/// In a dev build the pages are served by Vite, so the second window needs the
/// absolute dev URL; a bundled build resolves it inside the app bundle.
fn settings_page_url(app: &AppHandle) -> WebviewUrl {
    #[cfg(dev)]
    if let Some(mut base) = app.config().build.dev_url.clone() {
        base.set_path("/settings.html");
        return WebviewUrl::External(base);
    }
    let _ = app;
    WebviewUrl::App("settings.html".into())
}

/// The settings window is created hidden at launch and only ever shown and
/// hidden afterwards. A WebView2 window created later — on the main thread or
/// not — silently comes up blank in this app, so the window that works is the
/// one that exists before the island's webview does.
fn create_settings_window(app: &AppHandle) {
    let url = settings_page_url(app);
    match WebviewWindowBuilder::new(app, "settings", url)
        .additional_browser_args(BROWSER_ARGS)
        .title("Settings — Anti-Scrolling-Notch")
        .inner_size(560.0, 680.0)
        .min_inner_size(460.0, 480.0)
        .resizable(true)
        .visible(false)
        .center()
        .build()
    {
        Ok(win) => {
            // Closing it must only hide it, or it could never be reopened.
            let hidden = win.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = hidden.hide();
                }
            });
        }
        Err(err) => log::line(format!("settings window failed: {err}")),
    }
}

pub fn show_settings_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("settings") else {
        log::line("settings window missing");
        return;
    };
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_focus();
}

#[tauri::command]
fn open_settings_window(app: AppHandle) {
    show_settings_window(&app);
}

pub fn run() {
    let loaded = settings::load();
    let history = match storage::HistoryStore::open(&settings::history_path()) {
        Ok(history) => Some(history),
        Err(error) => {
            log::line(format!("local history storage unavailable: {error}"));
            None
        }
    };
    let gate = Arc::new(PollGate::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let _ = app.emit_to(island::WINDOW_LABEL, "tray", "open".to_string());
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name(identity::AUTOSTART_VALUE_NAME)
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = app.emit_to(island::WINDOW_LABEL, "toggle-island", ());
                    }
                })
                .build(),
        )
        .manage(Shared {
            settings: Mutex::new(loaded.clone()),
            history: Mutex::new(history),
            broker: broker::service::BrokerService::default(),
            gate: gate.clone(),
            shortcut_status: Mutex::new(ShortcutStatus::default()),
        })
        .manage(Pending::default())
        .invoke_handler(tauri::generate_handler![
            broker_sync,
            boot,
            save_settings,
            monitor_options,
            set_toggle_shortcut,
            set_file_drag_active,
            set_collapsed,
            set_island_rect,
            focus_window,
            reposition,
            open_url,
            open_codex,
            open_in_vscode,
            quit_app,
            hooks_status,
            hooks_preview,
            hooks_apply,
            codex_hooks_status,
            codex_hooks_preview,
            codex_hooks_apply,
            approval_decision,
            approval_ack,
            approval_decline,
            log_line,
            ingest_file,
            secret_present,
            secret_set,
            secret_clear,
            refresh_integration,
            open_n8n,
            open_settings_window,
            set_paused,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let shared = app.state::<Shared>();
            match parse_shortcut(&loaded.toggle_shortcut).and_then(|shortcut| {
                handle
                    .global_shortcut()
                    .register(shortcut)
                    .map_err(|err| err.to_string())
            }) {
                Ok(()) => {
                    shared.shortcut_status.lock().unwrap().active =
                        Some(loaded.toggle_shortcut.clone());
                }
                Err(err) => {
                    let message = format!("Could not register startup shortcut: {err}");
                    log::line(message.clone());
                    shared.shortcut_status.lock().unwrap().error = Some(message);
                }
            }
            tray::build(&handle)?;
            // Before the island: see create_settings_window.
            create_settings_window(&handle);

            if let Some(win) = island::window(&handle) {
                island::make_non_activating(&win);
                island::apply_geometry(&handle, &loaded.screen, false, loaded.edge_offset);
                let _ = win.show();
            }
            gate.collapsed.store(false, Ordering::Relaxed);
            gate.set_active(true);
            island::spawn_cursor_poll(handle.clone(), gate.clone());

            log::line(format!(
                "--- {} {} started ---",
                identity::PRODUCT_NAME,
                env!("CARGO_PKG_VERSION")
            ));
            hooks::ensure_hook_exe(&handle);
            pipe::start(handle.clone());
            codex_pipe::start(handle.clone());
            integrations::start(handle.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Anti-Scrolling-Notch");
}
