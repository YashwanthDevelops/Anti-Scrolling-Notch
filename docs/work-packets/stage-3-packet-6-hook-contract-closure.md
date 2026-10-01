# Stage 3 packet 6 — installed-release hook contract closure

| Field | Value |
|---|---|
| Packet | Stage 3, packet 6: installed CLI hook contract and neutral replay closure |
| Release profile | Monitor MVP compatibility boundary; no production monitor is enabled |
| Task IDs | COMPAT-01, COMPAT-02, COMPAT-03 |
| Branch / PR | `work/stage-3-hook-contract-closure` → Anti-Scrolling-Notch `main` |
| Dependencies | PR #9 merged into `origin/main` at `8bafb6c1cb00ead36408967939a49f1177836efe`; packet 1's accepted four-event loopback evidence and fixture |
| User outcome | Keep the accepted four-event CLI observer contract version-pinned, generated from the matching release schema, and demonstrably neutral when the app is unavailable. |
| Status | Implementation and local packet checks complete; hosted CI and PR review are the remaining packet checks. |

## Scope

This packet closes compatibility only for Codex CLI command hooks version `0.157.1`, the only declared supported hook surface. It records the complete hook schema filename inventory from the matching OpenAI source tag, vendors and checksum-pins the four event input schemas already established by runtime evidence, generates the adapter discriminator type from those schemas, and requires it to match the accepted sanitized fixture. It extends the Windows transport replay to cover neutral rejection of every known unverified hook event and the no-app fallback for all four captured events. Windows CI runs that replay.

The installed `OpenAI.Codex` Desktop package version `26.928.2636.0` remains recorded but is not a supported monitoring surface because no Desktop hook/session protocol or event fixture exists. The pinned CLI release has generated schemas for 12 hook inputs and 11 hook outputs; schema availability is not runtime-delivery evidence. Only `SessionStart`, `UserPromptSubmit`, `Stop`, and `SessionEnd` are runtime-observed and accepted. `PreToolUse`, `PostToolUse`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, `Interrupt`, and `PermissionRequest` remain runtime-unverified, rejected before transport, and disabled.

## Explicit exclusions

No new hook event is enabled or claimed as runtime-observed. This packet does not rerun the loopback provider probe, investigate Ollama, use computer-use tooling, install hooks into a Codex home, inspect or change normal Codex configuration/credentials, implement a production monitor, add session state/UI, implement the Stage 5 reducer, or begin Stage 4 or any later stage. App Server protocol schemas remain a separate COMPAT-05 v1 concern.

## Implementation and acceptance evidence

- The schema manifest pins `codex-cli 0.157.1` to OpenAI Codex source tag `rust-v0.157.1`, commit `36650394c5b38c2990ccf2a3457165ca3e9d9726`, and records all 12 input and 11 output schema filenames available there. The four adapter input schemas are checked in with Apache-2.0 attribution and fixed checksums.
- Cargo's build script verifies each vendored schema checksum and generates the strict `CodexHookEvent` enum from its `hook_event_name.const`. A contract test verifies the generated event order matches the four rows in the accepted sanitized capture and that the generation source is the pinned release revision.
- The Windows replay sends each accepted projection through the hook process and isolated named-pipe receiver, verifies the exact two-field envelope and neutral stdout/stderr/exit, rejects all eight unverified event names plus malformed JSON without transport, and checks all four observed events return neutrally within 1800 ms when no app pipe exists.
- The absent-app test timing is a local hook-process replay measurement, not a new in-Codex runtime measurement. No runtime delivery claim is made for unobserved events or Desktop.
- Windows CI runs the replay after building the hook binary; portable Rust tests also run the schema generator and checksum validation.

Local checks passed: `cargo fmt -p codex-hook-contract -- --check`; `cargo test --workspace --locked` (26 passed, 2 ignored); `cargo clippy --workspace --all-targets --locked` (passed with the same three inherited warnings and no packet warnings); `cargo build --release -p coucou-hook --locked`; the expanded PowerShell transport replay; four frontend capability tests; TypeScript checking; Vite production build; changed-document link/fence checks; and `git diff --check`. Across three local runs, the absent-app replay measured 13–17 ms per event. Hosted CI is the final packet check.

## Completion boundary

COMPAT-01–03 are complete for the selected four-event Codex CLI compatibility contract. Unobserved event categories and Desktop monitoring remain explicitly unsupported and disabled. This closes the compatibility evidence boundary only; the hook adapter is still not a production monitor, and Stage 5/6 state and UI work remains pending. Stop after this packet is validated, committed, pushed, and its PR checks pass.
