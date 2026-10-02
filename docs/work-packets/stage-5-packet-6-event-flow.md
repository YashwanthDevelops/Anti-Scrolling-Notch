# Stage 5 packet 6 — source identity deduplication and ticker sequences

| Field | Value |
|---|---|
| Packet | Stage 5, packet 6: supported CORE-05 event-flow foundation |
| Task ID | CORE-05 (partial; the text-delta requirement remains open) |
| Branch / PR | `work/stage-5-core-05-event-flow` / PR pending |
| Dependencies | Integrated CORE-01–04; PR #19 merge `916f6c4c1dfe599044e0c7667dc1eec8182ed665` |
| User outcome | Replayed normalized source events are ignored when their exact optional ID is still in the bounded window, and the inherited ticker continues to recognize new steps after its visible history reaches its cap. |
| Status | Implementation and local validation complete; publication, CI, review and integration pending |

## In scope

- Add bounded deduplication to the existing normalized broker stream for `source_event_id` and `deduplication_key` values already supplied by an adapter. Keys are scoped by the update's existing `SourceKind` and by identifier kind. The cache is bounded to the replay-window event count; missing IDs never trigger payload hashing or guessed identity.
- Preserve accepted optional IDs in replay envelopes. Return an explicit receipt bit when a duplicate is ignored, without applying its payload or consuming a sequence.
- Give each retained UI task step a monotonically increasing task-local sequence ID. Keep the 20-row state-history cap, the four-row animation queue, the 380 ms transition and the inherited easing/row behavior. Clear/restart and task switching must not replay stale rows.
- Add focused backend/frontend regression tests for source scoping, identifier-kind scoping, missing IDs, cache eviction, history rollover, burst queueing, final displayed content, clearing and task switching.

## Explicitly unsupported

The verified Codex CLI observer only carries the exact four captured event names and has no source event ID, session/turn identity, text, or delta. `BrokerUpdate` has normalized full-record upserts and a session-state patch; it has no text-delta or independent control-event schema. Therefore this packet does not infer IDs, add an upstream event schema, or claim raw text-delta coalescing. That feature remains disabled/unimplemented until a verified producer and normalized delta contract exist. The source-ID dedupe API is not wired to the anonymous observer by this packet.

## Excluded

- Any changes to the captured Codex hook wire contract or event set.
- Stage 6 production monitoring, Stage 5 CORE-06–11, and frontend authority/view-store integration.
- UI redesign or changes to inherited asset bytes, sound behavior, ticker motion timing, or island state behavior.
- Claims that current Codex hook events are deduplicated or that text deltas are coalesced.

## Acceptance checks

- Rust tests prove duplicate suppression before reducer mutation, source and identifier-kind scoping, missing-ID behavior, bounded eviction and replay-ID preservation.
- Frontend tests prove sequences continue past the 20-step history cap, the latest four rows survive a burst, the final newest text displays, and clear/task-switch operations cancel stale rows.
- Run the locked broker test suite, workspace Rust tests and Clippy, the frontend shell test suite and production TypeScript/Vite build, scoped Rust formatting, changed-document checks, and `git diff --check`.
- Keep CORE-05 unchecked until raw text-delta coalescing has an evidence-based normalized input contract and is implemented without dropping final content or control events.

## Local validation outcome

- `cargo test --workspace --locked -- --skip files::tests::ingest_copies_and_never_overwrites`: 69 app tests, 4 hook tests, 2 runtime-identity tests and 6 contract tests passed; 2 installed-app tests were ignored and the pre-existing temp-directory test was filtered because its default `%TEMP%` creation is denied in this sandbox.
- `cargo clippy --workspace --all-targets --locked`: passed with the inherited `needless_range_loop` warning in `src-tauri/src/hooks.rs:467`; no packet warnings.
- Focused broker-stream tests: 13 passed. Scoped Rust formatting and the locked optimized Windows app build passed.
- `npm.cmd run test:shell`: 9 passed, including task sequence rollover, queue burst/final-step preservation, clear-and-append before render, and task switching. The test runner is serial to avoid two Vite SSR workers contending for the same HMR port.
- `npm.cmd run build --ignore-scripts`: TypeScript check and Vite production bundle passed. The build needed an approved elevated rerun because sandboxed esbuild could not read the repository parent directory.
- The changed-document Markdown checker validated four files; `git diff --check` passed.
- No source IDs or text deltas are emitted by the verified Codex observer, and the new deduplication ingress is not connected to it. Raw text-delta coalescing and production hook deduplication remain unsupported; CORE-05 remains unchecked.
