# Stage work packet template

Use this template for one bounded stage/PR assignment from the master plan. Save a filled packet under `docs/work-packets/` when execution starts. This template is not an active implementation assignment.

## Packet identity and scope

| Field | Required value |
|---|---|
| Packet | Stage number, sequential packet number and short name |
| Release profile | Monitor MVP / interactive v1 / named extension |
| Task IDs | Exact master-plan IDs and sub-scope when a task spans profiles |
| Branch / PR | Short branch and PR against Anti-Scrolling-Notch `main` |
| Dependencies | Previously integrated packets and verified capability evidence |
| User outcome | One concrete behavior this packet delivers |
| In scope | Specific behavior, files/modules and schema/config changes |
| Excluded | Adjacent stages/features, optional services and unsupported controls |
| Validation selection | Smallest meaningful checks, their input/version fingerprints, reusable prior evidence and triggers for broader checks |
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
| Automated verification | Meaningful passing tests/build/contract checks and fixture references; docs-only packets use link/consistency/diff checks |
| Manual/platform checks | Relevant actual Windows focus/DPI/notification/install behavior, with tested versions |
| Release limitations | Disabled/deferred capabilities, retained risks and exact profile scope |

Do not create meaningless tests for cosmetic or documentation changes. Use relevant interaction/accessibility or document checks and explicitly state why backend/persistence evidence is not applicable. Do not claim a build or runtime check that was not run.

Follow the master plan's validation policy: affected checks during development, broader checks for changed boundaries and the actual release package. Preserve Coucou UI/motion against the small Stage 2 reference when affected. Reuse prior passing evidence only while its relevant inputs/versions remain valid; diagnose failures before repeating identical commands. Do not retest unrelated subsystems after required checks pass.

## PR and completion gate

- Inspect status/remotes and preserve unrelated changes before starting.
- Keep one active packet/implementation PR. Split a large stage into sequential independently reviewable packets.
- Make coherent verified commits and push each completed commit to `origin`; only the coordinator stages/commits/pushes.
- Update the execution ledger with task/profile sub-status, changed files, checks, limitations and publication evidence.
- Link this packet in the PR. Report behavior/failure/recovery/tests and verify remote commit SHA.
- Review the final diff, pass required checks and integrate the bounded PR before starting the next packet.
- Keep the stage pending until all its required in-scope packet gates pass. QA and delivery still require Stages 11–12 for the exact release profile/version.

Next planned packet: Stage 2 Windows baseline (`BASE-01` through `BASE-07`) on `work/windows-baseline`. It excludes app feature implementation, Codex hook installation, service credentials and product release publication.
