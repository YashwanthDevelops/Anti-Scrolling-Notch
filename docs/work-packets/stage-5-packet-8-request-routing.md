# Stage 5 packet 8 — bounded request-routing contract

| Field | Required value |
|---|---|
| Packet | Stage 5, packet 8: CORE-07 request routing |
| Release profile | Monitor MVP routing contract only |
| Task IDs | CORE-07 only |
| Branch / PR | `work/stage-5-core-07-request-routing` / pending |
| Dependencies | Integrated CORE-01 domain types, CORE-02 reducer, CORE-03 stream, CORE-04 storage, and the backend capability registry; no verified Codex request adapter exists |
| User outcome | A validated request has an exact backend route, can be acknowledged only after presentation, and can receive at most one bounded reply reservation before its explicit deadline |
| Status | Locally validated; publication pending |

## In scope

- Add a pure Rust broker request router over the existing `PendingRequest` model. Do not define or accept any new Codex hook event, reply wire schema, or request producer.
- Preserve the exact source-scoped request ID and its supplied session/turn/tool correlation fields. Do not derive, concatenate, or substitute process-local IDs for source IDs. Reject conflicting reuse of the same source request ID.
- Require an explicit source-supplied deadline; inject the current time into router operations for deterministic checks. A missing/elapsed deadline fails closed.
- Bound tracked request records and simultaneous reply reservations. When either bound is reached, reject without evicting an unresolved request or issuing an effect.
- Bind presentation acknowledgement to a one-use token for the exact request. Do not permit a reply before acknowledgement.
- Reserve a typed permission decision once, atomically, only when the current backend capability snapshot authorizes `codex.permissionDecision`. Reserve before any future adapter IO and accept only one typed completion for that reservation.
- Retain the existing backend capability registry's disabled decision state. Test a synthetic enabled registry only to exercise the internal route contract; it is not runtime compatibility evidence.
- Return typed outcomes for accepted, definite pre-send failure, and uncertain delivery. A local acknowledgement, emitted UI event, or queued effect never marks the source request resolved.

## Architecture and UI/resource contract

The router is backend-only and deterministic. It performs no IPC, network, process, filesystem, UI, sound, or animation work. A future bounded effect worker must own any transport side effect and report its typed result. No Tauri command or frontend bridge is added in this packet. Existing Coucou/Mochi UI, assets, sound, and animation behavior remain untouched.

The accepted Codex observer continues to transport only its versioned two-field message and four observed event names. The request router is not connected to that observer. The existing `codex.permissionDecision` and `codex.managedSession` production capabilities remain disabled; the legacy Claude pipe and its local request IDs/reply semantics are outside scope.

## Explicit limitations

- No supported Codex request event, decision response, or input reply contract is available in the accepted fixture. There is no live request producer or adapter, so all Codex reply behavior remains disabled.
- The router guarantees at most one reservation and one local completion per registered request during its lifetime. It does not claim exactly-once external delivery across process crashes; that requires a verified source idempotency/reconciliation contract and durable bookkeeping.
- The router requires a source deadline and fails closed when one is missing. No default timeout or fallback duration is inferred from Claude behavior.
- Conflicting/reused request IDs and capacity exhaustion are rejected without sending a reply. This packet does not add CORE-08 request clearing, external-client resolution, or crash recovery.

## Excluded

- CORE-06 source-event/turn timer integration, Stage 6 production monitoring, hook-install changes, request schema expansion, adapter IO, request UI, frontend view-store wiring, and any enabled decision/input capability.
- CORE-08 expiry/cancellation cleanup or another-client resolution; CORE-09 pipe changes; CORE-10 snapshot/view-store UI integration; CORE-11 architecture checks.
- Any Coucou UI, asset, sound, animation, settings, tray, window or product-identity changes.

## Acceptance evidence

- Tests prove exact source/request/scope matching, conflict rejection, explicit-deadline enforcement, presentation-token acknowledgement, disabled/stale capability rejection before mutation, bounded tracking and concurrent reservations, duplicate-claim suppression, one-time completion, and no retry after uncertain delivery.
- Contract tests use the existing typed domain records; no synthetic hook payload is added to the compatibility fixture.
- The existing capability tests continue to prove that permission decisions and managed sessions remain disabled in production.
- Run the locked request-router and broker tests, Rust workspace tests, Clippy, scoped formatting, relevant frontend/resource checks only if their inputs change, changed-document validation, and `git diff --check`.
- No live Codex request or external exactly-once behavior is claimed. Runtime request delivery remains disabled and unverified.

## Local implementation and validation

The backend router is implemented in `windows/src-tauri/src/broker/request_router.rs`. It exposes exact request and correlation IDs through the route key, rejects conflicting reuse across all stored scope fields, binds acknowledgement to the current presentation token, requires a live explicit deadline and the current capability snapshot, and reserves a bounded single-use decision attempt. Every completion outcome is terminal for that in-process route. It has no producer or transport and does not change runtime capability enablement.

Validation passed: all 12 request-router tests; `cargo test --workspace --locked` (92 passed, 2 installed-app tests ignored); `cargo clippy --workspace --all-targets --locked`; scoped rustfmt on `broker/mod.rs` and `broker/request_router.rs`; `docs/check-markdown.ps1`; and `git diff --check`. Clippy reports the pre-existing `src-tauri/src/hooks.rs:467` `needless_range_loop` warning. Workspace-wide `cargo fmt --all -- --check` reports pre-existing formatting drift in untouched Rust code and unchanged regions of `capabilities.rs`; those files were not reformatted to avoid unrelated changes. No frontend/resource check was needed because those inputs are unchanged. Windows CI and PR checks remain pending publication.
