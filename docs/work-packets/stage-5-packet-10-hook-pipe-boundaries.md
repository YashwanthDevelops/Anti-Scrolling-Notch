# Stage 5 packet 10 — inherited hook relay pipe boundary

| Field | Required value |
|---|---|
| Packet | Stage 5, packet 10: complete the remaining CORE-09 hook-relay transport slice |
| Release profile | Monitor MVP transport foundation |
| Task IDs | CORE-09 — inherited app-hook relay only; the Codex observer slice was completed by packet 1 |
| Branch / PR | `work/stage-5-core-09-pipe-boundaries` / pending |
| Dependencies | Packet 1 Codex pipe hardening (PR #15 merge `40d2fce80c5cae04138f50038ed1c4057cd99cfe`), packet 9 (PR #24 merge `6efabcace80b8137acb759f2e9e5585f0d8c57f0`), existing hook relay wire contract |
| User outcome | The inherited hook relay accepts only bounded messages from the current Windows user and cannot be held indefinitely by idle or excessive clients |
| In scope | Share the private named-pipe server helper; apply the current-user DACL, same-user sender verification, SID-only pipe namespace, first-instance collision rejection, bounded listeners/clients, one-message 1 MiB input framing, two-second receive/response-write deadlines, bounded relay stdin and decision response |
| UI/assets contract | No visual, asset, sound, animation, tray, settings, or interaction redesign; retain the existing hook JSON object and bare `allow`/`deny` response contract |
| Excluded | New hook events or schemas; changing the permission acknowledgement/decision policy; Codex reducer, request router, monitor, Tauri commands or frontend store; Stage 6 and all Stage 4 SHELL-08/09 work |
| Validation selection | Focused Windows pipe and hook tests; complete locked Rust workspace tests and Clippy; scoped rustfmt; existing Codex hook transport replay; runtime-identity check; changed-document checker and diff check |
| Ownership | Coordinator owns the packet implementation, validation, commit, push, PR and merge |

## Capability and architecture contract

This packet does not expose a new Codex capability or change the accepted Codex CLI 0.157.1 four-event observer contract. The inherited app pipe remains a separate hook path and preserves its existing JSON payload and permission request exchange. The app checks the current-user SID after a bounded read and before parsing or forwarding payload bytes. The relay authenticates the server SID before sending. A missing SID fails closed; there is no username fallback.

The app pipe uses a protected DACL, rejects remote clients, claims the first pipe instance, and uses a fixed listener and active-client bound. A saturated or failed request is dropped neutrally so a permission request returns to the upstream terminal fallback. No Codex permission capability is enabled by this packet.

## Acceptance evidence

- A live Windows pipe test reads the current-user DACL from the server handle and verifies that a second first-instance creation fails.
- The inherited app pipe accepts a real same-user JSON hook message and validates the sender before parsing. Missing or mismatched SID rejection remains covered by the existing win_user contract test; cross-account runtime access is reported unverified.
- A silent connected client is closed at the configured two-second receive deadline. Invalid JSON, non-object JSON, oversized payloads and extra framed data in one write are rejected.
- Listener, client and instance limits are finite and tested. The relay input and response readers are bounded.
- With no listener, the relay returns immediately within its existing 300 ms connect budget. Existing 800 ms acknowledgement, 108 s app decision deadline and 110 s upstream permission budget remain unchanged.
- The Codex observer transport contract and replay remain unchanged. No new source event or request semantics are inferred.
- Rust tests/checks, scoped formatting, the relay and app builds, existing transport replay, runtime identity, changed-document validation and diff checks pass.

## Local implementation and validation

The inherited app pipe now shares private server creation with the Codex observer, authenticates same-user clients before JSON parsing, and holds only eight listener and eight active-client permits. Its accepted object format and permission exchange remain intact. Input is bounded to the existing 1 MiB hook contract, receive and decision writes are bounded to two seconds, and relay stdin/decision responses cannot grow without limit. The app and relay require the SID-namespaced pipe path.

The focused Windows pipe suite passed 12 tests and the hook relay suite passed 7. The final locked workspace run passed 93 app tests, 7 hook tests, 2 runtime-identity tests and 6 hook-contract tests; 2 installed-app tests were ignored. Workspace Clippy passed with only the inherited warning in `src-tauri/src/hooks.rs:467`. Scoped rustfmt, the app debug build and optimized hook relay build passed. The established Codex hook transport replay passed the four accepted projections, nine neutral negative cases and four absent-app cases (14–18 ms). The runtime-identity check passed; the changed-document checker validated four Markdown files and the diff whitespace check passed. Hosted CI and PR integration remain pending.

## Integration and limitations

Cross-account runtime connection testing is unavailable on this single-account machine. The live current-user DACL, same-user delivery, collision behavior, missing/mismatched identity policy, timeout and no-app fallback have separate automated evidence. This packet does not claim production Codex monitoring or enable Codex request/managed-session capabilities.
