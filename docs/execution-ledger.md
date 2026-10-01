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

Added .github/workflows/windows-ci.yml with affected-path selection, separate frontend and locked Rust-test jobs, Node 22 and Cargo lockfile caches, docs-only checks, and PR-scoped stale-run cancellation. Added docs/check-markdown.ps1 for changed-document link/fence checks. The first hosted run parsed the workflow and passed scope selection plus the frontend job. It exposed two fixable integration issues: the docs script received the PR flag as a string but required a Boolean, and Tauri's build script expected the release hook resource before workspace tests. The docs parameter now compares a string and the Rust job builds coucou-hook in release mode before tests. PR #4 Windows CI run 2 at commit 9e56fe89b4feebc9df5ca4c5c80da1662f5e9e2c passed scope selection, docs checks (3.0 seconds), frontend checks (33.5 seconds) and locked Rust workspace tests (269.5 seconds). The inherited Build workflow also passed. BASE-04 and BASE-07 are complete with failure baseline and timings recorded. Local whitespace and changed-document validation passed. An initial whole-repository diagnostic also found an existing broken link in untouched docs/upstream-README.md to windows/README.md; the workflow checks only changed documents so this inherited source-reference issue does not block unrelated documentation edits. BASE-02, BASE-05 and BASE-06 remain pending: local C++/SDK setup, native app behavior and the local installer were not verified. Stage 2 remains pending; Stage 3 was not started.

Changed files: implementation-plan task checkboxes for BASE-01/03/04/07, the filled packet, this ledger, the Windows baseline, the scoped CI workflow and its document checker. Local git diff --check and changed-document link/fence validation passed; the final hosted Windows CI run and inherited Build workflow passed at the recorded commit.

Publication verification: the tested code commit 9e56fe89b4feebc9df5ca4c5c80da1662f5e9e2c and status commit eea0471b17d45d64ed98c2bf1162d62217b50159 are pushed to origin/work/windows-baseline; `git ls-remote` matches the current local SHA. Draft PR #4 targets `main`. A workstation recheck on 1 October 2026 confirmed the isolated Node v22.23.2 binary remains available, while `cl.exe`, `link.exe`, `vswhere.exe`, Windows SDK headers and `makensis.exe` are still absent. BASE-02, BASE-05 and BASE-06 remain pending; Stage 2 cannot be integrated until the administrator-approved C++/SDK setup and local app/installer checks complete.

## Rules for later entries

Record task IDs, release profile/version, packet, changed files, relevant test results, remaining limitations, branch/commit and push result for every completed milestone. Record a commit hash in the next progress entry rather than trying to embed a commit's own hash into its contents. Split status for tasks spanning profiles and record QA/REL evidence per release. Deferred/not-applicable work needs a reason and must not be marked implemented. Never mark a task complete solely because its source renderer or placeholder exists.

## Stage 2 local Windows toolchain and package verification — 1 October 2026

Resumed BASE-01 through BASE-07 on `work/windows-baseline` from prior pushed checkpoint `adfb7f14751cec454e3ebaa60156434ecc8da429`. The user confirmed Visual Studio 2026 installation and supplied Developer Command Prompt output; independent checks verified VS Community 2026 18.10.3, MSVC tools 14.51.36231 (compiler/linker 19.51.36260/14.51.36260), Windows SDK 10.0.26100.0 headers/libraries, `vswhere`, and successful x64 environment initialization. This supersedes the earlier 2022 installer attempt that returned 1602.

BASE-02, BASE-04 and BASE-06 now have local evidence. `cargo build --release -p coucou-hook && cargo test --workspace --locked` passed with all 11 Rust tests on Rust/Cargo 1.98.1; TypeScript/Vite production build and `npm run pack` passed on isolated Node v22.23.2/npm 10.9.8; `cargo clippy --workspace --all-targets --locked` passed in 44.72 seconds with three inherited warnings recorded in the baseline. The `npm run tauri dev` process built and launched `windows\\target\\debug\\coucou.exe`. Tauri's pack command downloaded and hash-validated NSIS 3.11 and `nsis_tauri_utils v0.5.3`, then produced two ignored local installer artifacts (4,216,930 bytes each, SHA-256 `7F3B0A22CDDE63B59F597B5FB8E831E28670FF2F7FC64ACF80F40FD15C0549B3`). The installer was not run; these are not distributable artifacts.

BASE-05 remains incomplete: process startup does not verify tray behavior, hidden/compact/expanded/greeting states, settings, sound, startup toggle, file drop or the fixed-DPI visual/motion reference. The supported computer-use helper's documented `@oai/sky` initialization and bounded recovery returned “failed to write kernel assets: The system cannot find the path specified.” No alternate UI automation route was used. No hooks, Codex configuration or service credentials were installed.

For BASE-07, `.github/workflows/windows-ci.yml` pins Node v22.23.2/npm 10.9.8 and Rust 1.98.1, prints/asserts these versions, and scopes Cargo cache keys by Rust version. Exact pinned-run CI passed on the pushed commit, with durations and links recorded in the next ledger entry. This checkpoint's changed files are the CI workflow, implementation plan, development baseline, Stage 2 packet and this ledger; the root convenience plan mirror has its Stage 2 task checkboxes synchronized. Local frontend/native/package/test and Clippy results above were collected before these workflow/documentation-only edits; changed-document validation passed locally and hosted checks passed for the exact pinned configuration. Stage 3 remains prohibited while BASE-05 is incomplete.

The commit SHA and push verification for this checkpoint are recorded in a subsequent ledger entry after publication, avoiding a self-referential hash.

## Stage 2 pinned CI and publication verification — 1 October 2026

Commit [`74ab268bab64760ca1aa279ed40c2617f2989643`](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/commit/74ab268bab64760ca1aa279ed40c2617f2989643) was pushed to `origin/work/windows-baseline`; `git ls-remote` matched the local HEAD. Draft PR #4 remains open against `main`.

The exact pinned configuration passed [Windows CI run 5](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/actions/runs/36794227971): scope 5 seconds, docs 7 seconds, frontend 29 seconds, and Rust tests 265 seconds including setup/cache and the hook prebuild. Node v22.23.2/npm 10.9.8 and Rust/Cargo 1.98.1 version assertions passed. The inherited [Build run 14](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/actions/runs/36794227965) passed in 105 seconds. BASE-07 is complete. Stage 2 still cannot close because BASE-05's actual UI-state/interactions and fixed-DPI capture remain unverified after the supported UI helper failed initialization. Stage 3 has not started.
