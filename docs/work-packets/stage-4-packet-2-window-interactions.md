# Stage 4 packet 2 — overlay placement and interaction behavior

## Packet identity and scope

| Field | Value |
|---|---|
| Packet | Stage 4, packet 2: overlay placement and interaction behavior |
| Release profile | Monitor MVP shell foundation |
| Task IDs | SHELL-04, SHELL-05, SHELL-06 |
| Branch / PR | `work/stage-4-shell-window-interactions` → Anti-Scrolling-Notch `main` |
| Dependencies | Stage 2 BASE-05 owner-accepted; Stage 3 compatibility packets integrated; Stage 4 packet 1 (SHELL-01–03) merged at `44b15a1fad0d6923a3241693a04307108ddb7311`; packet 1 merge-record follow-up integrated before packet activation |
| User outcome | The retained Coucou-style island stays reachable and non-disruptive, can be placed on a chosen display with predictable logical-pixel geometry, and cannot hide or replace an actionable request through cosmetic interaction. |
| In scope | Preserve the current topmost/transparent/non-activating/click-through/wake-strip shell; remove only the unnecessary WebView2 SmartScreen-disable override; support primary, fixed named-monitor, and follow-cursor placement; persist a bounded top-edge offset; add a configurable global toggle shortcut with visible startup/conflict failure and safe replacement; keep Escape, pointer-leave, text entry, drag and pinned-request behavior coherent. |
| UI/assets contract | Preserve Coucou/Mochi layout, artwork, sounds, animation, timing, transitions and existing island interactions. Add only the display/offset/shortcut settings needed by SHELL-05. No asset bytes or art are changed. |
| Excluded | SHELL-07–09; settings mute/reduced-motion and feature-audit work; Stage 5 reducer/storage architecture; Stage 6 monitor adapter/UI; Stage 7 GitHub workflow; all later stages and optional services. No change to Codex event schema, hook behavior, file-ingest contract or approval policy. |
| Validation selection | Focused Rust geometry/settings/hotkey tests; frontend FSM/input tests; runtime-identity/resource tests to prove retained assets and product namespace remain intact; TypeScript/Vite and locked Rust test/format/Clippy/build checks; changed-document/link/fence checks. Reuse prior owner-accepted baseline and packet-1 resource hashes. Record actual display count and mark mixed-DPI movement runtime-unverified on this single-display host. |
| Ownership | Coordinator owns all packet files, validation, staging, commits, push, PR and integration. |

## Capability and architecture contract

This packet adds no Codex capability and no hook events. The global shortcut is a local shell action that emits a frontend presentation event; it never invokes Codex, resolves a request, or bypasses the approved hook flow. When an actionable request is pending, toggling the shell presents its action card; Escape and cosmetic navigation cannot dismiss or replace that card.

Display selection and edge offset are ordinary persisted settings. Existing `primary` and `cursor` values remain readable; fixed-display values use the Windows display device name, and an unavailable saved display falls back to primary without discarding the saved selection. Monitor geometry remains physical at the Win32 boundary and logical in settings/UI; the configured offset is in logical pixels and is clamped so the window remains on the selected display.

The global shortcut uses Tauri's supported Rust global-shortcut plugin API. A failed or conflicting registration leaves the previously active shortcut and persisted setting intact; startup registration failure is visible in Settings and logged without blocking app launch. [Tauri Global Shortcut](https://v2.tauri.app/plugin/global-shortcut/) documents Windows support and Rust registration APIs.

The WebView2 flags are kept identical for the island and settings webviews. Microsoft documents `msSmartScreenProtection` as the feature flag that controls SmartScreen protection and recommends removing browser flags from code before shipping; this packet removes that one protection-disabling flag while preserving the unrelated inherited WebView features and autoplay behavior. [WebView2 browser flags](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/webview-features-flags) and [WebView2 WinUI guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/platforms/winui3-windows-app-sdk) are the references.

## Acceptance evidence

