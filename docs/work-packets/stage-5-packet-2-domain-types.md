# Stage 5 packet 2 — versioned backend domain types

| Item | Contract |
|---|---|
| Packet | Stage 5, packet 2: CORE-01 only |
| Base | Integrated `origin/main` at PR #15 merge `40d2fce80c5cae04138f50038ed1c4057cd99cfe` |
| User outcome | The backend has one explicit, versioned, validated model contract for future session, turn, tool, request, repository, integration and event processing. |
| Status | Local implementation, workspace validation and focused tests are complete; hosted CI and integration are pending |
| Current Codex evidence | The accepted observer still carries only `wire_version` and one of four captured event names; those observations contain no source IDs, prompt text, paths, tool payloads or request details. |
| Compatibility rule | Domain schema version 1 is distinct from hook pipe `wire_version: 1`; no upstream event name or payload schema is added. |
| Ownership | Backend Rust domain module only; do not replace legacy poller/presentation types in this packet. |

## Included work

- Add versioned `Session`, `Turn`, `ToolItem`, `PendingRequest`, `Repository`, `Integration` and generic `EventEnvelope<T>` records in the backend broker module.
- Use bounded opaque IDs paired structurally with their source; reject empty, oversized and control-character IDs rather than rewriting them.
- Keep session lifecycle, activity, waiting and connection health independent and default each to `Unknown`.
- Keep source correlations optional. The anonymous Codex hook observation must not be converted into an identified session or turn.
- Define only normalized application state. Do not add raw tool arguments/output, credentials, arbitrary API JSON, local filesystem paths, credential-bearing remote URLs or free-form integration errors.
- Keep represented request/approval states descriptive only; they do not enable an unverified hook or App Server response path.
- Wrap the existing `CodexHookObservation` in the internal generic envelope only in tests; do not change the pipe wire envelope.

## Explicit exclusions

- CORE-02 reducer or authoritative state mutation.
- CORE-03 sequence allocation, snapshots, replay buffers or Tauri snapshot/delta commands. The envelope sequence type is only a non-zero value contract.
- CORE-04 SQLite, history persistence, migrations, logs, retention or redaction workers. Any display-content fields remain unpopulated and must be governed by CORE-04 before storage.
- CORE-05 deduplication/coalescing behavior; no dedup keys are generated in this packet.
- CORE-06 timers; CORE-07/08 request routing or resolution; CORE-10/11 frontend store and architecture flow.
- Stage 6 production monitor, Codex hook installation, any new Codex event, App Server schema or capability enablement.
- Changes to existing `IntegrationUpdate`, `pipe::Pending`, `CapabilityRegistry`, frontend `AgentTask`, UI, assets, sounds or animation behavior.

## Acceptance evidence

1. All seven types serialize and deserialize as schema version 1; missing or unsupported versions and unknown fields/enum variants fail closed.
2. IDs preserve their exact text, enforce a 512-byte maximum, reject empty/control-character values, and remain distinct when the same source ID appears under two different sources.
3. Session lifecycle, activity, waiting and health remain separate fields with `Unknown` defaults; turn outcome is not inferred from `Stop`.
4. The EventEnvelope can wrap the existing observed hook message while keeping optional correlation empty and the nested `wire_version`/`event` payload exact.
5. Existing captured-event parser, pipe replay, backend tests, Clippy, scoped formatting, documentation checks and Windows CI pass. No existing capability is enabled and no legacy presentation or poller type is changed.

This packet closes CORE-01 only. Stop before CORE-02 or any Stage 6 work unless Stage 5 packet 2 is integrated and the next packet is separately scoped.
