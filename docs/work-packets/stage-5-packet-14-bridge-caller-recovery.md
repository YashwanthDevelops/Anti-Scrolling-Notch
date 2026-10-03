# Stage 5 packet 14 — typed bridge caller recovery

## Packet identity and scope

| Field | Required value |
|---|---|
| Packet | Stage 5, packet 14: CORE-10 typed bridge caller recovery |
| Release profile | Monitor MVP state-authority foundation |
| Task IDs | CORE-10 — existing caller-side bridge failure slice |
| Branch / PR | `work/stage-5-core-10-caller-recovery` / PR #29 (merged) |
| Dependencies | PR #26 snapshot/replay view store, PR #27 typed bridge failures, PR #28 integration-health projection; base `230b5ba98ebb866a240ec0093804da7c33ce4fae` |
| User outcome | Native command failures are surfaced or safely contained at existing callers, and local state is not cleared or changed as if a rejected user action succeeded |
| In scope | Existing Windows frontend bridge callers in app startup/tray actions, island/views/integration cards, hook acknowledgements/declines, integration credential-status refresh and settings; focused tests and plan/ledger documentation |
| UI/assets contract | Reuse the current note and settings notice styles. Preserve Coucou/Mochi layout, assets, sounds, animation behavior, and timings. |
| Excluded | New bridge/Rust event or command schemas; new Codex producers; task/session projection; Stage 5 reducer/router changes; Stage 6 monitor; request-resolution claims beyond existing command semantics; SHELL-08/09; UI redesign or capability enablement |
| Validation selection | Focused typed-bridge, view-store, shell, capability, identity, and resource tests; TypeScript check and Vite build; docs consistency and diff check; hosted Windows CI and Build. Rust sources/dependencies are unchanged, so reuse the passing packet 13 Rust tests/Clippy/build unless scope expands. |
| Ownership | Coordinator owns implementation, validation, publication, and integration |

## Capability and architecture contract

No capability is enabled by this packet. Native command calls retain the packet 12 `BridgeCallError(command, kind)` contract; no raw backend payload is exposed. Existing user actions report a safe fallback through the current UI. Best-effort internal operations contain rejections and log only fixed caller context plus typed command/failure kind. A rejected integration secret-status read leaves the last known presentation value unchanged. Hook approval delivery remains the inherited Claude relay path; this packet does not reinterpret its command completion as a verified Codex request-resolution event.

## Acceptance evidence

| Dimension | Required evidence |
|---|---|
| Real behavior | Existing native commands continue through their current typed bridge; callers recover using existing UI/status state |
| Failure behavior | Tests prove typed errors remain sanitized, safe best-effort handling resolves without unhandled rejection, and UI action fallback is invoked; failed startup/settings reads remain visibly unavailable rather than success-looking |
| Persistence/recovery | Existing saved settings and broker snapshot/replay behavior are unchanged; no new persistence schema |
| Automated verification | Relevant frontend tests, TypeScript check, Vite build, documentation check, diff review, and hosted Windows CI/Build pass |
| Manual/platform checks | No new platform-specific behavior; runtime UI remains covered by existing project-owner verification |
| Release limitations | Legacy task/session state still lacks a verified producer; Stage 6 remains pending. Existing optional integrations and request controls retain their current capability gates. |

## PR and completion gate

Implement and validate this caller-only slice, inspect the final diff for unchanged command/event contracts and retained UI assets/behavior, then commit, push, open/update one PR, verify final-head CI and review state, and merge under the standing authorization only after all repository gates pass.
