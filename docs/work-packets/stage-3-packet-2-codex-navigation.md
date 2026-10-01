# Stage 3 packet 2 — Codex navigation fallback

| Field | Value |
|---|---|
| Packet | Stage 3, packet 2: verified best-effort Codex app navigation |
| Release profile | Monitor MVP |
| Task IDs | COMPAT-07 (Codex app activation route; project route assessed), COMPAT-09 (navigation evidence matrix slice) |
| Branch / PR | `work/stage-3-codex-navigation` → Anti-Scrolling-Notch `main` |
| Dependencies | Stage 3 packet 1 integrated at `553e03ef70848df672cbc0fad9efb753de62ae6c`; accepted CLI hook subset remains four observed events |
| Status | Implementation and selected checks complete |
| User outcome | A future monitor card can open the registered stable Codex desktop app at its generic landing surface without claiming an unsupported chat/session deep link. |
| In scope | Verify the installed Windows Codex app identity; add a backend-only native activation command and typed bridge; test success and absent-app failure; document the existing explicit-path project-opening fallback and exact limits. |
| UI/assets contract | No layout, Coucou/Mochi, sound, motion, or state-machine changes. No new monitor card is added before the Stage 6 workflow. |
| Excluded | Codex conversation/session deep links; inferring project paths or session identity from unverified hook fields; COMPAT-08 notifications/DPI/drop probes; App Server; approval routing; Stage 4 shell identity and Stage 6 monitor UI. |
| Validation selection | Windows 11 stable Codex package/AUMID discovery, native activation API smoke, Tauri backend unit tests for success/failure contract, Rust workspace checks and affected frontend type/build checks. No model turn or provider request is issued; the navigation adapter does not read or write Codex settings or credentials. |
| Ownership | Coordinator owns implementation, tests, documentation, staging, commit, push, and PR. |

## Capability and architecture contract

The only verified Codex desktop destination in this packet is its generic app launch surface. The app is discovered as the stable `OpenAI.Codex` packaged app; launch uses its registered AppUserModelID and Windows' `IApplicationActivationManager::ActivateApplication`. No command arguments, URLs, conversation IDs, session IDs, or project paths are passed. The Tauri command returns an activation PID or an HRESULT error; it does not update monitor state or decide an approval.

The existing `open_in_vscode(path)` command remains the best-effort project-folder route: it passes a caller-supplied path as one process argument to VS Code when available, then attempts Explorer. The verified Codex hook adapter currently carries no project path or session identity, so the monitor must not bind that project action to a Codex session yet. Exact chat/terminal navigation remains unsupported; the verified fallback is to open Codex generally, or open a project only when a separately verified path is available.

The bridge method is only an OS navigation request. Later Stage 6 UI can offer it as a plainly named fallback; until then no Codex monitor control is exposed. If the app is absent or Windows activation fails, the command returns an HRESULT-bearing error and the observer lifecycle remains neutral. Activation is synchronous and this packet does not claim a bounded timeout.

## Acceptance evidence

- Record the Windows build, installed stable package version, display name, and registered AUMID.
- Exercise the native activation API against the installed stable package and verify that the active foreground application belongs to that package.
- Verify that an unknown AUMID returns an HRESULT error; the activation call uses `AO_NOERRORUI`, and the command does not mutate or enter observer event processing.
- Preserve the existing source-backed Open Project fallback, but keep session-specific project navigation disabled until a project path is present in an accepted Codex contract.
- State that no exact conversation/session navigation, Codex Terminal, or Codex CLI session identity is verified.

## Task status

| Task slice | Status | Evidence / remaining work |
|---|---|---|
| COMPAT-07 — generic Codex app destination | Complete | Windows 11 build 26200 / stable app 26.928.2636.0 activation test passes. The generic home destination is not a conversation link. |
| COMPAT-07 — project fallback | Limited | Existing explicit-path VS Code/Explorer behavior is source-verified. The four-event Codex adapter has no verified project path, so Codex-session project controls stay unavailable. |
| COMPAT-09 — navigation evidence | Complete for this slice | `docs/codex-compatibility.md` records package/AUMID, transport, error behavior, fallback and scope. |

## Packet completion gate

Close this packet only after the native activation command and its missing-app failure path pass their selected checks, the compatibility matrix records the exact fallback boundary, and all commits are pushed. This packet does not complete the remaining Stage 3 probes or authorize Stage 4.
