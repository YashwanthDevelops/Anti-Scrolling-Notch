# Stage 3 packet 4 — release-profile compatibility matrix

| Field | Value |
|---|---|
| Packet | Stage 3, packet 4: compatibility evidence and release-profile map |
| Release profile | Monitor MVP evidence, with explicit v1 and extension dispositions |
| Task IDs | COMPAT-09 |
| Branch / PR | `work/stage-3-compatibility-matrix` → Anti-Scrolling-Notch `main` |
| Dependencies | Stage 3 packets 1–3 are integrated; accepted four-event hook contract, generic Codex launch route and bounded Windows probe results are recorded |
| User outcome | A reviewer can see which surfaces are supported, disabled, unverified, deferred to v1 or optional, and what evidence/fallback applies to each. |
| In scope | Consolidate existing sanitized fixtures and runtime/source evidence in `docs/codex-compatibility.md`; map the plan's MVP, interactive v1 and optional-extension capabilities; explicitly mark missing evidence, tests and protocol versions without guessing; reconcile COMPAT-09 and ledger status. |
| UI/assets contract | Documentation and existing fixture references only; no UI, Coucou/Mochi assets, animation, sound or product behavior changes. |
| Excluded | New runtime probes or provider investigation; expanding the accepted hook schema; COMPAT-10 registry enforcement; Stage 4 identity work; Stage 5 reducer/storage; Stage 6 monitor UI; Stage 7 GitHub implementation; Stage 8 managed sessions; Stages 9–13. |
| Validation selection | Changed-document link/fence and whitespace checks. Reuse the already integrated adapter, navigation and Windows test evidence; do not rerun unchanged probes or claim documentation as new runtime evidence. |
| Ownership | Coordinator owns documentation, review, staging, commit, push and PR. |

## Capability and architecture contract

The matrix is an evidence index, not an enablement source. It must preserve the distinction between a documented API, a sanitized fixture, an automated replay, a live adapter probe and an owner-reported/manual result. Only four hook event names were captured at the Codex CLI boundary; the fixture is a sanitized projection and is not a complete upstream JSON schema. Do not add guessed event fields or promote an unobserved event based on documentation.

Every selected or candidate surface is assigned to Monitor MVP, interactive v1 or an optional extension. A row without a verified version/surface, fixture or runnable test, failure behavior and fallback remains disabled. Future work references the master task that owns its probe; this packet does not perform those probes or expose controls.

## Acceptance evidence

- `docs/codex-compatibility.md` maps the observed hook subset, unsupported hooks, Codex app activation, Windows notification/DPI/drop routes, managed App Server, permission route, shared-desktop attachment, Git/GitHub workflow and unrelated optional integrations to their intended release profile.
- Each row records the tested version/surface when evidence exists; otherwise it explicitly says unprobed or runtime-unverified. Each row links the existing sanitized fixture/test or states that none exists yet, records the tested failure behavior/fallback or its pending status, and states whether the capability is verified, limited, disabled or deferred.
- The profile capability table matches the Stage 3 deliverable matrix for lifecycle/activity, streamed output, send/steer/interrupt, permission decisions, questions, grants and attachments. Hook-only limits remain distinct from App Server and optional desktop attachment.
- Existing fixture provenance and privacy limits are explicit. The four captured records remain the only claimed runtime event contract. Runtime-unverified Windows observations are not converted into successful evidence.
- The task and ledger are updated only after these checks pass. COMPAT-01–03 partial hook work and COMPAT-10 remain pending; this packet must not mark Stage 3 complete.
- Run the repository's changed-Markdown check against the PR base/head range and `git diff --check`.

## Packet completion gate

Close COMPAT-09 when the version/surface/evidence/failure/fallback matrix explicitly distinguishes MVP, v1 and extensions, cites all existing sanitized fixtures and runnable checks, and leaves missing evidence visibly disabled with its planned gate. This packet does not satisfy the production registry or authorize any later stage.

## Results

COMPAT-09 is complete as a documentation/evidence-index task. The matrix now has explicit release-profile scope and separate rows for the observed CLI hook subset, unobserved hooks, generic Codex activation, Windows toast/DPI/drop, managed App Server, permission decisions, Git/GitHub workflow, shared-desktop attachment and optional services. The Stage 3 user-capability table follows the master plan's lifecycle, streaming, control, permission, question, grant and attachment categories.

The only committed runtime hook fixture remains the four sanitized loopback projections. Existing contract, pipe-replay, navigation and direct file-ingest checks are linked with their limits; no guessed protocol schema or new runtime capability was added. COMPAT-01–03 remain incomplete for unverified event/surface coverage, COMPAT-10 remains pending, and no later stage was started.

Validation passed after the document edits: `git diff --check`; `pwsh -NoProfile -File docs/check-markdown.ps1` checked four changed Markdown files for local links and code fences. CI and publication evidence are tracked by this packet's PR.
