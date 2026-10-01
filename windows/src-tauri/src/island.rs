// Island window: placement on the chosen display, the two window sizes
// (full panel / invisible wake strip), click-through and the cursor poll.
//
// There is no notch on a PC, so the island is a black shape drawn at the top
// centre of the main display inside a borderless, transparent, always-on-top
// window that never takes focus.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

use windows::core::BOOL;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::Ole::RevokeDragDrop;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetClassNameW};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

/// Logical size of the full window — the largest island view, like the macOS panel.
pub const PANEL_W: f64 = 720.0;
pub const PANEL_H: f64 = 320.0;
/// Logical size of the invisible strip that wakes the island when it is hidden.
pub const STRIP_W: f64 = 240.0;
pub const STRIP_H: f64 = 6.0;
/// User-configured distance from a monitor's top edge, in logical pixels.
pub const MAX_EDGE_OFFSET: f64 = 120.0;

pub const WINDOW_LABEL: &str = "island";

/// Margin around the island that still counts as "on the island", in logical px.
/// Wider than the macOS 6 pt because a click must never be swallowed.
const HIT_MARGIN: f64 = 14.0;

#[derive(Serialize, Clone)]
pub struct CursorPayload {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Clone)]
pub struct ScreenInfo {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MonitorOption {
    pub id: String,
    pub label: String,
    pub scale: f64,
}

/// The island shape in window-logical coordinates, pushed by the front end.
/// The poll thread owns the click-through decision so it lands in the same 16 ms
/// tick as the cursor read — an IPC round trip here loses clicks.
#[derive(Clone, Copy, Default)]
pub struct IslandRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Wakes / parks the cursor poll thread so a hidden island costs literally nothing.
pub struct PollGate {
    active: Mutex<bool>,
    cv: Condvar,
    pub collapsed: AtomicBool,
    /// True from Tauri's drag-enter event until drop or the initiating button is released.
    pub file_drag_active: AtomicBool,
    pub rect: Mutex<IslandRect>,
    /// Mirrors the window flag so we only call into Win32 when it changes.
    ignoring: AtomicBool,
}

impl PollGate {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(false),
            cv: Condvar::new(),
            collapsed: AtomicBool::new(true),
            file_drag_active: AtomicBool::new(false),
            rect: Mutex::new(IslandRect::default()),
            ignoring: AtomicBool::new(false),
        }
    }

    pub fn set_rect(&self, rect: IslandRect) {
        *self.rect.lock().unwrap() = rect;
    }

    /// Forces the next poll tick to re-apply the flag (after a window resize).
    pub fn forget_ignore_state(&self) {
        self.ignoring.store(false, Ordering::Relaxed);
    }

    pub fn set_active(&self, on: bool) {
        let mut guard = self.active.lock().unwrap();
        *guard = on;
        self.cv.notify_all();
    }

    fn wait_until_active(&self) {
        let mut guard = self.active.lock().unwrap();
        while !*guard {
            guard = self.cv.wait(guard).unwrap();
        }
    }

    fn is_active(&self) -> bool {
        *self.active.lock().unwrap()
    }
}

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(WINDOW_LABEL)
}

fn cursor_physical() -> Option<(f64, f64)> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok()? };
    Some((p.x as f64, p.y as f64))
}

/// Lets dropped files reach the app again.
///
/// wry installs its drop target by walking the webview's child windows **once**,
/// when the webview is created. WebView2 creates `Chrome_RenderWidgetHostHWND`
/// later and registers its own target on it; being the innermost window, that one
/// wins, and since the page has no HTML5 drop handler it refuses everything — the
/// "no drop" cursor, with nothing reaching Tauri. Revoking it makes OLE fall
/// through to the target wry registered on the parent widget, which is the one
/// that feeds Tauri's drag events.
///
/// Cheap and idempotent, so it is simply re-run whenever a drag might be starting.
pub fn unblock_webview_drops(app: &AppHandle) {
    for label in [WINDOW_LABEL, "settings"] {
        let Some(win) = app.get_webview_window(label) else {
            continue;
        };
        let Some(hwnd) = hwnd_of(&win) else { continue };
        unsafe {
            let _ = EnumChildWindows(Some(hwnd), Some(revoke_render_widget), LPARAM(0));
        }
    }
}

