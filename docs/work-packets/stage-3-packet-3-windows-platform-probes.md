# Stage 3 packet 3 — Windows platform probes

| Field | Value |
|---|---|
| Packet | Stage 3, packet 3: Windows notification, DPI and file-drop probes |
| Release profile | Monitor MVP |
| Task IDs | COMPAT-08; COMPAT-09 (Windows platform evidence slice) |
| Branch / PR | `work/stage-3-windows-probes` → Anti-Scrolling-Notch `main` |
| Dependencies | Packet 1 integrated at `553e03ef70848df672cbc0fad9efb753de62ae6c`; packet 2 integrated at `db06356d6512b6e0a22f98c73b42dbf060a3dd67` |
| Status | Active; probes and final evidence pending |
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

Fill this section with exact commands, versions, artifacts, observed outcomes, failures and cleanup before closing the packet. Separate source findings, automated tests, owner-reported evidence and independently observed runtime behavior.
