# Stage 5 packet 1 — secure Codex CLI observer pipe

| Item | Contract |
|---|---|
| Packet | Stage 5, packet 1: bounded hardening of the verified Codex CLI observer transport |
| Base | Integrated `origin/main` at PR #14 merge `2989f668035daffd868a598ae6636358f5317870` |
| Task | The Codex observer portion of CORE-09 only; CORE-09 remains open until its remaining planned scope is addressed |
| Status | Local implementation and validation complete; pull request and hosted CI evidence are pending |
| Product/API contract | Preserve the two-field wire message (`wire_version`, `event`) and the four verified events: `SessionStart`, `UserPromptSubmit`, `Stop`, and `SessionEnd` |
| Data rule | No prompt, path, session ID, provider credential, or unverified hook event may cross or be logged from this pipe |
| UI/assets contract | No UI, animation, sound, settings, or inherited resource change |
| Required platform | Windows x64; use the installed pinned Rust toolchain and existing dependencies |

## Included work

- Create every Codex observer pipe instance with a protected DACL granting access only to the current user's SID; reject remote clients and fail closed if the SID or security descriptor cannot be resolved.
- Verify the sender SID from the last pipe message before parsing or logging it. Revert thread impersonation before returning to the async runtime; fail closed on impersonation/token errors.
- Preserve first-instance ownership checks. Bound the listener pool, active receive handlers, message framing/size, accepted schema, and per-client read time. Retry listener-instance failures without attaching to an existing pipe owner.
- Remove the username fallback only from the Codex observer pipe naming path. Keep the legacy Claude pipe behavior outside this packet.
- Add a real Windows named-pipe test for the installed DACL, collision rejection, same-user identity, exact wire delivery, client timeout, and concurrency bound. Keep the existing CLI-boundary replay and no-app relay checks.

## Excluded work

- Any new hook event, wire field, upstream schema, session identity, or event correlation.
- The legacy Claude pipe/permission flow, hook installation, Codex configuration changes, or credential handling.
- CORE-01–08 or CORE-10–11 types, reducer, snapshots/replay, SQLite history, request router, frontend store, or monitoring UI.
- Stage 6 production Codex monitor/installer and all later stages.
- Cross-account execution if the current machine has no second test account. Inspect the actual pipe DACL and record cross-account connection attempts as runtime-unverified when unavailable.

## Acceptance evidence

1. Windows integration test reads the DACL from the live pipe handle and confirms the protected current-user-only ACE; a second first-instance creation fails.
2. A real same-user client connects, is identified via its pipe-message security context, and delivers only an accepted two-field observation. Missing/mismatched token identity is unit-tested as rejection.
3. Malformed, oversized, unsupported-version, unknown-field, unterminated, and multi-frame messages are rejected without logging their content.
4. A connected silent client is closed by the existing two-second deadline; active receives cannot exceed the fixed semaphore capacity, and listener count/pipe instances remain finite.
5. `windows/scripts/test-codex-hook-transport.ps1` passes for the four accepted projections and neutral unsupported inputs; absent-app latency remains within its existing bound. Do not claim that this replay is a production app-monitor test.
6. Rust formatting/tests/checks, relay build, the Windows CI workflow, changed-document checks, resource/identity checks, and diff review pass. Report cross-account limitations explicitly.

Stop at this packet's boundary. Do not check CORE-09 as wholly complete unless the rest of the planned CORE-09 pipe scope has also been implemented and evidenced.
