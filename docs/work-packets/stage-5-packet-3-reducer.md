# Stage 5 packet 3 — normalized state reducer

| Item | Contract |
|---|---|
| Packet | Stage 5, packet 3: CORE-02 only |
| Base | Integrated `origin/main` at PR #16 merge `1cc8f7f953a8a1cdcca7806967e92fdfcbd6227e` |
| User outcome | A pure Rust reducer owns current normalized backend records and can query them by exact source-scoped session, thread, turn, tool and agent identities. |
| Status | Implementation and local validation complete; hosted CI, review and integration pending |
| Current Codex evidence | The observer still accepts only the captured anonymous four-event contract. It is not connected to this reducer. |
| Ownership | Backend Rust state reduction only; no event adapter, Tauri command, persistence or frontend change |

## Included work

- Add an in-memory `BrokerState` that upserts the existing Session, Turn, ToolItem, PendingRequest, Repository and Integration records.
- Key records by their exact typed identities and use source-scoped secondary indexes for thread, agent, parent session, turn and tool relationships. Keep index reads deterministic and remove stale index entries when a record's relationships change.
- Apply lifecycle, activity, waiting and connection updates as independent session-state patches. `None` means unchanged; `Some(Unknown)` is an explicit reset.
- Scope an unscoped pending-request thread/turn/tool/agent reference to its linked session source when available; use the request's source only for detached requests. This is an internal index rule, not a claim about upstream event payloads.
- Accept normalized child records before their parent session is present. Their exact parent IDs remain indexed and queryable when the parent later arrives.
- Make identical upserts and patches idempotent. This is value idempotence, not source-event deduplication.

## Explicit exclusions

- No Codex, Claude, GitHub or Windows source-event schema is added. `BrokerUpdate` is an internal backend mutation, not an upstream protocol.
- No translation from the anonymous `CodexHookObservation`, no new event variant, no monitor, no pipe wiring and no capability enablement.
- No event sequence assignment, ordering conflict resolution, snapshot/replay, bounded event queue, persistence, SQLite, deduplication/coalescing, request routing, timers, Tauri IPC, frontend store or UI. These remain assigned to later CORE tasks.
- No record removal semantics or history-retention policy; CORE-04/08 define those needs.

## Acceptance evidence

1. Tests prove separate sessions and relationship indexes for identical raw IDs from different sources.
2. Tests prove child-before-parent updates remain queryable and identical repeated values return `Unchanged`.
3. Tests prove replacing a session, turn, tool or request relationship removes all old index entries and makes the new entries queryable.
4. Tests prove a patch changes only its named lifecycle/activity/waiting/connection axes and a first partial patch leaves other axes `Unknown`.
5. Tests cover repository and integration replacement by identity; existing domain and Codex hook contracts remain unchanged.
6. Workspace tests, Clippy, scoped Rust formatting, documentation checks and Windows CI pass.

The reducer applies updates in call order. It does not claim to recognize which of two conflicting source updates is newer. CORE-03 owns monotonic event ordering and replay; the current captured hook payload has no identity or sequence with which this packet could infer either.
