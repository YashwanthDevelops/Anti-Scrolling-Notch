# Stage 5 packet 4 — sequenced snapshots and replay

| Item | Contract |
|---|---|
| Packet | Stage 5, packet 4: CORE-03 backend synchronization contract |
| Base | Integrated `origin/main` at PR #17 merge `5f0637e13a25bf3d92356b2d2bd50c2ba24471e` |
| User outcome | A consumer can obtain a complete backend snapshot or contiguous versioned updates after its last applied sequence. |
| Status | Backend implementation and local validation complete; CORE-10 UI consumption, hosted CI, review and integration pending |
| Current Codex evidence | Only the accepted anonymous four-event hook contract exists; the stream accepts normalized internal updates and is not wired to the hook or UI. |
| Ownership | In-memory Rust sequencing, snapshot serialization and bounded replay only |

## Included work

- Add `BrokerStream`, which applies a normalized `BrokerUpdate`, allocates a non-zero monotonic sequence only when state changes, and records the caller-supplied observation timestamp.
- Serialize internal update variants inside the existing versioned `EventEnvelope`; preserve empty correlation, source-event ID and deduplication fields where source evidence or CORE-05 behavior does not yet supply them.
- Return a complete `BrokerSnapshot` with every current entity family and a high-water cursor. Cursor zero means an empty initial state; event sequence numbers remain non-zero.
- Retain the latest 256 event envelopes in memory. Replay is returned only when the cursor is retained and every sequence through the current head is contiguous.
- Return a snapshot for a fresh UI cursor, future cursor, expired cursor or detected replay gap. A cursor already at the current head receives an empty replay.
- Keep observation time explicit input so the stream remains deterministic and performs no clock, network, process or disk IO.

## Explicit exclusions

- No translation from `CodexHookObservation`, no new Codex/Claude/GitHub/Windows source-event schema, no hook monitor, no pipe or Tauri command wiring, and no capability enablement.
- No frontend view store/reload caller; CORE-10 will connect this synchronization contract to the UI.
- No durable history, SQLite, event-source deduplication/coalescing, request routing/resolution, timers, effect queue or text-retention policy.
- No arbitrary event correlation or source IDs are synthesized. The internal update type is versioned only by its enclosing domain `EventEnvelope` and is not an upstream protocol.

## Acceptance evidence

1. A fresh cursor receives a complete schema-versioned snapshot; each snapshot contains every entity collection.
2. State-changing updates receive strictly increasing non-zero sequences; duplicate values do not advance the sequence or create replay entries.
3. A retained cursor receives exactly the contiguous following events with unchanged observation timestamps and source kind.
4. An expired, future or internally gapped cursor receives a snapshot at the current high-water sequence.
5. Replay retains no more than its configured non-zero capacity; sequence exhaustion fails before mutating state.
6. Locked workspace tests, Clippy, scoped Rust formatting, documentation checks and Windows CI pass.

The 256-entry ring is volatile and is not history. CORE-10 will invoke the sync API on UI reload or a detected sequence gap; this packet supplies the backend resynchronization behavior without creating a frontend store. The master CORE-03 checkbox remains open until the UI caller is implemented.
