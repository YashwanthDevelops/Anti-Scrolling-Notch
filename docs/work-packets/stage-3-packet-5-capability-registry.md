# Stage 3 packet 5 — backend-owned capability enforcement

| Field | Value |
|---|---|
| Packet | Stage 3, packet 5: capability registry enforcement |
| Release profile | Monitor MVP hard gate; interactive v1 capabilities remain disabled |
| Task IDs | COMPAT-10; enforcement slice only |
| Branch / PR | `work/stage-3-capability-registry` → Anti-Scrolling-Notch `main` |
| Dependencies | Packets 1–4 integrated; COMPAT-07 generic Open Codex route and the COMPAT-01–03 four-event CLI slice are recorded in `docs/codex-compatibility.md` |
| User outcome | Only the exact runtime-verified generic Open Codex launch route may be requested; missing, unknown, stale or unsupported capability evidence fails closed. |
| In scope | Add a backend-owned runtime registry, discover the installed Codex Desktop package identity/version, gate the generic activation command against fresh backend evidence, make the frontend bridge consume the backend snapshot, and add positive/negative enforcement tests. Keep the production monitor capability disabled because no production monitor adapter/UI exists. |
| UI/assets contract | No new cards, controls, settings or layout. The existing UI and Coucou/Mochi assets, sounds and motion remain unchanged. Plain-browser/dev preview receives no production capabilities. |
| Excluded | COMPAT-01–03 additional runtime coverage; no provider/tooling investigation; no Codex hook installation or new event fields; Stage 4 identity work; Stage 5 reducer/store; Stage 6 monitor UI; Stage 7 GitHub workflow; Stage 8 managed sessions; toast, mixed-DPI, Explorer-drop or inherited inbox behavior changes; Stage 9–13. |
| Validation selection | Unit-test registry policy, exact/missing/unknown identity, stale generation, missing adapter and fixture rejection; test the frontend fail-closed bridge guard; run relevant Rust tests, npm capability test/build, formatting, lint and changed-document checks. Reuse COMPAT-07/08 evidence without repeating UI probes. |
| Ownership | Coordinator owns all packet files, validation, commit, push and PR. |

## Capability and architecture contract

The registry is produced by Rust from read-only runtime discovery. It is serializable to the WebView but not deserializable from it. The frontend can request only the exact capability ID and snapshot generation returned by `boot`; the `open_codex` command rediscovers the installed package, compares generations, checks the allowlist and only then invokes the verified generic activation adapter. The browser preview and stale snapshots fail closed. No frontend-supplied version or enabled flag can authorize execution.

The only capability eligible for enablement is generic activation of packaged OpenAI Codex version `26.928.2636.0` with the verified `IApplicationActivationManager` route. The four-event CLI hook parser/replay remains compatibility evidence, not a production monitor: the internal wire version does not identify the live CLI, the app has no production hook-install context, and Stage 5/6 state/UI is absent. Monitoring, tool/compaction/subagent/interrupt events, Desktop session APIs, approvals, notifications and Codex attachments remain disabled with their recorded fallback.

The existing Coucou inbox file-drop and its animations remain unchanged. COMPAT-08's unverified Explorer transport and Codex attachment delivery are distinct capabilities; this packet does not gate or alter the inherited drop behavior.

## Acceptance evidence

- Runtime discovery reports only package family and version, with a bounded, hidden, no-profile Windows query. Failure, multiple matches, a wrong family or an unknown version yields a disabled capability.
- Backend authorization rejects stale generation, unsupported ID, disabled entry and missing adapter before the action closure can run. Only the current verified `codex.openApp` entry reaches activation.
- The frontend bridge accepts only an enabled runtime registry entry and forwards its generation; null/browser, demo/fixture, malformed, disabled or stale registry snapshots cannot expose the bridge action.
- Registry data describes surface/version, transport, evidence/test, failure behavior, fallback and enabled state. It contains no user paths, session IDs, credentials or event payloads.
- Unverified controls remain absent/disabled; the registry does not claim that parser/replay equals a production monitor.
- No Coucou/Mochi interaction, inbox drop, existing sound, notification, Codex configuration or credential behavior is changed.
- Update the compatibility matrix, implementation-plan checkbox and execution ledger only after required tests pass. Leave COMPAT-01–03 and the overall Stage 3 gate open.

## Implementation results

Added a Rust-owned registry to `boot`. It records the tested surface/version, transport, evidence/test reference, failure behavior, fallback, adapter availability and enabled state for the current generic Open Codex route and the relevant disabled MVP/v1 capabilities. A bounded no-profile PowerShell query with a five-second process limit discovers the current user's `OpenAI.Codex` package family/version; a missing, ambiguous, malformed, wrong-family or unrecognized package disables generic activation. The Tauri command rediscovers identity before action and rejects stale generations and every non-enabled/unknown capability. Registry snapshots serialize outward only; the backend command accepts no frontend version, enabled flag or registry object.

The frontend stores only the Rust boot snapshot. Its `Bridge.openCodex` guard rejects browser/demo snapshots, unknown schema, missing entries, disabled entries and entries without an adapter, then forwards the snapshot generation. Rust remains authoritative and rechecks it. No new card, visible control, reducer, monitor workflow or setting was added. The accepted four-event parser contract is represented as compatibility evidence but remains disabled as a production monitor because the pipe does not prove the running CLI version and no production installation/state/UI context exists. Existing Coucou inbox ingestion, drag animations, sounds and file-drop behavior were left untouched.

Local validation passed: scoped `rustfmt --edition 2021 --check src-tauri/src/capabilities.rs src-tauri/src/codex_navigation.rs`; `cargo test --workspace --locked` (25 passed, 2 ignored across the workspace); `cargo clippy --workspace --all-targets --locked` (passed with the same three inherited warnings, no packet warnings); frontend capability tests (4 passed); `npm run build` including the release hook prebuild; the isolated release build; and the changed-document checks. An initial ignored read-only package-discovery smoke returned a disabled registry; after increasing the bounded PowerShell process limit from two to five seconds, the smoke passed in 0.97 seconds against the installed app, confirming the current user's `OpenAI.Codex_2p2nqsd0c76g0` package version `26.928.2636.0` enables only generic app activation. That smoke did not launch the app through the new gated Tauri command. COMPAT-01–03 remain partial, all unverified controls remain disabled, and Stage 3 is not closed.