unsafe extern "system" fn revoke_render_widget(hwnd: HWND, _: LPARAM) -> BOOL {
    let mut name = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut name) };
    if len > 0 {
        let class = String::from_utf16_lossy(&name[..len as usize]);
        if class == "Chrome_RenderWidgetHostHWND" {
            let _ = unsafe { RevokeDragDrop(hwnd) };
        }
    }
    true.into()
}

/// True while the left mouse button is held — the only signal we get that a
/// drag might be in flight before it reaches the window.
fn left_button_down() -> bool {
    unsafe { (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) != 0 }
}

fn monitor_contains(m: &Monitor, x: f64, y: f64) -> bool {
    let p = m.position();
    let s = m.size();
    x >= p.x as f64
        && x < (p.x + s.width as i32) as f64
        && y >= p.y as f64
        && y < (p.y + s.height as i32) as f64
}

fn monitor_id(monitor: &Monitor) -> String {
    monitor.name().cloned().unwrap_or_else(|| {
        let p = monitor.position();
        let s = monitor.size();
        format!("{},{}:{}x{}", p.x, p.y, s.width, s.height)
    })
}

pub fn monitor_options(app: &AppHandle) -> Vec<MonitorOption> {
    app.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let id = monitor_id(&monitor);
            let label = monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("Display {}", index + 1));
            MonitorOption {
                id,
                label,
                scale: monitor.scale_factor(),
            }
        })
        .collect()
}

/// The display the island lives on: the primary one, or the one under the cursor.
fn target_monitor(app: &AppHandle, pref: &str) -> Option<Monitor> {
    let monitors = app.available_monitors().ok()?;
    if let Some(id) = pref.strip_prefix("monitor:") {
        if let Some(monitor) = monitors.iter().find(|monitor| monitor_id(monitor) == id) {
            return Some(monitor.clone());
        }
    }
    if pref == "cursor" {
        if let Some((cx, cy)) = cursor_physical() {
            if let Some(m) = monitors.iter().find(|m| monitor_contains(m, cx, cy)) {
                return Some(m.clone());
            }
        }
    }
    app.primary_monitor()
        .ok()
        .flatten()
        .or_else(|| monitors.into_iter().next())
}

fn geometry_for_monitor(
    monitor_position: PhysicalPosition<i32>,
    monitor_size: PhysicalSize<u32>,
    scale: f64,
    collapsed: bool,
    edge_offset: f64,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let (requested_width, requested_height) = if collapsed {
        (STRIP_W, STRIP_H)
    } else {
        (PANEL_W, PANEL_H)
    };
    let logical_monitor_width = monitor_size.width as f64 / scale;
    let logical_monitor_height = monitor_size.height as f64 / scale;
    let logical_width = requested_width.min(logical_monitor_width);
    let logical_height = requested_height.min(logical_monitor_height);
    let width = (logical_width * scale).round().max(1.0) as u32;
    let height = (logical_height * scale).round().max(1.0) as u32;
    let requested_offset = if edge_offset.is_finite() {
        edge_offset.clamp(0.0, MAX_EDGE_OFFSET)
    } else {
        0.0
    };
    let offset = requested_offset.min((logical_monitor_height - logical_height).max(0.0));
    let x = monitor_position.x + (monitor_size.width as i32 - width as i32) / 2;
    let y = monitor_position.y + (offset * scale).round() as i32;
    (
        PhysicalPosition::new(x, y),
        PhysicalSize::new(width, height),
    )
}

pub fn screen_info(app: &AppHandle, pref: &str) -> ScreenInfo {
    match target_monitor(app, pref) {
        Some(m) => {
            let scale = m.scale_factor();
            let p = m.position();
            let s = m.size();
            ScreenInfo {
                x: p.x as f64 / scale,
                y: p.y as f64 / scale,
                width: s.width as f64 / scale,
                height: s.height as f64 / scale,
                scale,
            }
        }
        None => ScreenInfo {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
            scale: 1.0,
        },
    }
}

