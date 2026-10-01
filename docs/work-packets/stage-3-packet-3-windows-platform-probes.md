# Stage 3 packet 3 — Windows platform probes

| Field | Value |
|---|---|
| Packet | Stage 3, packet 3: Windows notification, DPI and file-drop probes |
| Release profile | Monitor MVP |
| Task IDs | COMPAT-08; COMPAT-09 (Windows platform evidence slice) |
| Branch / PR | `work/stage-3-windows-probes` → Anti-Scrolling-Notch `main` |
| Dependencies | Packet 1 integrated at `553e03ef70848df672cbc0fad9efb753de62ae6c`; packet 2 integrated at `db06356d6512b6e0a22f98c73b42dbf060a3dd67` |
| Status | Probe packet complete on 1 October 2026; each capability outcome and runtime limitation is recorded below |
| User outcome | Establish which Windows notification, mixed-DPI overlay and Explorer-to-WebView2 drop routes are supported, with truthful fallbacks and no unverified control enabled. |
| In scope | Probe local app-notification delivery and activation for an unpackaged desktop app; inspect/runtime-test monitor scale and reposition behavior; exercise actual Explorer-originated file drops through WebView2/Tauri and the existing inbox path; add reproducible probe/test evidence and the Windows rows in the compatibility matrix. |
| UI/assets contract | Keep all Coucou/Mochi layout, visuals, sound and motion unchanged. No production toast, card, setting or redesign is added by this platform probe. |
| Excluded | Stage 4 identity/rebrand and settings/history UI; Stage 5 reducer/store; Stage 6 monitor UI/relay installation; Stage 9A notification policy/history; Codex App Server, approvals, GitHub integration and non-Windows extensions. |
| Validation selection | Existing Windows 11 host and installed app/runtime versions; native Windows notification/activation probe; current display and window DPI APIs; existing Rust file-ingest tests; actual Explorer drag to the running Tauri/WebView2 window in an isolated profile where feasible; affected Rust/frontend/document checks. No Codex home, model provider or credential is used. |
| Ownership | Coordinator owns the packet, probes, tests, docs, staging, commit, push and PR. |

## Capability and architecture contract

This packet determines support; it does not expose an unverified capability. Notification activation must name its Windows API/runtime, app identity and activation route, and must distinguish a visible toast from a click that actually returns to the companion. Any missing identity, runtime or target detail stays disabled with the generic Open Codex fallback from COMPAT-07. Toasts do not resolve approvals or act on stale session/PR identifiers.

For DPI, record the host monitor count/scales, process/window DPI-awareness context, effective window DPI and the island's physical/logical size/position mapping. Use an actual heterogeneous-monitor transition only when the host provides monitors with different effective DPIs; synthetic scale arithmetic must be labeled as a unit test, never physical mixed-DPI evidence.

For file drop, preserve the Tauri drag event path and `files::ingest` copy boundary. Test a disposable file from Explorer, observe the actual event sequence, verify the copied bytes/name/size, and record folder/error behavior and cleanup. Do not treat a direct command invocation or a browser-only drag simulation as proof of an Explorer-to-WebView2 drop.

The accepted Codex hook contract remains exactly `SessionStart`, `UserPromptSubmit`, `Stop`, and `SessionEnd`. These Windows probes do not change the observer schema, and no event/session/project data is inferred.

## Acceptance evidence

- Record the exact Windows build, relevant notification runtime/package and application registration. If the production notification API cannot be exercised in this unpackaged Tauri environment, document the tested bridge/runtime route and keep production toast activation disabled.
- Verify notification delivery and click activation separately. Record target/arguments, whether the app is already running, and the no-registration/activation failure fallback.
- Record live display/window DPI data and inspect the overlay's logical-to-physical geometry and reposition trigger. Run mixed-DPI runtime checks only if two actual monitors with different effective DPI are available; otherwise state that limitation explicitly and retain it for Stage 11 QA.
- Drag a uniquely named temporary file from Explorer onto the native app. Verify the Tauri `enter`/`over`/`drop` flow, successful copied file contents, and cleanup. Verify a folder or other rejected input follows the existing error path without claiming unsupported file types are accepted.
- Keep all hooks, configuration and credentials outside the normal Codex home untouched; do not include raw user data or temporary payloads in committed artifacts.
- Update `docs/codex-compatibility.md` and the execution ledger with source/runtime evidence and enabled/disabled conclusions for each capability. Do not check COMPAT-08 complete unless all three probe outcomes and their remaining limitations are explicit.

## Results

