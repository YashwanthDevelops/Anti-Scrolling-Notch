# Stage 6 packet 1 — Codex CLI hook installer

## Packet identity and scope

| Field | Required value |
|---|---|
| Packet | Stage 6, packet 1: Codex CLI hook installer |
| Release profile | Monitor MVP foundation |
| Task IDs | MON-02 only |
| Branch / PR | `work/stage-6-mon-02-codex-hook-installer` / [#32](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/pull/32) |
| Dependencies | Integrated Stage 3 four-event CLI hook contract; integrated Stage 4 isolated relay identity; clean `origin/main` at `9827417d20c395e95c0097ea96ef747ce7e150dd` |
| User outcome | A user can review, install, and remove Anti-Scrolling-Notch's Codex CLI observing hooks without overwriting unrelated Codex configuration or another hook. |
| In scope | User-level `hooks.json` preview/merge/backup/fingerprint-checked atomic write/removal; exact ownership of the existing `--codex-observer` command; hook installer settings controls; tests and evidence documentation. Install only the four accepted events from the pinned CLI `0.157.1` contract. |
| UI/assets contract | Keep the retained Coucou/Mochi visuals, sounds, motion, layout and island state behavior unchanged. Use the existing settings presentation patterns for the preview and explicit apply step. |
| Excluded | New hook events or payload fields; session/turn/tool/agent identity; production monitor state or capability enablement; onboarding/test-session readiness; approval/action hooks; App Server/Desktop; Stage 5 completion; GitHub; Stage 7+; SHELL-08/09; any change to normal Codex configuration during tests. |
| Validation selection | Focused installer tests using temporary directories; hook-contract and relay replay tests; relevant Rust workspace tests/Clippy/formatting; frontend settings tests/build if UI changes; changed-doc and diff checks; final-head hosted Windows CI and Build. |
| Ownership | Coordinator owns implementation, validation, commit and publication; no agents assigned. |

## Capability and architecture contract

The only installable events are the four names captured at the Codex CLI `0.157.1` boundary and accepted by `codex_hook_contract`: `SessionStart`, `UserPromptSubmit`, `Stop`, and `SessionEnd`. Each command invokes the existing Windows relay in neutral observer mode. It reads the real hook input but interprets only `hook_event_name`; it discards all other fields and returns no Codex hook response. Codex still requires the user to review and trust the configured hook. A present file or installed command is not a connected/active session and must not produce a green monitor state.

The installer writes only the user-level `hooks.json` in the active `CODEX_HOME` (default `%USERPROFILE%\\.codex`). It refuses unreadable/invalid JSON or TOML, shows a concrete diff, creates a byte-preserving backup when an existing file will change, checks the preview fingerprint and exact backup destination before applying, writes via same-directory replacement, and removes only its exact command handler while retaining foreign handlers and group metadata. If the same user config layer declares inline hook handlers in `config.toml`, the installer reports a conflict and does not add a second representation. It uses the TOML parser already present in the workspace lockfile; no package download is required. Tests use injected temporary paths; they do not read or write the owner's regular Codex home or credentials.

## Acceptance evidence

| Dimension | Required evidence |
|---|---|
| Real behavior | Installer preview has the four exact event handlers and the production relay command; apply/reload/uninstall operate on temporary Codex-home fixtures. |
| Failure behavior | Invalid JSON/TOML, inline-hook conflict, stale fingerprint, missing relay, and filesystem errors fail closed without damaging the original file. |
| Persistence/recovery | Backup bytes match the pre-edit file; missing-file install clearly reports that there is no prior file to back up; unique previewed backup names are never overwritten; atomic replacement preserves unrelated settings/hooks; rerunning install does not duplicate the owned handler; removal preserves foreign handlers. |
| Automated verification | Rust unit tests cover exact ownership, minimal four-event allowlist, merge/removal, no duplicates, backup, missing-file install, backup-name collision/race, stale preview, malformed JSON/TOML, and conflict handling. Existing transport replay stays green. |
| Manual/platform checks | No normal Codex configuration is modified by packet tests. Live trust review and actual hook delivery are left to the later monitor onboarding/readiness packet. |
| Release limitations | Session/turn identity, independent sessions/agents, tool/compaction/permission events, event readiness, Desktop monitoring and monitor capability remain disabled/unverified. Stage 5 remains incomplete under the owner's sequencing exception. |

## Local implementation and validation — 5 October 2026

The Tauri settings page now has a separate Codex CLI hook section with detected CLI support, installed relay/config status, exact four-event count, previewed diff, explicit apply/removal, and the `/hooks` review/trust reminder. The installer parses TOML with the already locked workspace `toml` crate, fails closed on malformed config, and refuses an existing inline-hook representation. It reuses the established hook JSON parsing, fingerprint and diff helpers. A first install with no `hooks.json` explicitly reports that no backup exists; existing files preview a unique free backup destination, verify that exact destination at apply time, preserve the original bytes, and never overwrite a backup created by another writer.

Validation passed: the focused installer suite (10 tests); the locked offline Rust workspace suite (109 app tests, 7 relay tests, 2 runtime-identity tests and 6 hook-contract tests passed; 2 installed-app tests ignored); workspace Clippy; the workspace debug build; optimized relay build; and the 17-case real relay transport replay. Frontend capability (5), shell/motion (12), view-store (7), typed bridge (8), architecture (2), runtime identity and resource-byte (2) checks passed, along with TypeScript and Vite production builds. The bridge-error test emitted a non-fatal local WebSocket port-24678-in-use warning while all eight assertions passed. The new Codex hook module passes `rustfmt --check`; the workspace-wide check reports inherited formatting drift in unchanged files/regions, and no unrelated formatting was applied. Changed-document and diff checks pass. PR #32 is open; final-head hosted Windows CI/Build and merge remain pending.

No Codex home, credentials, or installed hooks were changed by these tests; installer tests use temporary directories. The new settings section has compile/test coverage but was not manually exercised in the Tauri window. Actual trust review and hook delivery remain for MON-03. No session/turn/tool/agent identity or monitor capability is inferred or enabled.

## PR and completion gate

The packet is complete only after focused/local checks, review of the staged diff, clean commit and push, a PR containing only MON-02 changes, final-head Windows CI and Build passing, and merge under the owner's standing authorization. Do not begin another packet until the PR is integrated.
