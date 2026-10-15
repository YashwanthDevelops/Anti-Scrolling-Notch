# Anti-Scrolling-Notch project instructions

@RTK.md

## Product and source

- This is YashwanthDevelops/Anti-Scrolling-Notch, a Windows Codex companion based on Coucou's Windows source. The implementation plan is `docs/implementation-plan.md`; actual progress is `docs/execution-ledger.md`.
- Read the relevant upstream instructions and module code, but follow the user's approved Windows/Codex scope and original product identity. Keep the macOS source as reference unless explicitly assigned work there.
- Preserve MIT attribution and the separate asset-license notice. Keep project/runtime IDs separate from inherited visuals. Initially retain Coucou/Mochi visual assets and animation behavior for development/prototyping as replaceable inherited assets. Before any public/distributable release, perform the explicit asset/license review and obtain appropriate rights or replace affected assets with original ones (REL-11).
- Do not imply that inherited Claude/Coucou source is already implemented for Codex. Mark capabilities and validation truthfully.
- Coucou is the foundation, not a mandatory parity specification. Record inherited features as keep/adapt/defer/remove according to their Codex benefit and release profile. Deferred services/controls must remain inactive.
- The user explicitly wants the existing Coucou UI and animations, including Mochi visuals, retained initially. Keep layout, character expressions/animations, sounds, transitions, timing and interaction behavior for the development prototype; adapt Codex data/backend without redesigning. Treat assets as replaceable resources. New UI work requires a later user request; release asset clearance is a separate gate, not a reason to redesign during development.

## Milestone execution and completion

- The owner's 15 October 2026 speed-first workflow supersedes older one-packet/one-PR, sequential-merge and immediate-push instructions, including historical packet sequencing limits that stop related implementation. Existing consent, credential/privacy and runtime safety boundaries still apply. Use one milestone branch for a substantial working outcome across related stages. Continue dependent implementation on it without intermediate PRs, merges or routine approval rounds.
- Follow the plan's four remaining milestones: M1 usable monitor, M2 monitor beta QA/delivery, M3 interactive v1, M4 final QA/delivery. Use `docs/stage-packet-template.md` once per milestone or record its concise scope directly in the ledger. Do not create separate activation, evidence-capture or closeout PRs. Split only for a concrete review/delivery need.
- Resolve missing Codex evidence within the milestone. Keep unsupported capabilities disabled and record the exact gap; continue independent GitHub, history, notification or cleanup work instead of repeatedly capturing the same failure or writing documentation-only packets. Actual data/contract dependencies still apply. Starting related work does not mark an incomplete task complete.
- Follow Monitor MVP -> interactive v1 -> selected extensions. Stage 9A notifications/recovery belong to MVP and must not depend on Stage 8 managed chat. Optional services/shared desktop/voice/platform extensions do not block core delivery.
- A production capability needs supported-version evidence, fixture/integration tests, tested failure behavior and documented fallback. Enforce this in backend and UI; unknown/unverified capabilities remain disabled. Demo fixtures do not prove production readiness.
- Authority flows through backend adapters -> normalized events -> pure Rust reducer -> snapshot/event sequence -> frontend view store -> UI. No direct frontend Codex/GitHub queries, authoritative state or request-resolution decisions. Local drafts/focus/animations are presentation state; IO belongs outside the reducer.
- A feature requires real backend behavior, failure behavior, persistence/recovery and automated verification. Explain relevant N/A for stateless/presentation-only work; never count a renderer/placeholder as completion.
- Track task/profile/milestone sub-status and repeat Stages 11–12 gates for each release profile/version. The complete core project requires interactive v1 QA and delivery; deferred extensions remain deferred. External MVP signing/asset blockers do not prevent independent interactive development after the monitor foundation is integrated.

## Commits and pushes

- Make coherent local checkpoint commits. Batch accumulated commits into one push at a stable checkpoint or session end; do not push every small commit or push solely to trigger CI. Incomplete work may be backed up on its feature branch with clear status.
- Verify `origin` is `https://github.com/YashwanthDevelops/Anti-Scrolling-Notch.git`. `upstream` is Coucou and is fetch-only. Never publish changes or releases to Coucou.
- Stage only intended files and inspect the staged diff. Keep credentials, local logs, attachment copies and generated build output out of commits.
- Use descriptive messages and update one milestone ledger record with task IDs, outcomes, checks and blockers. Keep documentation/captures/fixes in the owning milestone; do not add hash-only closeout commits or evidence paperwork for every helper.
- Verify the remote branch SHA after push. If a push fails, preserve local commits and report the reason; do not claim upload success.
- No force push, unrelated-history deletion or resetting other work. Preserve any new destination changes with reviewed merges.
- Implement features on milestone branches and open one PR when a substantial user outcome is ready for review. Pass required hosted checks and review the final diff before integration. Documentation-only maintenance may go directly to `main` when branch protection permits; do not create a separate PR by default. Never bypass protection.

## Agents

- The user permits agents when useful. Give each an independent task and clear file/module ownership.
- Only the coordinating agent stages, commits and pushes a shared checkout. Agent reports include changed paths, checks and limitations.
- Use isolated worktrees for independent branch work. Do not run concurrent Git mutations or overwrite another agent's edits.

## Validation and runtime boundaries

- Run the smallest relevant checks after a coherent implementation batch, not after every file edit or commit. Documentation-only changes use consistency/link/diff checks; do not run app builds for prose. Never claim unrun builds/tests passed.
- Reuse passing evidence while relevant code/dependency/config/environment inputs remain valid. Run one combined affected regression/build/Clippy pass at milestone readiness, plus required hosted checks; package/install matrices belong to release candidates or changed packaging behavior. A docs-only follow-up does not invalidate code evidence. Do not repeat full suites or release/debug builds without a concrete reason, and never skip a repository-required check.
- Test new behavior and realistic failure/recovery risks, not implementation details or low-impact cosmetic edits. Record commands/results/versions with the milestone; changed paths plus commit/tool versions suffice for ordinary checks. Diagnose failures before repeating; allow one justified transient retry, then change inputs or resume independent work with the blocker recorded.
- Keep observing hooks fast and neutral. Approvals need an explicit user decision, exact request routing and bounded fallback. Respect Codex's hook review/trust flow.
- Existing-session monitoring and companion-managed App Server control have different capabilities. Do not invent unsupported desktop APIs or deep links.
- Keep service secrets backend-only and OS-protected. Use typed commands/argument arrays, redacted diagnostics and bounded state/history.
- Inspect inherited release destinations and complete runtime identity plus REL-11 asset rights/replacement clearance before public release tags, downloadable artifacts or publishers are used.
