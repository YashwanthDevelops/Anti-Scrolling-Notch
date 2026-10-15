# Milestone scope template

From 15 October 2026, use this once per working milestone, which may span related stages. The legacy filename is retained for existing links. Save one record under `docs/work-packets/` or put these concise notes directly in the execution ledger; update it in place. Small tasks, captures and fixes need no separate packet, PR or merge gate. This template does not itself start implementation.

## Milestone identity and scope

| Field | Required value |
|---|---|
| Milestone | M1 monitor / M2 beta delivery / M3 interactive v1 / M4 final delivery, or named extension |
| Release profile | Monitor MVP / interactive v1 / named extension |
| Task IDs | Exact master-plan IDs and sub-scope when a task spans profiles |
| Branch / PR | One milestone branch; PR only when the substantial working outcome is reviewable |
| Dependencies | Actual code/data/contract prerequisites and missing capability evidence; related work may share the branch |
| User outcome | Concrete end-to-end behavior the milestone delivers |
| In scope | Specific behavior, files/modules and schema/config changes |
| UI/assets contract | Retain existing Coucou UI/Mochi visuals/animations for development; isolate replaceable assets; redesign only when later requested |
| Excluded / blocked | Deferred features, disabled controls and exact missing evidence; name independent work to continue |
| Validation selection | Affected fast checks during development, combined milestone checks, reusable results and release-only checks |
| Ownership | Coordinator and any agents' non-overlapping files/read-only review tasks |

## Capability and architecture contract

For each exposed capability, link the tested Codex version/surface, sanitized fixture and automated test, real-adapter integration result, failure behavior and documented fallback. Missing evidence keeps it disabled. For non-Codex functionality record the tested OS/API contract or explain why the capability gate does not apply.

State the adapter event/intent types and how they follow backend adapter -> normalized event -> Rust reducer -> snapshot/sequence -> frontend view store. The frontend must not query Codex/GitHub, own authoritative state or decide request resolution. Keep reducer logic pure and IO in bounded effect workers.

## Acceptance evidence

| Dimension | Required evidence for each in-scope feature |
|---|---|
| Real behavior | Actual backend/adapter outcome and visible user result; no mock-only success |
| Failure behavior | Applicable denial, invalid input, timeout, disconnect, cancellation and stale request cases; user-facing fallback |
| Persistence/recovery | Applicable reload, restart, sleep, duplicate/out-of-order event and schema migration behavior; justify any N/A |
| Automated verification | Meaningful affected tests/build/contract results and fixture references; docs-only edits use link/consistency/diff checks |
| Manual/platform checks | Relevant actual Windows focus/DPI/notification/install behavior, with tested versions |
| Release limitations | Disabled/deferred capabilities, retained risks and exact profile scope |

Do not create meaningless tests for cosmetic or documentation changes. Use relevant interaction/accessibility or document checks and explicitly state why backend/persistence evidence is not applicable. Do not claim a build or runtime check that was not run.

Follow the master plan's validation policy: run affected checks after a coherent implementation batch, one combined regression/build pass at milestone readiness, and package/install matrices for the release candidate or changed packaging behavior. Preserve the retained Coucou UI/assets/animation behavior when affected. Record commands, results, relevant commit/changed paths and tool/runtime versions together; ordinary changes need no separate fingerprint document. Reuse passing evidence while relevant inputs remain unchanged. Docs-only follow-ups do not invalidate code checks. Diagnose before retrying; do not retest unrelated subsystems.

Development retains inherited visual resources as replaceable assets; original asset creation is not an integration prerequisite. Every public/distributable release must complete REL-11 asset/license review and record appropriate rights or verified replacements before distributing installers, updates or downloadable CI artifacts.

## Checkpoints and completion

- Inspect status/remotes and preserve unrelated changes before starting.
- Continue related stages and implementation slices on the milestone branch without intermediate PR/merge gates. Split only for a concrete review/delivery need.
- Make coherent local checkpoint commits and batch pushes at a stable checkpoint or session end; only the coordinator stages/commits/pushes. Clearly label an incomplete backup on the feature branch.
- Update the milestone's existing ledger record with task/profile sub-status, checks and blockers. No separate activation/capture/closeout PRs or hash-only commits.
- Open one PR for the substantial working outcome, linking the scope record. Review the final diff and pass required hosted checks before integration; verify the remote SHA after pushing. Docs-only maintenance may go directly to `main` if protection permits.
- Keep incomplete tasks pending even when their partial implementation merges. Missing evidence blocks that capability, not unrelated work. QA and delivery still require Stages 11–12 for the exact release profile/version.