Environment: Windows 11 Home 25H2, build 26200.9457 (x64). The registry ProductName field reports “Windows 10 Home”; the existing Stage 2 baseline identifies this build as Windows 11. No product code, Codex configuration, credentials or model-provider settings were changed.

### Native toast delivery and activation

The machine has Microsoft.WindowsAppRuntime 1.7 (7000.785.2325.0) and 1.8 (8000.994.2142.0), x64 and x86. An isolated unpackaged probe was built from the already-present temporary .NET SDK 10.0.401 and cached Windows App SDK package. The requested NuGet version 1.8.260803002 was not present; restore had resolved 1.8.260804001. The temporary no-restore Release build passed after removing a cleanup API absent from the resolved package. No additional SDK or package installation was attempted during this closeout; the isolated SDK/cache had already been provisioned under %TEMP%.

The probe called AppNotificationManager.Register and Show without throwing and wrote shown=true to its temporary status file. That confirms only that the API accepted the request. The bounded desktop check found no UI Automation element matching the test notification title, the lower-right screen crop showed no visible toast, and the NotificationInvoked callback file was absent. The process was stopped after this check; no click activation was observed. Get-StartApps had no Coucou or Anti-Scrolling-Notch registration. Therefore visible delivery and click activation are runtime-unverified. Do not enable production toast activation until an app-identity/registration path and an actual delivery plus activation test are established. The probe source and status marker remain local under %TEMP%\anti-scrolling-stage3-windows-probes; no probe code or notification API was added to the app.

### Overlay DPI and monitor changes

The host reports one display, \\.\DISPLAY1, at 1920×1200. The running Coucou island HWND reports DPI 120 (125%) and awareness class 2 (Per-Monitor); its collapsed physical window rectangle was (810, 0)–(1110, 8). This agrees with the source constants STRIP_W=240 and STRIP_H=6 logical pixels multiplied by 1.25 and rounded to physical pixels. Source inspection confirms screen_info divides monitor geometry by scale_factor, apply_geometry multiplies logical window dimensions by that factor, and the active cursor poll compares monitor position, size and scale every 30 × 16 ms before emitting screen-changed. This is source-verified geometry and single-monitor runtime evidence only. Because the machine has one display, heterogeneous-DPI movement, hotplug and cross-monitor repositioning remain runtime-unverified and must be tested in Stage 11.

### Explorer-to-WebView2 drop

Source inspection confirms Tauri dragDropEnabled=true; Bridge.onDragDrop subscribes to the current WebView drag event; the island expands its upload view on enter/over and passes the first dropped path to Bridge.ingestFile on drop; the Rust ingest command copies regular files into %LOCALAPPDATA%\Coucou\inbox, avoids overwriting an existing name and rejects directories. The frontend logs enter/leave/drop but intentionally does not log every over event. A folder error is shown in the note view, plays the error sound and returns to the default view after 2.4 seconds; this frontend failure path is source-verified only. The existing Windows Rust test files::tests::ingest_copies_and_never_overwrites passed (1 passed, 11 filtered); it verifies copy bytes, collision handling, directory rejection and copy-time age behavior. This is direct-ingest test evidence, not proof of Explorer/WebView2 transport or folder-drop UI behavior.

An isolated 87-byte, uniquely named text file was displayed in a foreground Explorer CabinetWClass window. A guarded native pointer press/move/release gesture was aimed at the running island’s collapsed wake strip. No drag enter/leave/drop line was appended to the app log and no matching file appeared in the inbox. The supported desktop automation surface returned “failed to write kernel assets: The system cannot find the path specified. (os error 3)”; a PowerShell UI Automation search also did not expose the Explorer file row. No retry or helper repair was attempted. The Explorer-to-WebView2 enter/over/drop path is therefore runtime-unverified by this packet. The failed automation gesture alone is not sufficient to diagnose a product defect. The owner separately reported successful general file-drop interaction for BASE-05; that owner-reported check is not an independently observed COMPAT-08 Explorer trace. The disposable source file and screen crops were cleaned; the inbox had no matching test copy.

### Disposition

COMPAT-08 is complete as a bounded compatibility spike because the native-toast, display-scale and Explorer-drop outcomes and limitations are explicit. This does not enable toast activation, prove mixed-DPI support or verify Explorer-to-WebView2 delivery. Preserve the inherited file-drop behavior unchanged; the independent route remains a Stage 11 verification item. The accepted Codex observer contract remains exactly SessionStart, UserPromptSubmit, Stop and SessionEnd. This packet changes documentation only and does not implement monitoring, a reducer, UI state or notifications.