/// Places and sizes the window. `collapsed` picks the wake strip instead of the panel.
pub fn apply_geometry(app: &AppHandle, pref: &str, collapsed: bool, edge_offset: f64) {
    let Some(win) = window(app) else { return };
    let Some(m) = target_monitor(app, pref) else {
        return;
    };

    let scale = m.scale_factor();
    let mp = *m.position();
    let ms = *m.size();
    let (position, size) = geometry_for_monitor(mp, ms, scale, collapsed, edge_offset);

    let _ = win.set_size(size);
    let _ = win.set_position(position);
    // Moving across displays can rescale the window: re-assert the physical size.
    let _ = win.set_size(size);
    let _ = win.set_always_on_top(true);
}

fn hwnd_of(win: &WebviewWindow) -> Option<HWND> {
    let raw = win.hwnd().ok()?.0 as isize;
    if raw == 0 {
        return None;
    }
    Some(HWND(raw as *mut _))
}

/// WS_EX_NOACTIVATE keeps clicks from stealing focus; WS_EX_TOOLWINDOW keeps the
/// island out of Alt-Tab.
pub fn make_non_activating(win: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(win) else { return };
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let want = ex | WS_EX_NOACTIVATE.0 as isize | WS_EX_TOOLWINDOW.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, want);
    }
}

/// Temporarily allow activation so a text field inside the island can be typed in.
pub fn set_activating(win: &WebviewWindow, activating: bool) {
    let Some(hwnd) = hwnd_of(win) else { return };
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let want = if activating {
            ex & !(WS_EX_NOACTIVATE.0 as isize)
        } else {
            ex | WS_EX_NOACTIVATE.0 as isize
        };
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, want);
    }
}

type MonitorFingerprint = (String, i32, i32, u32, u32, u64);
type DisplayLayoutKey = (String, Option<String>, Vec<MonitorFingerprint>);

/// Selected display identity and full layout. The full layout refreshes monitor
/// choices after connect/disconnect, while the target catches cursor-follow moves.
fn current_screen_key(app: &AppHandle) -> Option<DisplayLayoutKey> {
    let pref = app
        .try_state::<crate::Shared>()
        .map(|s| s.settings.lock().unwrap().screen.clone())
        .unwrap_or_else(|| "primary".into());
    let target = target_monitor(app, &pref).map(|monitor| monitor_id(&monitor))?;
    let primary = app
        .primary_monitor()
        .ok()
        .flatten()
        .as_ref()
        .map(monitor_id);
    let mut monitors: Vec<_> = app
        .available_monitors()
        .ok()?
        .into_iter()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            (
                monitor_id(&monitor),
                position.x,
                position.y,
                size.width,
                size.height,
                monitor.scale_factor().to_bits(),
            )
        })
        .collect();
    monitors.sort_by(|left, right| left.0.cmp(&right.0));
    Some((target, primary, monitors))
}

