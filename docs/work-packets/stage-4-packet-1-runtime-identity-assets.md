# Stage 4 packet 1 — runtime identity and replaceable assets

## Packet identity and scope

| Field | Value |
|---|---|
| Packet | Stage 4, packet 1: runtime identity and replaceable assets |
| Release profile | Monitor MVP foundation |
| Task IDs | SHELL-01, SHELL-02, SHELL-03 |
| Branch / PR | `work/stage-4-runtime-identity` / one PR against Anti-Scrolling-Notch `main` |
| Dependencies | PR #10 merged; `origin/main` at `7e96c2c55aaaaa0c2c287610b4204a91a504f4ba`; Stage 2 accepted and Stage 3 selected CLI hook contract integrated |
| User outcome | A new Anti-Scrolling-Notch install owns a distinct Windows/runtime identity while displaying the retained Coucou/Mochi prototype unchanged. |
| In scope | Set product, app/package, credential, settings/local-data, relay, named-pipe, log, tray and autostart identities; preserve old Coucou data/configuration; make hook ownership exact; keep current inherited asset files byte-identical behind a replaceable manifest/reference; update build, test and installer paths. |
| UI/assets contract | Keep existing Coucou layout, Mochi renderer/expressions/animations, icons, sounds, timing and interaction behavior. No asset regeneration or UI redesign. Assets remain inherited development resources; distribution requires REL-11 clearance. |
| Excluded | Window/focus/DPI changes (SHELL-04–06), new Codex monitor/reducer/UI, Stage 5 persistence/migration, interactive App Server, optional services, release publication, asset replacement/rights determination. No automatic migration, reading, copying or deletion of legacy Coucou settings, secrets, startup values, relay files or hook entries. |
| Validation selection | Locked workspace tests, Clippy, frontend type/build and capability tests, resource manifest/hash parity, exact runtime-identity consistency, existing Codex relay replay, installer build, Markdown/link/whitespace checks, and hosted Windows CI. Verify the sound source files have no diff. Do not toggle or write the user's startup setting or Claude/Codex configuration. |
| Ownership | Coordinator owns the packet, shared checkout, staging, commits and push. |

## Capability and architecture contract

This packet adds no Codex event or interaction capability. The accepted four-event Codex CLI observer boundary and its neutral fallback stay unchanged. The helper executable, backend listener, configuration labels and frontend continue to use the already verified contract; no schema is added.

The app will use a product-specific Tauri/Cargo identity, `%APPDATA%\Anti-Scrolling-Notch`, `%LOCALAPPDATA%\Anti-Scrolling-Notch`, a new Windows Credential Manager service namespace, a new HKCU Run value name, an app-specific relay executable, and separate app/Codex pipe prefixes. Existing `%APPDATA%\Coucou`, `%LOCALAPPDATA%\Coucou`, Coucou credential entries, legacy startup entry, and old Coucou hook commands are outside the new app's ownership. No migration is implemented here.

The release build may compile/package the inherited assets for local validation, but CI must not publish a downloadable installer or create a public release until REL-11 is explicitly cleared. Build identity tests and resource-manifest tests are deterministic; they do not claim that the user's actual startup preference was toggled or that an asset license was granted.

## Acceptance evidence

- [ ] Product/runtime IDs agree across Cargo, Tauri, npm, helper, Windows paths, logs, pipes, settings, installer cleanup, tray, and autostart registration.
- [ ] The app starts with startup disabled by default; the supported autostart adapter registers only the new explicit app name. The current user's Run key is not modified during validation.
- [ ] Settings/secrets/local paths are fresh product-specific locations. Existing Coucou data and Credential Manager entries are left untouched; no implicit migration occurs.
- [ ] Hook install/remove recognizes only the exact new relay executable path. Tests prove existing Coucou and unrelated hooks are retained.
- [ ] The development resource manifest points to current Mochi, icon and sound resources. The source sound set and production build copies match byte-for-byte; no visual/animation/sound source asset changes are included.
- [ ] Installer builds with product-specific naming and new owned cleanup paths. Release publication is blocked until REL-11 clearance.
- [ ] Locked tests, CI, build/type checks, existing verified Codex replay, documentation checks and `git diff --check` pass.
- [ ] Ledger records unsupported/unverified runtime capabilities and exact commit/PR/check evidence; later Stage 4 tasks remain pending.

## PR and completion gate

Review the staged diff for accidental edits to asset bytes, user data, hooks, credentials, startup state, generated artifacts or later-stage code. Commit and push each completed coherent change. The packet stops after its PR is validated and published; no later packet starts until this PR is reviewed and integrated.
