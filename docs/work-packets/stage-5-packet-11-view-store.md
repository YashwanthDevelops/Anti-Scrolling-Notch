# Stage 5 packet 11 — sequenced frontend view-store bridge

| Field | Required value |
|---|---|
| Packet | Stage 5, packet 11: CORE-10 snapshot/replay view-store foundation |
| Release profile | Monitor MVP state synchronization foundation |
| Task IDs | CORE-10 — backend snapshot/replay consumption slice |
| Branch / PR | `work/stage-5-core-10-view-store` / pending |
| Dependencies | CORE-02 reducer and CORE-03 snapshot/replay stream; packet 9 terminal-request cleanup; packet 10 protected hook transports |
| User outcome | The frontend maintains a recoverable, backend-sourced projection and surfaces synchronization failures without inventing Codex session state |
| In scope | Manage the existing `BrokerStream` in Tauri; expose its snapshot/replay sync command; publish only normalized broker updates through the same existing contract; add a typed frontend view store that applies contiguous replay and resynchronizes gaps; expose bridge errors for this sync path |
| UI/assets contract | Preserve the existing Coucou/Mochi UI, animation, sound and asset bytes; this packet adds no visible redesign |
| Excluded | Stage 6 production monitor/adapters; converting anonymous hook names into sessions or turns; new upstream event schemas; enabling Codex request/approval or managed-session controls; full migration of legacy integration/task state and all legacy bridge callers; SHELL-08/09 |
| Validation selection | Rust broker service/command tests; Node view-store tests for snapshots, replay, gaps, ordering and errors; locked Rust workspace, Clippy, focused frontend tests/build, scoped formatting, docs and diff checks |
| Ownership | Coordinator owns implementation, validation, publication and integration |

## Local implementation and validation status

Implemented on the packet branch: Tauri-managed broker stream service and snapshot/replay command; normalized-update event publication; typed bridge error and sync intent; frontend store with initial snapshot, ordered replay, duplicate/stale handling, sequence-gap recovery, immutable snapshots and retryable error state. The store receives no Codex sessions because the accepted hook observation remains anonymous and has no producer in this packet.


## Capability and architecture contract

The view store consumes only the existing versioned `BrokerSnapshot`, `BrokerSyncResponse` and normalized `BrokerUpdate` contracts. It does not read Codex or GitHub, infer IDs, or transform the captured anonymous four-event hook observation into session data. Production update producers remain disabled/out of scope until Stage 6.

The initial Tauri sync request returns a complete snapshot. Later updates are sequenced through the existing replay stream; a stale cursor, sequence gap, unsupported schema or transport failure causes a snapshot recovery or a visible store error. The frontend applies records as a projection of backend updates and never resolves requests or enables capabilities.

This packet is the first CORE-10 slice, not completion of CORE-10: existing legacy task/integration state and their bridge callers remain for a later migration packet. Unsupported Codex actions stay disabled.

## Acceptance evidence

- A managed Tauri broker service returns the existing backend snapshot/replay contract and emits only updates accepted by `BrokerStream`.
- The frontend store subscribes before its initial synchronization, applies only contiguous event sequences, ignores stale duplicates and requests a full snapshot after a gap or invalid replay.
- Backend synchronization errors are represented as typed errors and retained in the store; they are never returned as a success-shaped `null` by this new path.
- Tests cover empty and populated snapshots, upserts/patches, terminal-request cleanup, replay continuity, stale/out-of-order input, gap recovery, and transport failure/retry.
- Existing UI layout, visuals, animation timing, sound bytes, capabilities and Codex hook contract remain unchanged.
- Production monitoring is not claimed; the store receives no Codex session data until its later verified producer is implemented.
