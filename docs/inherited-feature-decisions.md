# Inherited feature decisions

This is the disposition of every catalog item F01–F124 in [the Coucou source audit](coucou-analysis.md). “Keep” retains the behavior or purpose, not a claim that the inherited implementation is production-ready. “Adapt” replaces its implementation or meaning for Codex. “Defer” means outside the current MVP unless a row says otherwise. “Remove” means remove the inherited control/claim from the applicable profile; it does not claim every dormant source module has already been deleted.

The current shell is a development prototype under SHELL-02/SHELL-07: retain the existing UI, Mochi appearance, timing, layout, animations, and sounds while integrating the Codex backend. This retention is not a distribution license. REL-11 must document rights or completed original replacements for all affected inherited names, artwork, character animation, icons, sounds, and media before any public or distributable artifact. No rights are asserted here. MVP/v1/extension names refer to the profiles in `implementation-plan.md`.

## Shell, lifecycle, and visual system

| Catalog IDs | Decision | Codex benefit / disposition | Profile and task IDs |
|---|---|---|---|
| F01–F06 | Keep | Preserve the resident island, placement, display behavior, topmost surface, click-through, and wake affordance as the desktop entry point. | MVP; SHELL-04/05 |
| F07–F10 | Keep | Keep compact/expanded transitions and collapse timing; protect active typing, drag, and actionable states. | MVP; SHELL-06/07 |
| F11–F14 | Adapt | Keep the greeting, motion language, silhouette, and bounded geometry in the retained prototype; long session/PR details belong in the detail surface. | MVP; SHELL-07/08; REL-11 |
| F15 | Adapt | Keep navigation but route unsupported Ask/chat to generic Open Codex; do not imply control of a monitored conversation. | MVP fallback, interactive v1 chat; SHELL-08, MON-07, PARITY-13 |
| F16–F18 | Adapt | Replace the shared Claude/service coding pill with independently selectable Codex sessions and separately pinned integrations; remove the four-session ceiling. | MVP; MON-05, SHELL-08 |
| F19 | Keep | Give a truthful empty state and a generic Codex entry point when no observed sessions exist. | MVP; MON-03/05 |
| F20–F23 | Adapt | Keep keyboard, tray, and pause semantics; pause presentation separately from observer/network state and never discard an actionable request. | MVP; SHELL-05/06, STATUS-01/02 |
| F24–F28 | Keep | Keep validated preferences, startup choice, two-level settings, quick settings, and concise status notes; show backend evidence rather than credential-presence as health. | MVP; SHELL-07, MON-03/04, STATUS-05 |
| F29–F43 | Keep | Retain the current character, expressions, motion, particles, sound vocabulary, and hidden-render suspension in the development UI prototype. Keep feedback accessible and responsive to reduced-motion settings. | Development/MVP shell; SHELL-02/07; REL-11 before distribution |

## Codex observation, session state, and interaction

| Catalog IDs | Decision | Codex benefit / disposition | Profile and task IDs |
|---|---|---|---|
| F44 | Adapt | Replace Claude event assumptions with only version-matched, actually observed Codex hook events; keep unsupported event families disabled. | MVP observed subset; MON-01/02, MON-06 |
| F45 | Adapt | Use backend session/worktree identity where known; show no fabricated project label when it is unknown. | MVP; MON-05, GH-01/02 |
| F46–F48 | Adapt | Keep concise activity feedback and ticker interaction, but source it from broker-owned turns with bounded history and monotonic identity. Do not expose prompt/tool content absent verified fields. | MVP subset; CORE-05/06/10, MON-05/06/07, SHELL-08 |
| F49–F52 | Adapt | Separate tool failure, turn outcome, completion, and session end; never imply tests passed. End or reset only the exact observed session/turn. | MVP verified fields; MON-05/06/07, CORE-05/06 |
| F53 | Defer | Independent child-agent hierarchy is not evidenced by the current observer contract; no subagent labels or relationships until IDs and parent links are verified. | Extension; MON-01/05/06 |
| F54–F58 | Defer | Approval cards, Allow/Deny/Always, queues, and reply fallback need a verified Codex request transport and exact response semantics. Keep these controls unavailable and use Open Codex. | Interactive v1 only after capability gate; CORE-07/08, MON-06, STATUS-01, PARITY-13 |
| F59–F60 | Adapt | Report observed hook/session evidence only. Keep generic Open Codex; exact terminal or editor tab navigation remains a separately verified capability. | MVP fallback; MON-07, STATUS-08 |
| F61–F62 | Adapt | Keep local IPC with explicit schema, identity, size, timeout, and privacy constraints for Codex inputs. | MVP; CORE-09, MON-01 |
| F63–F65 | Adapt | Reuse safe preview/backup/ownership ideas for Codex hook installation; show configured, trust-unknown, connected, and last-event separately. | MVP; MON-02/03/04, STATUS-05 |
| F66 | Adapt | Backend broker snapshot/history is authoritative; view state is a projection. On restart, the backend replays bounded normalized Codex history and reports recovery scope/count/partial/capacity status; it marks restored activity, connection and waiting as unknown rather than reestablishing live state. Completeness is still partial. | MVP partial; CORE-03/04/10, MON-05/07/08, SHELL-08 |

