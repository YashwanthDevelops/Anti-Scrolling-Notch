# Stage 3 packet 1 — Codex CLI hook compatibility

| Field | Value |
|---|---|
| Packet | Stage 3, packet 1: installed Codex CLI observing-hook contract |
| Release profile | Monitor MVP |
| Task IDs | COMPAT-01 (CLI slice), COMPAT-02 (CLI slice), COMPAT-03 (observer contract), COMPAT-09 (sanitized hook fixtures/matrix slice) |
| Branch / PR | `work/stage-3-codex-hooks` → Anti-Scrolling-Notch `main` |
| Dependencies | Stage 2 packet integrated at `965376b39af1d0351fe479778dbfa5a1f637070b`; project-owner BASE-05 acceptance recorded |
| Status | Implementation and local packet checks complete; publication/review remains |
| User outcome | Implement a backend-only Codex CLI observer adapter limited to the four lifecycle events delivered by the successful isolated loopback turn, while keeping every unverified event disabled. |
| In scope | Record the installed CLI version and accepted loopback hook evidence; preserve a sanitized recorder projection; implement a strict four-event adapter contract and separate Codex named-pipe path; verify malformed/unsupported input, neutral output and pipe delivery with isolated fixture replay; document that the Ollama-specific test was not performed because Ollama is not installed. |
| UI/assets contract | No app UI, Coucou/Mochi assets, sounds, animation timing or layout changes. The packet adds only a passive backend adapter and does not change the Claude hook path. |
| Excluded | Codex Desktop support unless its version/surface is independently probed; hook installation into the normal Codex home; user-facing monitoring UI/session state/history; the Stage 5 reducer, persistence, event ordering and snapshot pipeline; managed App Server (COMPAT-05); hook permission decisions (COMPAT-04); shared-desktop attachment (COMPAT-06); navigation and Windows toasts/DPI/drop (COMPAT-07/08); production capability registry/UI enforcement (COMPAT-10); remaining Stage 4–13 work. |
| Validation selection | Accepted installed CLI version/feature and loopback evidence; Rust contract/unit tests; release hook build; isolated Windows named-pipe replay for all four captured event names and an unverified event; neutral stdout/stderr/exit and absent-app bounded-return checks; changed-document validation. Do not load, alter, or copy normal Codex configuration/auth. No live API key or service credential is permitted in this packet. |
| Ownership | Coordinator owns adapter and documentation changes, tests, staging, commit, push and PR. |

## Capability and architecture contract

Only the installed CLI surface may become eligible for Monitor MVP support in this packet. Record Codex Desktop as unverified and disabled unless a separate real probe identifies its version and confirms the same behavior. The managed App Server and shared-desktop adapter are explicitly out of scope.

The accepted loopback probe exercised Codex's command-hook boundary with a self-authored, read-only recorder and neutral exit/output. The adapter accepts only the four event discriminators delivered by that turn, strips the input down to a versioned internal event envelope, and relays it through a Codex-only named pipe. It does not inject prompt context, make permission decisions, install hooks, or send events to the frontend. Stage 5 remains responsible for authoritative reduction and state; Stage 6 remains responsible for user-facing monitoring workflows. A verified event discriminator does not enable any unverified event or complete COMPAT-10.

## Packet acceptance evidence

- Record the installed CLI version and effective `hooks` feature state, and cite the current official hook reference.
- Treat only `SessionStart`, `UserPromptSubmit`, `Stop`, and `SessionEnd` as runtime-observed. Keep `PreToolUse`, `PostToolUse`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, and `Interrupt` explicitly unverified and rejected by the adapter; do not substitute source/docs for runtime evidence.
- Verify the adapter returns neutral success, writes no stdout/stderr, never transmits prompt text or arbitrary hook fields, and sends only the versioned event envelope over a Codex-only pipe. Test absent app, malformed input, unsupported events, oversized input and a bounded transport wait; preserve Codex's safe continuation behavior.
- Sanitize identifiers, local paths, transcript contents and prompt text in committed fixtures. Contract and transport tests must validate the observed event names, allowlist behavior, app-private wire fields and neutral semantics, not merely duplicate a hand-written parser.
- Keep normal hooks, credentials, the global Codex home and app behavior unchanged. Preserve the isolated probe artifacts as local-only evidence; do not commit unredacted probe state.
- Leave unsupported Desktop and v1/extension capabilities disabled. This packet does not satisfy COMPAT-10 or close Stage 3 by itself.

## Task status

| Task slice | Status | Evidence / remaining work |
|---|---|---|
| COMPAT-01 — installed CLI slice | Complete | `codex-cli 0.157.1`, stable `hooks` feature and successful loopback delivery are recorded. Ollama-specific smoke was not performed because Ollama is not installed. Desktop is not declared supported. |
| COMPAT-02 — four-event adapter slice | Complete | `SessionStart`, `UserPromptSubmit`, `Stop`, and `SessionEnd` are accepted by the separate backend adapter. Seven unobserved events plus permission/approval, tool metadata, compaction, interruption, session identity and Desktop remain unsupported. |
| COMPAT-03 — adapter neutral/failure slice | Complete | Contract tests and Windows pipe replay cover malformed/oversized input, unknown events, field stripping, neutral output, no receiver and bounded return. |
| COMPAT-09 — sanitized CLI artifacts slice | Complete | Recorder projections, scope limitations and runnable test commands are recorded. Broader capability/version matrix evidence remains for later Stage 3 packets. |

## PR and completion gate

This packet can close only when the four claimed CLI events pass contract and pipe-replay checks, neutral/failure paths pass, the supported surface is explicitly bounded, and local documentation/fixture checks pass. Missing runtime event coverage remains disabled and is carried into the next packet; do not claim the whole Stage 3 or the Stage 5/6 monitoring workflows are complete here.
