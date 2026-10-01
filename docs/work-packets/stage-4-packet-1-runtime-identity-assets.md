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

The app will use a product-specific Tauri/Cargo identity, `%APPDATA%\Anti-Scrolling-Notch`, `%LOCALAPPDATA%\Anti-Scrolling-Notch`, a new Windows Credential Manager service namespace, and the explicit HKCU `Software\Microsoft\Windows\CurrentVersion\Run` value name `Anti-Scrolling-Notch`, plus an app-specific relay executable and separate app/Codex pipe prefixes. Existing `%APPDATA%\Coucou`, `%LOCALAPPDATA%\Coucou`, Coucou credential entries, legacy startup entry, and old Coucou hook commands are outside the new app's ownership. No migration is implemented here.

The release build may compile/package the inherited assets for local validation, but CI must not publish a downloadable installer or create a public release until REL-11 is explicitly cleared. Build identity tests and resource-manifest tests are deterministic; they do not claim that the user's actual startup preference was toggled or that an asset license was granted.

## Acceptance evidence

- [x] Product/runtime IDs agree across Cargo, Tauri, npm, helper, Windows paths, logs, pipes, settings, installer cleanup, tray, and autostart registration.
- [x] Startup is disabled by the default settings value, and the supported autostart adapter registers only the explicit new app name. The current user's Run key was not modified during validation.
- [x] Settings/secrets/local paths are fresh product-specific locations. Existing Coucou data and Credential Manager entries are left untouched; no implicit migration occurs.
- [x] Hook install/remove recognizes only the exact new relay executable path. Tests prove existing Coucou and unrelated hooks are retained.
- [x] The development resource manifest points to current Mochi, icon and sound resources. The source sound set and production build copies match byte-for-byte; no visual/animation/sound source asset changes are included.
- [x] The local installer builds with product-specific naming and new owned cleanup paths. Release publication is blocked until REL-11 clearance.
- [x] Locked tests, CI, build/type checks, existing verified Codex replay, documentation checks and `git diff --check` pass.
- [x] Ledger records unsupported/unverified runtime capabilities and exact commit/PR/check evidence; later Stage 4 tasks remain pending.

## Local implementation and validation — 1 October 2026

The runtime identity manifest now defines `Anti-Scrolling-Notch`, `com.yashwanthdevelops.antiscrollingnotch`, the app/frontend/relay package names, fresh `%APPDATA%` and `%LOCALAPPDATA%` directory names, Credential Manager service, explicit startup value name, separate app/Codex pipe prefixes, and diagnostic log name. A shared Rust crate generates the constants consumed by the app and relay; tests check the manifest against Cargo, Tauri, npm, paths, pipes, autostart and installer configuration. Hook ownership compares the normalized full executable path, so the inherited Coucou command and similarly named binaries remain foreign. Settings still default startup off; validation did not modify the user's Run key, Codex/Claude settings, credentials, or old Coucou paths.

`windows/resources.json` marks the inherited Mochi renderer, greeting, inline/application icons and 28 WAV sounds as replaceable development assets. The sound player reads its public path from that manifest. The resource test checks all 28 production-bundle WAV files against their inherited source SHA-256 values. No Mochi renderer, island state/motion, layout/style, icon or sound source file changed; no asset-rights conclusion is made. The changed UI text and app/window/tray labels only reflect the new product identity.

On the installed Rust/Cargo 1.98.1 MSVC toolchain, scoped Rust 2021 formatting checks passed for edited Rust logic, `cargo test --workspace --locked` passed 30 tests with 2 installed-app environment tests ignored, and `cargo clippy --workspace --all-targets --locked` passed with the same three inherited warnings recorded in the baseline. `npm run test:capabilities`, `npm run test:identity`, `npm run test:resources`, and the release frontend type-check/build passed. The Windows hook replay passed all four captured lifecycle events, eight unverified/malformed neutral fallbacks, and four no-app fallbacks. `npm run pack` rebuilt the relay from the current source and produced the local 4.04 MiB NSIS installer `Anti-Scrolling-Notch-Windows-0.1.1-setup.exe`; generated installers remain ignored local artifacts and were not installed or launched.

The current user's startup registration and settings were not exercised at runtime. The installed UI was not relaunched for this packet; source and build evidence establish the identity changes while the earlier project-owner BASE-05 acceptance remains separate. Local changed-document/link/fence and whitespace checks passed.

## Published validation — 1 October 2026

Implementation commit `c7dc5fb09f39cce2f8b33f71688bae5ae023ddf7` is pushed to `origin/work/stage-4-runtime-identity`, and the remote branch SHA matched. [PR #11](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/pull/11) is open against `main` and remains unmerged at this packet boundary. [Windows CI run 28](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/actions/runs/36889500973) passed scope, frontend, documentation and Rust workspace jobs; inherited [Build run 37](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/actions/runs/36889500988) also passed. SHELL-01–03 are complete on the packet branch; SHELL-04 onward and all later stages remain pending.

## PR and completion gate

Review the staged diff for accidental edits to asset bytes, user data, hooks, credentials, startup state, generated artifacts or later-stage code. Commit and push each completed coherent change. The packet stops after its PR is validated and published; no later packet starts until this PR is reviewed and integrated.
