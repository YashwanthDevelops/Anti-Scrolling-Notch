# Anti-Scrolling-Notch execution ledger

Client date: 1 October 2026 (Asia/Calcutta).

This ledger records verified progress. Source/documentation import is not an application build or Codex implementation.

| Task | Status | Evidence | Commit / publication |
|---|---|---|---|
| SETUP-01 | Complete | GitHub repository exists; `git ls-remote` returned no refs before import. | Destination: YashwanthDevelops/Anti-Scrolling-Notch. |
| SETUP-02 | Complete | Separate `app` checkout cloned from the audited source with preserved Git history. | Source baseline: 3cc3333203f60f63326ee949b7b86c7549992a1f. |
| SETUP-03 | Complete | origin points to the user's repository; upstream points to Coucou with disabled push URL. | Verified using git remote -v. |
| SETUP-04 | Complete | HEAD matches the exact audited source commit. | Baseline SHA recorded in source-provenance.md. |
| SETUP-05 / SETUP-06 | Complete for initial import | Updated plan, audit, source inventory, attribution and project documentation committed and pushed to main. The first implementation packet will use its own short branch. | `47f52e6ce4ea552bda101b0423752b064dbdd5b8`; local and remote SHA matched. |
| SETUP-07 | Partial | Inherited workflows/scripts inspected; macOS publisher still targets Coucou. Do not use it. | Replacement and release enablement remain future work. |
| SETUP-08 | Complete | Project README, original README reference, AGENTS.md and RTK guidance added; existing output/secret ignores retained. | Included in planning publication. |
| Stages 2–13 | Pending | No dependency installation, application build, Codex hook installation or feature implementation performed. | No claims of build/test success. |

## Checks for the planning publication

- Source SHA/remotes/destination refs inspected.
- Agent independently reviewed repository strategy, history preservation and commit/push rules.
- Plan validated: 13 stages, 117 unique task IDs, no stale repository placeholders, and no broken local links in the five checked project documents.
- Staged diff/whitespace checks passed; changes from the audited source baseline are nine documentation/instruction files only.
- Planning commit `47f52e6ce4ea552bda101b0423752b064dbdd5b8` was pushed to `origin/main`; `git ls-remote` confirmed the same full SHA.
- No application code tests are necessary for documentation-only edits; the Windows baseline build remains Stage 2.

## Published checkpoint — 1 October 2026

[Planning import commit](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/commit/47f52e6ce4ea552bda101b0423752b064dbdd5b8) preserves the full Coucou source history and publishes this project's plan. This subsequent ledger update records that verified checkpoint; its own publication is verified after commit and reported through Git history/session output.

Next: create a Stage 2 packet on `work/windows-baseline`, perform BASE-01 through BASE-07, then test the MVP Codex compatibility contract. Dependency installation, local Windows build, Codex hook installation, app rebranding and feature changes have not started. Inherited automation may run on GitHub after a push; this ledger makes no claim about its result.

## Plan revision — 1 October 2026

Prior verified published checkpoint: `60b063f5f184bba236af58b3c0290e0ca3d016ef` on `origin/main` (fetched and local/remote divergence checked before this update).

This documentation-only packet on `docs/release-scope-and-gates` incorporates the user's pasted feedback:

- Retain the 13-stage master plan; execute one bounded stage/PR at a time using the new packet template.
- Split monitor MVP, interactive v1 and selected extensions. Move Stage 9A notifications/observer recovery onto the MVP path without a managed-chat dependency.
- Make supported-version evidence, fixture/integration tests, tested failures and documented fallback a hard production capability gate.
- Establish backend adapters -> normalized events -> pure Rust reducer -> snapshot/sequence -> frontend view store -> UI authority.
- Require backend behavior, failure handling, persistence/recovery and automated verification for completion; track task/profile sub-status.
- Use the audit as a complete reference while recording inherited features as keep/adapt/defer/remove; do not require unrelated Coucou parity.
- Repeat QA/delivery Stages 11–12 per release profile/version; interactive v1 completes the core project.

Changed files: implementation plan, stage packet template, project README, AGENTS.md and this ledger. Application sources, assets, dependencies and build workflows remain untouched. The root workspace plan is a synchronized convenience mirror; all implementation stages remain pending.

Revision checks: 13 master stages, 120 unique task IDs, 13 valid local links across the five changed documents, paired code fences and required scope/gate sections verified. A read-only agent reviewed all seven feedback items and identified one unconditional review-monitoring gate; it was changed to require CI for MVP and reviews only when in scope for v1. Whitespace/staged-file checks passed before publication. No local application builds/runtime tests were run for this documentation-only change.

Published revision commit: [`c1f42f3c28a253a4b9fca434fb095e60381c2e3e`](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/commit/c1f42f3c28a253a4b9fca434fb095e60381c2e3e) on `origin/docs/release-scope-and-gates`; `git ls-remote` confirmed the same full SHA. [PR #1](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/pull/1) contains this bounded documentation packet and the validation report. At this checkpoint the inherited macOS Build workflow was running; no Windows CI result or merged status is claimed. This subsequent ledger commit records the verified publication without embedding its own hash. Final PR/merge state is verified separately in GitHub/session output.

## Rules for later entries

Record task IDs, release profile/version, packet, changed files, relevant test results, remaining limitations, branch/commit and push result for every completed milestone. Record a commit hash in the next progress entry rather than trying to embed a commit's own hash into its contents. Split status for tasks spanning profiles and record QA/REL evidence per release. Deferred/not-applicable work needs a reason and must not be marked implemented. Never mark a task complete solely because its source renderer or placeholder exists.
