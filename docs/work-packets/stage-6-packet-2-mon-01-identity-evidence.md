# Stage 6 packet 2 — MON-01 runtime identity evidence gate

> **Status: blocked; preparation only.** This packet records the evidence needed before MON-01 implementation can safely start. It does not implement or complete MON-01, expand the accepted hook contract, or enable monitoring.

## Packet identity and scope

| Field | Required value |
|---|---|
| Packet | Stage 6, packet 2: MON-01 runtime identity evidence gate |
| Release profile | Monitor MVP foundation |
| Task IDs | MON-01; evidence gate only, implementation remains open |
| Branch / PR | `work/stage-6-mon-01-capture-gate` / preparatory evidence-gate PR; see execution ledger |
| Dependencies | MON-02 integrated by PR #32; accepted Codex CLI 0.157.1 four-event observer contract |
| User outcome | Make the missing runtime evidence and exact resumption criteria reviewable without presenting anonymous events as session monitoring. |
| In scope | Document the MON-01 capture prerequisites, privacy rules, evidence requirements, and fail-closed conditions. |
| UI/assets contract | No UI, asset, sound, animation, or interaction changes. |
| Excluded | Observer/parser changes; new event or field support; reducer/storage changes; monitor UI or capability enablement; normal Codex authentication/configuration; provider installation or switching; MON-03 and later tasks. |
| Validation selection | Changed-document consistency/link checks and `git diff --check`; no product build or runtime claim. |
| Ownership | Coordinator; GPT-6.1 Sol provided read-only review. |

## Current verified boundary

The accepted CLI 0.157.1 runtime projection contains only `SessionStart`, `UserPromptSubmit`, `Stop`, and `SessionEnd`. It records `source: startup`, prompt presence, and the `SessionEnd` reason, but no session, turn, tool-use, parent, or agent identity. The probe used a disposable `CODEX_HOME`, local repository, and loopback fake Responses endpoint; the projection is not a complete hook payload. See [Codex compatibility evidence](../codex-compatibility.md#captured-loopback-contract) and [`captured-projections.jsonl`](../../tests/compat/codex-hooks/captured-projections.jsonl).

The version-pinned schemas declare possible `session_id` members on the four observed event types, `turn_id` on `Stop` and `UserPromptSubmit`, and `agent_id`/`agent_type` on `UserPromptSubmit`. These are schema facts only. The accepted runtime capture did not retain or verify those values. Tool-use and subagent event names were configured during the probe but were not delivered. Schema availability, fixtures, generated IDs, and replay tests do not establish runtime delivery or correlation semantics.

The current observer parses only `hook_event_name`; its internal message contains only `wire_version` and `event`. Unknown tool/subagent events remain rejected. This anonymous boundary and the disabled monitoring capability are intentional until the evidence below is available.

## Evidence required to resume MON-01 implementation

The evidence must come from a genuine Codex runtime on an explicitly identified supported version and surface. It may be collected in an already permitted isolated environment or supplied as a sanitized trace by the project owner. It must not use the owner's normal Codex authentication, OpenAI credentials, or production configuration unless the owner explicitly changes that constraint. Do not install or switch providers without explicit authorization.

The trace must establish, from actual delivered payloads:

1. The exact runtime version, surface, and observed event names.
2. Which session and turn identity fields are present, when they appear, and whether values correlate consistently across lifecycle events.
3. At least one delivered tool-use lifecycle with the actual tool identity fields and a verified relation to its session and turn.
4. At least one delivered parent/agent relationship with the actual field names, values, and correlation semantics.
5. Event ordering and duplicate/retry behavior relevant to preserving those identities. Do not infer ordering or uniqueness from a field name.

Sanitization must remove prompt contents, commands, tool input/output, credentials, and private paths. Replace observed identifiers with stable, type-scoped pseudonyms consistently across the trace so equality and parent/child relationships remain testable. Do not fabricate identifiers or events. Retain provenance sufficient to identify the tested Codex version/surface, but do not commit raw private payloads.

## Implementation and acceptance gate after evidence arrives

Only after the trace satisfies the evidence requirements above may a separate MON-01 implementation packet extend the adapter. That packet must:

- add only runtime-observed event names and fields to the versioned contract;
- preserve observed source session, turn, tool-use, and parent/agent identities without substituting app-local counters or timers;
- include the sanitized runtime fixture and tests for field preservation, correlation, malformed/oversized input, unknown events, and neutral fallback;
- keep capabilities disabled for any event or identity relation that the capture did not prove; and
- preserve the existing Coucou/Mochi UI, animations, sounds, and assets.

If the required tool-use or parent/agent information is not delivered by the permitted surface, MON-01 remains open. Do not mark it complete from schema inspection, change labels to imply session awareness, activate MON-03, or move to another plan task without explicit sequencing authorization.

## Review and current disposition

GPT-6.1 Sol's read-only review found no bounded implementation that can satisfy MON-01 from the accepted evidence. It recommends keeping MON-01 unchecked and seeking an independently reviewable sanitized runtime trace or explicit authorization for a concrete compliant capture environment. This packet records that gate; it is not a product implementation or a substitute for the required runtime evidence.
