# Stage 5 packet 9 — request lifecycle cleanup

| Field | Required value |
|---|---|
| Packet | Stage 5, packet 9: CORE-08 terminal request cleanup |
| Release profile | Monitor MVP request-state foundation |
| Task IDs | CORE-08 cleanup sub-scope only |
| Branch / PR | `work/stage-5-core-08-request-cleanup` / [PR #24](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/pull/24) |
| Dependencies | Integrated CORE-07 process-local request-routing contract; no verified Codex request producer or reply adapter |
| User outcome | An authoritative resolved, expired or cancelled request stops appearing in active backend request state and frees its exact in-memory route |
| Status | Integrated on main at `6efabcace80b8137acb759f2e9e5585f0d8c57f0` |

## In scope

- Remove a request from reducer active state and every request relationship index when an authoritative `PendingRequest` update has lifecycle `Resolved`, `Expired` or `Cancelled`.
- Treat the source update as authoritative regardless of whether the user resolved it in this companion or in another client. Do not wait for a local submit action.
- Add an exact-identity reconciliation method to the CORE-07 router so matching terminal updates release tracked records and capacity. Reject conflicting source/scope reuse without clearing a different route; stale in-flight completions must be rejected after cleanup.
- Preserve deterministic sequence behavior: when a terminal update removes an active request, the existing stream records the state-changing terminal update for replay, and snapshots omit the removed request.
- Preserve deadlines and other correlation metadata; do not infer missing source IDs or convert a timeout into a source resolution.

## Architecture and UI/resource contract

This is backend state cleanup only. No Tauri command, UI, timer, hook installation, request producer, response transport, frontend authority or animation change is added. Existing Coucou/Mochi visuals, sounds, motion and asset bytes remain untouched. `codex.permissionDecision` and `codex.managedSession` stay disabled.

## Explicit limitations

- Only a source lifecycle update that explicitly reports `Resolved`, `Expired` or `Cancelled` can clear a route. The accepted observer sends only four anonymous event names and has no request event. No source-side timeout, request expiry, or response semantics are inferred.
- The plan's prompt fallback when a request cannot be presented remains disabled and runtime-unverified. There is no verified Codex request hook, presentation-failure response, or fallback value to send. Do not implement Claude fallback behavior for Codex.
- This packet does not address CORE-07's external exactly-once limitation, CORE-09 pipe work, CORE-10 view-store consumption, CORE-11 architecture tests, or Stage 6 production monitoring. CORE-08 remains unchecked until the fallback requirement has a verified source contract and real integration.

## Acceptance evidence

- Tests cover all three terminal lifecycles, another-client/source-originated resolution, complete removal from every affected reducer index, replay sequencing, exact router reconciliation, capacity reclamation, and stale late completions.
- Conflicting or incomplete terminal records fail closed and do not clear a different request.
- The existing production registry continues to disable Codex request and managed-session capabilities.
- Run focused reducer/router/stream tests, the locked Rust workspace suite and Clippy, scoped formatting of changed Rust modules, Markdown validation, and `git diff --check`. No UI/resource check is needed because none are modified.
- Record the unavailable presentation-fallback requirement as runtime-unverified; do not claim CORE-08 fully complete.

## Local implementation and validation

Reducer terminal upserts now remove an active request only when all request fields other than lifecycle match; mismatched source/request metadata leaves the active record and every index intact. Router reconciliation applies the same exact signature rule and releases a route on a source `Resolved`, `Expired` or `Cancelled` update, including after its deadline. Any late in-flight completion is rejected as unknown, and a cleared slot can be reused. The existing stream sequences a terminal update for replay while its current snapshot omits the removed request.

Validation passed: focused broker tests (43 passed); `cargo test --workspace --locked` (96 passed, 2 installed-app tests ignored); `cargo clippy --workspace --all-targets --locked`; rustfmt check for `broker/request_router.rs`; `docs/check-markdown.ps1`; and `git diff --check`. Clippy reports only the existing `src-tauri/src/hooks.rs:467` `needless_range_loop` warning. Whole-file rustfmt checks for `reducer.rs` and `stream.rs` still report formatting differences in unchanged legacy tests; the newly added/changed hunks produce no formatter diff. No frontend/resource checks were needed. PR #24 passed Windows CI run 58 and Build run 67 on final head 5761ed533b047eab7614bcb445888ca330a8c769 and was merged at 6efabcace80b8137acb759f2e9e5585f0d8c57f0. CORE-08 remains unchecked because the accepted Codex contract still supplies no real request event or presentation-fallback response semantics.
