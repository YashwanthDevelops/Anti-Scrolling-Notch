# Anti-Scrolling-Notch project instructions

@RTK.md

## Product and source

- This is YashwanthDevelops/Anti-Scrolling-Notch, a Windows Codex companion based on Coucou's Windows source. The implementation plan is `docs/implementation-plan.md`; actual progress is `docs/execution-ledger.md`.
- Read the relevant upstream instructions and module code, but follow the user's approved Windows/Codex scope and original product identity. Keep the macOS source as reference unless explicitly assigned work there.
- Preserve MIT attribution and the separate asset-license notice. Use original product name, app IDs, icon, character and sounds before distributing this product.
- Do not imply that inherited Claude/Coucou source is already implemented for Codex. Mark capabilities and validation truthfully.
- Coucou is the foundation, not a mandatory parity specification. Record inherited features as keep/adapt/defer/remove according to their Codex benefit and release profile. Deferred services/controls must remain inactive.

## Bounded execution and completion

- Treat the plan as a master specification. Activate one stage work packet and one bounded implementation PR at a time using `docs/stage-packet-template.md`; review/integrate it before starting the next packet. Split large stages into sequential PRs.
- Follow Monitor MVP -> interactive v1 -> selected extensions. Stage 9A notifications/recovery belong to MVP and must not depend on Stage 8 managed chat. Optional services/shared desktop/voice/platform extensions do not block core delivery.
- A production capability needs supported-version evidence, fixture/integration tests, tested failure behavior and documented fallback. Enforce this in backend and UI; unknown/unverified capabilities remain disabled. Demo fixtures do not prove production readiness.
- Authority flows through backend adapters -> normalized events -> pure Rust reducer -> snapshot/event sequence -> frontend view store -> UI. No direct frontend Codex/GitHub queries, authoritative state or request-resolution decisions. Local drafts/focus/animations are presentation state; IO belongs outside the reducer.
- A feature requires real backend behavior, failure behavior, persistence/recovery and automated verification. Explain relevant N/A for stateless/presentation-only work; never count a renderer/placeholder as completion.
- Track task/profile/packet sub-status and repeat Stages 11–12 gates for each release profile/version. The complete core project requires interactive v1 QA and delivery; deferred extensions remain deferred.

## Commits and pushes

- The user explicitly requests frequent GitHub commits and pushes. Make a commit for each coherent verified change; push every completed commit immediately and before ending a work session.
- Verify `origin` is `https://github.com/YashwanthDevelops/Anti-Scrolling-Notch.git`. `upstream` is Coucou and is fetch-only. Never publish changes or releases to Coucou.
- Stage only intended files and inspect the staged diff. Keep credentials, local logs, attachment copies and generated build output out of commits.
- Use descriptive messages and update the execution ledger with task IDs, evidence and pending work. Record hashes in subsequent progress entries or an externally verified milestone, without creating a self-reference loop.
- Verify the remote branch SHA after push. If a push fails, preserve local commits and report the reason; do not claim upload success.
- No force push, unrelated-history deletion or resetting other work. Preserve any new destination changes with reviewed merges.
- After the initial source/documentation import, implement on short feature branches and integrate reviewed working milestones into main.

## Agents

- The user permits agents when useful. Give each an independent task and clear file/module ownership.
- Only the coordinating agent stages, commits and pushes a shared checkout. Agent reports include changed paths, checks and limitations.
- Use isolated worktrees for independent branch work. Do not run concurrent Git mutations or overwrite another agent's edits.

## Validation and runtime boundaries

- Run relevant checks before committing code; documentation-only changes use consistency/link/diff checks. Never claim unrun builds/tests passed.
- Keep observing hooks fast and neutral. Approvals need an explicit user decision, exact request routing and bounded fallback. Respect Codex's hook review/trust flow.
- Existing-session monitoring and companion-managed App Server control have different capabilities. Do not invent unsupported desktop APIs or deep links.
- Keep service secrets backend-only and OS-protected. Use typed commands/argument arrays, redacted diagnostics and bounded state/history.
- Inspect inherited release destinations and replace identities/assets before release tags or publishers are used.