- [x] Tauri transparency/topmost/focus policy, Win32 non-activation, hit-tested click-through, hidden wake strip, compact/expanded geometry and file-drop mouse routing remain intact; only `msSmartScreenProtection` is removed from matching WebView2 argument sources.
- [x] Settings offer Main display, Follow cursor, and each enumerated fixed Windows monitor. A disconnected saved monitor falls back to primary and becomes selectable again if it returns.
- [x] Logical-pixel top offset persists, remains within its documented range, scales to physical pixels once, and keeps both wake strip and expanded panel within the selected monitor.
- [x] The default global shortcut is registered through the Windows-supported Tauri plugin and routes through hidden/compact/expanded shell states. User changes persist through settings JSON; invalid or conflicting changes leave the current registration/settings intact and show an error. Startup registration failure is visible and does not prevent app launch. The OS registration/conflict path was source/build verified, not manually exercised.
- [x] Escape collapses only when no actionable request is pinned; pointer leave and auto-close are gated while text input or a drag interaction is active; focus loss restores non-activating style and resumes the close timer.
- [x] A pinned actionable card cannot be replaced by tabs, file-drop/greeting/status alerts, or auto-close. It remains reachable from compact activation through the toggle event and returns to ordinary auto-close after resolution/timeout.
- [x] Existing Coucou/Mochi visuals, asset hashes, sound playback, animation durations and card transitions are unchanged.
- [x] Unit/contract/build checks pass. The host reports one 1536×960 display. Heterogeneous mixed-DPI movement and monitor hotplug were not runtime-tested; no claim is made from single-monitor evidence.

## Implementation and validation record

Implementation and local validation — 2 October 2026:

- Fixed-display selection uses each Windows display name. The monitor poll now tracks the selected target, primary identity, and full display topology so cursor-follow moves and connect/disconnect/resolution changes refresh placement and the Settings list. A missing saved display falls back to primary while preserving its preference.
- Placement settings gained migration-safe defaults and JSON round-trip coverage. Offset is clamped to 0–120 logical pixels, converted once at the monitor scale, and further clamped to keep the panel/wake strip inside small displays. Four Rust geometry tests cover scale, negative origins, short/small monitors and offset sanitization.
- The global shortcut uses `tauri-plugin-global-shortcut` 2.4.0. It registers the new accelerator before releasing the old one, rolls back if saving fails, and reports startup/conflict failures. The Windows default syntax is tested; the frontend cycles the existing FSM states, while pinned approvals force presentation of the approval card. Three additional Rust settings/shortcut tests cover defaults, migration, JSON persistence and shortcut parsing.
- Text-field focus and Explorer OLE drags suspend FSM auto-close; focus loss/drop/cancel resumes it. Escape bubbles from the prompt and remains blocked for a pending approval. View changes and chat errors cannot replace that approval. The existing Win32 style, transparent hit testing, wake-strip, file-drop and animation/audio implementation remains in place. Only the SmartScreen-disable flag was removed from both WebView2 argument sources.
- No styles, artwork, renderer, sound implementation, animation timing or asset bytes changed. Runtime-identity tests pass; the resource-set test verified all 28 inherited WAV hashes.

Local checks passed on Rust/Cargo 1.98.1: `cargo check --locked -p anti-scrolling-notch --lib`; `cargo test --workspace --locked` (38 passed, 2 installed-app environment tests ignored); `cargo clippy --workspace --all-targets --locked` (one inherited warning in `src-tauri/src/hooks.rs`); and scoped Rust 2021 formatting. Frontend checks passed: `npm run test:shell` (4), `npm run test:capabilities` (4), `npm run test:identity`, `npm run test:resources` (2), TypeScript, Vite, and `npm run pack`. The release pack rebuilt the hook and app and produced a local 4.07 MiB NSIS installer; it was not installed or launched. The existing sanitized Codex hook transport replay passed all 17 recorded assertions. Local Node/npm were 26.4.0/11.17.0; the PR CI remains responsible for the repository-pinned Node/npm 22.23.2/10.9.8 check.

Runtime limits: the host exposes one 1536×960 display, so heterogeneous mixed-DPI movement and monitor hotplug remain runtime-unverified. The app was not launched for this packet; actual Windows global-hotkey registration/conflict, focus transitions and Explorer drop/cancel were verified from the implementation and focused tests/build, not an interactive runtime session. No user settings, startup value, hook configuration or credentials were changed.

The installed `ponytail:ponytail` skill was read and applied; no separate `@Ponytail` app tool is available in this session. Existing Tauri/winit monitor/window helpers, settings persistence and FSM were reused; the only runtime dependency added is the Tauri global-shortcut plugin required by SHELL-05. Inherited UI/assets were retained unchanged.

Publication, final CI and merge evidence will be recorded after the packet PR passes its gates.

## PR and completion gate

Keep this packet limited to SHELL-04–06. Do not activate SHELL-07 or later work until this PR is validated and integrated.