/// Emits `cursor` (window-logical coordinates) at ~60 Hz while the island is
/// visible. Parked on a condvar the rest of the time.
pub fn spawn_cursor_poll(app: AppHandle, gate: Arc<PollGate>) {
    std::thread::spawn(move || {
        let mut was_down = false;
        // Remembered across wakes so a display change while hidden is noticed the
        // moment the island comes back.
        let mut last_screen = None;
        let mut screen_initialized = false;
        loop {
            gate.wait_until_active();
            let mut last = (f64::MIN, f64::MIN);
            let mut ticks: u32 = 0;
            while gate.is_active() {
                std::thread::sleep(Duration::from_millis(16));

                // Monitors get plugged in, unplugged, rearranged and rescaled, and
                // an island pinned to coordinates that no longer exist is an island
                // nobody can reach. Checked about twice a second — the cursor poll
                // is already running, so this costs one monitor query.
                ticks = ticks.wrapping_add(1);
                if ticks.is_multiple_of(30) {
                    let now = current_screen_key(&app);
                    if now != last_screen {
                        let first = !screen_initialized;
                        last_screen = now;
                        screen_initialized = true;
                        if !first && last_screen.is_some() {
                            crate::log::line("display layout changed — repositioning");
                            let _ = app.emit("screen-changed", ());
                        }
                    }
                }

                let Some(win) = window(&app) else { continue };
                let Ok(origin) = win.outer_position() else {
                    continue;
                };
                let scale = win.scale_factor().unwrap_or(1.0);
                let Some((cx, cy)) = cursor_physical() else {
                    continue;
                };
                let x = (cx - origin.x as f64) / scale;
                let y = (cy - origin.y as f64) / scale;

                // Keep an in-flight Explorer drag from expiring the island if it
                // leaves the drop surface and is then cancelled elsewhere.
                let down = left_button_down();
                if down && !was_down {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || unblock_webview_drops(&handle));
                }
                if !down && gate.file_drag_active.swap(false, Ordering::Relaxed) {
                    let _ = app.emit_to(WINDOW_LABEL, "file-drag-cancelled", ());
                }
                was_down = down;

                let size = match win.inner_size() {
                    Ok(s) => (s.width as f64 / scale, s.height as f64 / scale),
                    Err(_) => (PANEL_W, PANEL_H),
                };
                if (x - last.0).abs() < 1.0 && (y - last.1).abs() < 1.0 {
                    continue;
                }
                last = (x, y);

                // Click-through: the window only takes the mouse over the island
                // shape. A small entry margin means the flag is already off by the
                // time a moving cursor reaches a button.
                let r = *gate.rect.lock().unwrap();
                let on_island = r.w > 0.0
                    && x >= r.x - HIT_MARGIN
                    && x <= r.x + r.w + HIT_MARGIN
                    && y >= r.y - HIT_MARGIN
                    && y <= r.y + r.h + HIT_MARGIN;

                // A file being dragged has to be able to find us. WS_EX_TRANSPARENT
                // — what click-through is on Windows — hides the window from
                // WindowFromPoint, so OLE finds no drop target and shows the "no
                // drop" cursor. macOS has no such problem: AppKit delivers drags to
                // registered destinations whatever ignoresMouseEvents says. So while
                // a button is held anywhere over the panel, the whole panel takes
                // the mouse, which also makes the drop zone as forgiving as the Mac's.
                let dragging = down && x >= 0.0 && x <= size.0 && y >= 0.0 && y <= size.1;

                let accept = on_island || dragging;
                if gate.ignoring.load(Ordering::Relaxed) == accept {
                    gate.ignoring.store(!accept, Ordering::Relaxed);
                    let _ = win.set_ignore_cursor_events(!accept);
                }

                let _ = win.emit("cursor", CursorPayload { x, y });
            }
        }
    });
}

pub fn set_ignore_cursor(app: &AppHandle, ignore: bool) {
    if let Some(win) = window(app) {
        let _ = win.set_ignore_cursor_events(ignore);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_offset_and_panel_size_scale_once_on_negative_origin_monitor() {
        let (position, size) = geometry_for_monitor(
            PhysicalPosition::new(-1920, 0),
            PhysicalSize::new(3840, 2160),
            1.5,
            false,
            24.0,
        );

        assert_eq!(position, PhysicalPosition::new(-540, 36));
        assert_eq!(size, PhysicalSize::new(1080, 480));
    }

    #[test]
    fn wake_strip_uses_same_offset_and_logical_geometry() {
        let (position, size) = geometry_for_monitor(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(1920, 1080),
            1.25,
            true,
            12.0,
        );

        assert_eq!(position, PhysicalPosition::new(810, 15));
        assert_eq!(size, PhysicalSize::new(300, 8));
    }

    #[test]
    fn edge_offset_is_capped_and_keeps_panel_inside_short_display() {
        let (position, size) = geometry_for_monitor(
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(1200, 400),
            1.0,
            false,
            120.0,
        );

        assert_eq!(position.y, 80);
        assert_eq!(position.x, 240);
        assert_eq!(size, PhysicalSize::new(720, 320));
    }

    #[test]
    fn geometry_clamps_panel_to_a_small_monitor_and_sanitizes_offset() {
        let (position, size) = geometry_for_monitor(
            PhysicalPosition::new(-800, -100),
            PhysicalSize::new(600, 240),
            1.0,
            false,
            f64::NAN,
        );

        assert_eq!(position, PhysicalPosition::new(-800, -100));
        assert_eq!(size, PhysicalSize::new(600, 240));
    }
}