## Chat, files, context, and optional actions

| Catalog IDs | Decision | Codex benefit / disposition | Profile and task IDs |
|---|---|---|---|
| F67–F72 | Defer | Companion-owned chat is distinct from monitoring and requires its own App Server transport, session identity, stream, and capability UI. Remove/disable inherited Claude chat controls in MVP; Open Codex remains available. | Interactive v1; Stage 8, MON-07, PARITY-13 |
| F73–F81 | Defer | File staging and attachment affordances should return only with real supported Codex inputs, truthful preparation progress, preview/removal, and retention rules. Do not present the legacy first-file workflow as active. | Interactive v1; PARITY-01/02/03, SHELL-07 |
| F82–F84 | Defer | External window metadata, highlighting, and automatic context require explicit consent and a verified Windows adapter; no hidden context capture. | Optional extension; PARITY-04 |
| F85 | Defer | Structured search is not a reachable inherited workflow. Keep its dormant controls out of MVP; consider only as a separate evidence-backed feature. | Optional extension; PARITY-13 |
| F86–F89 | Defer | Email compose/send is not part of session monitoring. Any later share flow must distinguish opening a draft from accepted delivery. | Optional extension; PARITY-08/11/12 |
| F90 | Defer | Voice input has no implemented pipeline in the audit. | Optional extension; separate future task |

## GitHub and other services

| Catalog IDs | Decision | Codex benefit / disposition | Profile and task IDs |
|---|---|---|---|
| F91 | Adapt | Replace account-stat-only emphasis with an explicitly connected local-repository PR/current-head checks card. Keep repository association separate from Codex session identity unless verified. | MVP; GH-01/02/03/04/05/09, SHELL-08 |
| F92–F108 | Defer | n8n, Vercel, Resend, Stripe, Notion, Cal.com, service filters, and unrelated integration pills are outside Codex session/PR MVP. Do not expose dormant controls as working; revisit individual service only when selected. | Optional extensions; PARITY-05–12 |
| F109–F110 | Adapt | Use a backend-owned, opt-in GitHub refresh policy with truthful stale/offline health. Deferred services must not start requests. | MVP GitHub only; GH-03/06/07, PARITY-12, STATUS-05 |
| F111 | Adapt | Keep secrets backend-only and use explicit existing `gh` authentication for the GitHub workflow. Do not equate saved credentials with authenticated health. | MVP GitHub; GH-03, STATUS-05 |
| F112 | Keep | Preserve local-first operation and bounded local history; no account/cloud-sync dependency for monitoring. | MVP; CORE-04, GH-06 |

## Platform delivery, support, assets, and new modules

| Catalog IDs | Decision | Codex benefit / disposition | Profile and task IDs |
|---|---|---|---|
| F113–F114 | Remove | macOS-specific packaging and App Store constraints are reference material, not Windows product scope. Preserve capability-driven lessons only. | Not applicable; SHELL-01/03 |
| F115 | Adapt | Retain the Windows packaging foundation only after app identity, capabilities, and release checks are aligned. | MVP delivery; REL-01/02/05/08 |
| F116 | Adapt | Provide explicit disconnect and safe removal of owned hooks/data before uninstall cleanup; preserve unrelated Codex configuration. | MVP/v1 delivery; REL-06/07 |
| F117–F121 | Adapt | Keep CI, focused tests, redacted local logs, previews, and accurate support/onboarding as release work; update all product claims to match supported evidence. | MVP/v1 delivery; REL-08/09, Stage 11 |
| F122 | Adapt | Keep asset files in the retained development prototype only; distribution requires per-asset rights evidence or original replacements and notices. | Development only until gate; SHELL-02/03, REL-11 |
| F123 | Defer | No updater is required to establish the monitor MVP; add signed updates with recovery only as release work. | v1/release extension; REL-03/04/05 |
| F124 | Defer | Native toast delivery and tracked-process resource monitoring are separate modules. Do not infer process ownership from a session or PID; notifications require verified activation targets. | Extension; STATUS-02/03/04/07 |

## Scope and evidence notes

The current frontend has a broker-backed session/history view and an explicitly opt-in GitHub workflow card. It renders only captured session/turn facts and safe history metadata. The monitor status currently reports a limited observer; configured hooks do not establish trust. Bounded normalized Codex history is replayed after restart with explicit partial-recovery and capacity reporting; restored active state is marked unknown and no live connection is recreated. PR/check fields come from the typed GitHub service snapshot, while repository/connection state comes from the broker. GitHub refresh is opt-in and may continue on the backend at its documented interval until disconnected.

This classification does not mark pending backend capabilities, deferred service code, native notifications, release packaging, or REL-11 as complete. The audit remains the detailed source reference; this document records scope decisions for the Windows Codex companion.
