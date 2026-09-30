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

## UI fidelity and efficient validation clarification — 1 October 2026

Documentation packet: `docs/ui-fidelity-and-validation`. Prior merged planning checkpoint: `046d0e9008f7b38b81326aef7d3a777c9b2dec0b` (PR #1).

The user's preference is now explicit: preserve Coucou's island layout, generic animations/transitions and interaction timing while adapting its backend/data to Codex. Exact protected character expressions/animations/icons/sounds are conditional on recorded written permission for a distributed derivative; the asset-license fallback is original replacements without redesigning the surrounding UI. No asset permission is currently recorded.

The plan adds affected-check selection, reusable evidence with code/config/dependency/environment fingerprints, bounded transient retries and diagnosis before rerunning unchanged failures. Broader validation remains required for affected integration boundaries and the actual release package. Stage 2 will add path-scoped/cached Windows CI; inherited workflow files are unchanged by this planning update.

Scope clarification: every audited Coucou feature is accounted for, but service extensions and unverified existing-desktop controls are not promised in the core release. CLI read-only checks found `codex-cli 0.157.1` and App Server tooling; no server, chat, compatibility probe or hook was started/installed. No application build or feature implementation has begun.

Documentation validation passed: 13 master stages, 120 unique task IDs, 13 valid local links, paired code fences, synchronized root mirror and intended-file/whitespace checks. A read-only agent found no scope, asset-permission or evidence-reuse contradictions in the five-file diff. Publication is verified through remote SHA and the PR/session record rather than embedding this commit's own hash.

## Retained Coucou prototype direction — 1 October 2026

Documentation packet: `docs/inherited-ui-prototype`. Prior merged checkpoint: `9abeed3474cdc514de0b8088398bef42e61cc8ff` (PR #2).

The user's latest instruction explicitly supersedes any development-time asset replacement/redesign requirement: initially retain Coucou/Mochi visual assets and animation behavior for development/prototyping as replaceable inherited resources. Preserve the existing UI, character expressions/animations, sounds, layout and timing while integrating Codex. A new UI can be considered later only at the user's request.

SHELL-02 now uses the supplied development-retention wording. A separate REL-11 gate requires explicit asset/license inventory and appropriate rights or original replacements before any public/distributable release, including beta installers, updates and downloadable CI artifacts. No rights are assumed or recorded by this update. Development-stage completion does not depend on producing original art or redesigning the UI.

Updated documents: implementation plan, README, AGENTS.md, packet template, source provenance and this ledger; root plan mirror synchronized. Application code, visuals/assets, licenses and workflows remain unchanged; implementation has not started. Task count becomes 121 with REL-11; the 13 master stages and phased feature scope are unchanged.

Documentation checks passed: 13 stages, 121 unique task IDs, 13 valid local links, paired code fences, exact supplied wording and intended-file/whitespace checks. A read-only agent approved the six-file diff with no development/release-gate contradictions. No application build or runtime test was run for this documentation update. Publication is verified by remote SHA and the PR/session record.

## Stage 2 Windows baseline packet — partial checkpoint, 1 October 2026

Packet: `docs/work-packets/stage-2-windows-baseline.md` on `work/windows-baseline`; monitor MVP foundation. Scope is BASE-01 through BASE-07 only. Coucou application sources, artwork, animations, sounds and runtime code were not modified.

BASE-01 and BASE-03 are complete. Node v22.23.2/npm 10.9.8 were installed in an isolated user-scoped location and `npm ci` passed in 7.5 seconds with 18 packages and 0 vulnerabilities. Rust stable MSVC 1.98.1/Cargo 1.98.1 were installed in the user profile. The frontend TypeScript check and Vite bundle passed in 2.9 seconds. The exact commands, versions and archive checksums are in `docs/development-baseline.md`.

BASE-02 remains blocked: official Visual Studio Build Tools 2022 v17.14.41 was invoked with the C++ workload, x64 MSVC tools and Windows SDK 26100; the standard Windows elevation flow ended with installer code 1602. `link.exe`, `cl.exe`, `vswhere.exe` and Windows SDK headers remain absent. One `cargo test --workspace --locked` run resolved dependencies and then stopped at `link.exe not found`; no Rust tests executed. `cargo fmt --all -- --check` identifies inherited formatting drift in 11 Rust files. No source reformat was applied. This is a toolchain prerequisite blocker, not a test assertion failure.

Added `.github/workflows/windows-ci.yml` with affected-path selection, separate frontend and locked Rust-test jobs, Node 22 and Cargo lockfile caches, docs-only checks, and PR-scoped stale-run cancellation. Added `docs/check-markdown.ps1` for changed-document link/fence checks. Local whitespace and changed-document validation passed. An initial whole-repository diagnostic also found an existing broken link in untouched `docs/upstream-README.md` to `windows/README.md`; the workflow checks only changed documents so this inherited source-reference issue does not block unrelated documentation edits. BASE-04, BASE-05 and BASE-06 remain partial/pending: native app behavior and the local installer were not verified. BASE-07 awaits workflow syntax inspection and hosted pull-request results. Stage 2 remains pending; Stage 3 was not started.

Changed files: implementation-plan task checkboxes for BASE-01/03, the filled packet, this ledger, the Windows baseline, the scoped CI workflow and its document checker. Local `git diff --check` and changed-document link/fence validation passed; workflow syntax inspection and a hosted PR run are required before the checkpoint can be integrated. Commit/push and remote verification are recorded in the following publication entry.

## Rules for later entries

Record task IDs, release profile/version, packet, changed files, relevant test results, remaining limitations, branch/commit and push result for every completed milestone. Record a commit hash in the next progress entry rather than trying to embed a commit's own hash into its contents. Split status for tasks spanning profiles and record QA/REL evidence per release. Deferred/not-applicable work needs a reason and must not be marked implemented. Never mark a task complete solely because its source renderer or placeholder exists.
