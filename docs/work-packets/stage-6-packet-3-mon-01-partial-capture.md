# Stage 6 packet 3 — MON-01 bounded identity capture (partial)

> **Status: evidence captured; MON-01 remains open.** This documentation-only supplement records one owner-authorized isolated CLI run. It does not expand the adapter contract, complete MON-01, enable monitoring, or authorize MON-03.

## Packet identity and scope

| Field | Value |
|---|---|
| Packet | Stage 6, packet 3: MON-01 partial runtime identity evidence |
| Release profile | Monitor MVP foundation |
| Task IDs | MON-01 evidence only; implementation remains incomplete |
| Branch / PR | `work/stage-6-mon-01-partial-identities`; PR link recorded after publication |
| User outcome | Preserve verified session/turn presence and equality from a real isolated CLI run while making the remaining evidence gap explicit. |
| In scope | Add the sanitized trace fixture; update compatibility, gate, plan and ledger documentation. |
| UI/assets contract | No UI, asset, sound, animation or interaction changes. |
| Excluded | Any product code or adapter change; additional capture/retry; new event support; reducer/storage/monitor UI; MON-03 or later tasks; Ollama, normal Codex configuration, credentials or provider installation. |
| Validation selection | JSONL structure and field-correlation assertions, changed-document link/fence checks and `git diff --check`. |

## Capture provenance and observations

One bounded invocation of Codex CLI `0.157.1` used a disposable `CODEX_HOME`, disposable local repository, loopback fake Responses endpoint and neutral hook recorder. It did not use the owner's normal Codex authentication, OpenAI credentials or production configuration. The disposable home had no `auth.json`. The loopback provider summary recorded one `/v1/responses` request, `authorization_header_present: false`, and no advertised tools (`tool_count: 0`, `tools: []`).

The sanitized fixture [`captured-identities-cli-0.157.1.jsonl`](../../tests/compat/codex-hooks/captured-identities-cli-0.157.1.jsonl) records these delivered events and only selected fields:

| Event | Runtime observation |
|---|---|
| `SessionStart` | `session_id` present; `source` was `startup`. |
| `UserPromptSubmit` | Same `session_id`; `turn_id` present; prompt contents omitted. |
| `Stop` | Same `session_id` and `turn_id`; assistant text omitted. |
| `SessionEnd` | Same `session_id`; `reason` was `other`. |

Identifiers were replaced with type-scoped HMAC pseudonyms using a per-run salt. The fixture's integer `sequence` is assigned by the recorder, not Codex. Prompt text, command text, tool input/output, transcript paths, working directory, credentials, raw request bodies and private diagnostics are not committed. The disposable home/repository, scripts, salt and raw diagnostics remain local-only.

The process also attempted an unauthenticated ChatGPT featured-plugin metadata request and received HTTP 401. No authorization header was sent to the loopback provider, but the process did not have an exclusively loopback network footprint. The probe was not repeated after this observation.

## Acceptance gap and disposition

The capture establishes session ID presence on the four events and matching values across them within this one turn. It establishes a matching turn ID between `UserPromptSubmit` and `Stop`. It does not establish cross-session uniqueness, independent-turn behavior, event-source IDs, or duplicate/retry behavior.

The loopback Responses request exposed zero tools, so the run delivered no `PreToolUse`, `PostToolUse`, `SubagentStart` or `SubagentStop`. It provides no runtime evidence for tool-use IDs, tool/session/turn relationships, parent/agent identity or ordering/correlation of those events. The version-pinned schemas remain schema facts only for these unsupported fields/events. The existing adapter still accepts only the four confirmed event names, discards identity fields, and rejects unverified events. Monitoring stays disabled; MON-01 remains open and MON-03 remains unopened.

The original accepted projection [`captured-projections.jsonl`](../../tests/compat/codex-hooks/captured-projections.jsonl) is unchanged. See the [compatibility matrix](../codex-compatibility.md#captured-loopback-contract) and [MON-01 evidence gate](stage-6-packet-2-mon-01-identity-evidence.md) for the current contract and remaining requirements. No product behavior changed.
