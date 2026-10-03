# Stage 5 packet 13 — broker-backed integration health

| Field | Required value |
|---|---|
| Packet | Stage 5, packet 13: CORE-10 integration-health projection |
| Release profile | Monitor MVP state authority foundation |
| Task IDs | CORE-10 — existing integration-health consumer slice |
| Branch / PR | `work/stage-5-core-10-integration-view` / [PR #28](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/pull/28), merged at `230b5ba98ebb866a240ec0093804da7c33ce4fae` |
| Dependencies | CORE-03 and packet 12 integrated at `9974a28d95b3e17ef5b7f8c60c9465e146833299`; existing broker `Integration` and poller result contracts |
| User outcome | Current integration connection health comes from the sequenced backend broker snapshot/replay store while existing integration detail cards keep their current content and behavior |
| In scope | Project the existing integration poller result into the existing typed Rust `Integration` health record; apply/publish it via `BrokerService`; render health through `BrokerView`; test the projection and snapshot/replay path; packet/plan/ledger updates |
| UI/assets contract | Keep existing card layout, provider data details, sound, badge, and animation behavior unchanged |
| Excluded | Storing arbitrary API response JSON or free-form transport errors in the broker; Claude hook task state; Codex session producers or new hook schemas; Stage 6 monitor; SHELL-08/09; capability changes; UI redesign |
| Validation selection | Focused Rust integration-health projection and broker tests; view-store tests; locked Rust workspace, Clippy, scoped formatting, frontend tests/typecheck/build, documentation checks, and diff review. |
| Ownership | Coordinator owns implementation, validation, publication, and integration |

## Capability and architecture contract

No Codex or GitHub capability is added. Existing server-side integration pollers remain the only producers, and only the existing typed `Integration` contract enters `BrokerService`. Identity remains the existing integration key; no Codex session, event, or request ID is fabricated. The current provider result event remains available for presentation details and notification behavior but does not become broker state.

The broker record contains only provider, unknown configuration, connection health, typed coarse error kind, and unread event identifiers. Last-success, data-revision and retry timestamps remain null because these pollers do not retain those source values; the broker envelope still records the observation time. The record does not contain API response JSON, credentials, or free-form errors. If a poller identifier is unknown, the broker projection fails closed while the legacy result event continues.

## Acceptance evidence

| Dimension | Evidence |
|---|---|
| Real behavior | The actual Rust poller emission path applies a typed health update to `BrokerService` and publishes its existing sequenced snapshot/replay response; the existing `integration` event still carries current presentation data. |
| Failure behavior | Error results project to degraded health with only a typed coarse error kind; unknown IDs do not produce broker records; a broker publish failure does not suppress the existing presentation event. |
| Persistence/recovery | The process-local snapshot restores current health after WebView reload and replay repairs missed updates; persistent provider response history is out of scope. |
| Automated verification | Rust projection/service tests and frontend view-store snapshot/replay tests; locked workspace tests, Clippy, frontend checks/build, docs and diff checks. |
| Manual/platform checks | No new UI interaction or provider behavior is introduced; runtime requests are not issued for verification. |
| Release limitations | Per-provider API payload details remain transient legacy presentation state. Claude hook state and Codex session/task projection remain outside this packet; no production Codex monitoring is claimed. |

## Implementation and validation status

Implemented the health projection in the existing poller emission path and rendered broker health through the overview cards. The legacy integration event and local provider-detail data remain intact. A failed broker publish is logged with only the integration ID and does not suppress the legacy event; its error remains visible while broker health is stale. Unknown provider IDs do not create broker records.

Local validation passed: workspace tests (98 app tests passed, 2 installed-app tests ignored; hook 7, identity 2, contract 6 passed), Clippy, locked debug build, view-store tests (7), shell tests (12), typed-bridge tests (5), capability tests (5), runtime identity, resource tests (2), TypeScript typecheck, Vite production build, changed-document link/fence checks (3 Markdown files), and `git diff --check`. Clippy reports only the pre-existing `hooks.rs:467` warning. `cargo fmt --all -- --check` reports pre-existing formatting drift in legacy files and untouched portions of `integrations.rs`; the new projection and test regions were separately confirmed identical to rustfmt output, and no unrelated formatting changes were applied. Hosted CI, PR review, and merge evidence will be recorded after publication.
