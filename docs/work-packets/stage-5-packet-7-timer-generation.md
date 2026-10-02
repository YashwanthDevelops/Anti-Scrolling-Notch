# Stage 5 packet 7 — task activity generations for delayed completion timers

| Field | Value |
|---|---|
| Packet | Stage 5, packet 7: CORE-06 timer scoping |
| Task ID | CORE-06 only |
| Branch / PR | `work/stage-5-core-06-timer-generation` / PR pending |
| Dependencies | Stage 5 CORE-01–04 and packet 6 integrated; PR #20 merge `2bf681458cc23009940819806dc2b23d7c004d3f` |
| User outcome | A delayed completion/badge cleanup may update a task only if that same task is still in the activity generation and status for which the timer was scheduled. |
| Status | Implementation and required local validation passed; publication pending |

## In scope

- Add activity-generation tokens to frontend task state, allocated from a monotonically increasing app-local counter so task removal/re-addition cannot reuse a stale token. Advance the token when activity changes task state or task steps.
- Add a conditional task transition that checks the expected generation and allowed current status before changing status.
- Capture the current generation for the 5.2-second hook `Stop` reset and the 60-second integration success/error cleanup. Ignore stale callbacks; retain integration timer replacement by task ID as a resource optimization.
- Test two completion events for the same task, newer work arriving before expiry, current-generation expiry, and integration success/error expected-state handling.

## Existing behavior retained

- No duration, sound, animation, badge design, or island presentation changes.
- The approval timeout remains request-scoped and replaceable; island FSM timers remain guarded by their current state/pin/interaction checks; Mochi animation tokens and sound idle timers remain unchanged.
- The generation is a local UI invalidation token, not a Codex source event ID, session ID, or turn ID. The verified hook observer remains anonymous; no upstream identity is inferred.

## Excluded

- Stage 6 production monitor or any new Codex event schema.
- Changes to request routing/resolution, approval timeout behavior, unrelated animation/FSM/settings timers, or the Stage 5 view-store/backend architecture.
- Reworking inherited UI, animations, sound behavior, assets, or timer durations.

## Implementation and local validation

`AppState` now allocates a unique monotonically increasing app-local activity token for task state/step changes and for each newly created integration task. Removing and re-adding the same integration ID therefore cannot reuse the token held by an older timer. The 5.2-second hook `Stop` reset and 60-second integration-result cleanup call shared, testable cleanup helpers that atomically check the captured token/state and update state, steps, and badges. Existing timer durations, cancellation/replacement, and UI presentation are retained.

The frontend shell suite passed 12/12, including tests through the actual Stop and integration cleanup helpers for stale work, later completion, current expiry, error expiry, and task removal/re-addition. The TypeScript check and Vite production build passed. The locked Rust workspace passed 82 tests with 2 installed-app tests ignored; Clippy passed with the existing `src-tauri/src/hooks.rs:467` warning. The changed-document checker validated four Markdown files and `git diff --check` passed. No Rust files, UI assets, animation code, or sound code changed. No app/WebView2 timer measurement was performed; configured source delays remain 5.2 seconds and 60 seconds.

## Acceptance checks

- Unit tests exercise the same cleanup helpers used by both timers and prove that an older token cannot reset a later completion, active work, or a removed/re-added integration, while the current matching generation can expire only from an allowed terminal status.
- Existing integration timer cancellation remains in place, with its callback also checking the generation before atomic state/step/badge cleanup.
- Locked workspace tests, Clippy, scoped Rust formatting if Rust files change, frontend shell tests, TypeScript/Vite build, changed-document validation, and `git diff --check` pass.
- No claim of source-level Codex session/turn scoping is made; CORE-06's local UI timers are protected without inventing IDs.
