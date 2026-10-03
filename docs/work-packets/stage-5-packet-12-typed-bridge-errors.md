# Stage 5 packet 12 — typed native bridge failures

| Field | Required value |
|---|---|
| Packet | Stage 5, packet 12: CORE-10 bridge-error contract slice |
| Release profile | Monitor MVP reliability foundation |
| Task IDs | CORE-10 — native bridge failure-contract sub-scope |
| Branch / PR | `work/stage-5-core-10-legacy-bridge` / PR pending |
| Dependencies | CORE-03 and packet 11 integrated at `d213715dc83f3b90b6ca1c6b30b9e55da0b5fa67`; accepted four-event observer contract |
| User outcome | A failed native command is distinguishable from an absent optional result, with a safe typed error callers can handle |
| In scope | Common Tauri invocation path in `windows/src/core/bridge.ts`; stable `BridgeCallError` command/kind; focused bridge tests and CI; packet/plan/ledger status |
| UI/assets contract | Preserve Coucou/Mochi UI, assets, animation timing, and sounds byte-for-byte |
| Excluded | Legacy task/integration state migration and UI caller handling; reducer changes; new Codex events or producer; Stage 6 monitor; SHELL-08/09; UI/assets/sound/animation changes |
| Validation selection | Focused native/browser bridge-contract tests; existing view-store tests; TypeScript and Vite builds; Windows frontend CI; changed-document checks and diff review. Rust validation is not repeated unless Rust inputs change. |
| Ownership | Coordinator owns bridge implementation, validation, publication, and integration |

## Capability and architecture contract

This packet adds no Codex capability and does not alter the accepted observer fixture or event contract. The bridge remains a command boundary; it owns no frontend authority and does not convert anonymous hook observations into sessions or tasks. Capability guards remain unchanged and closed capabilities remain disabled.

## Acceptance evidence

| Dimension | Evidence |
|---|---|
| Real behavior | Exercise the actual frontend bridge module and installed Tauri JS `invoke` wrapper with a controlled `__TAURI_INTERNALS__` endpoint; prove optional browser-preview calls keep the documented no-op result and simulated native command failures reject. This is not a live app/backend IPC test. |
| Failure behavior | Assert required and optional native commands plus `broker_sync` reject with command-specific `BridgeCallError`; assert runtime-unavailable is typed; assert raw error text is absent from the public error. |
| Persistence/recovery | Not applicable: this packet changes no settings, broker state, persistence, or event ordering. |
| Automated verification | Focused bridge and view-store tests, TypeScript check, Vite production bundle, relevant Windows CI, changed-document checker, and `git diff --check`. |
| Manual/platform checks | No new Windows UI interaction is introduced; the Tauri invocation contract is exercised via its supported JS API boundary. |
| Release limitations | CORE-10 remains open for legacy frontend state authority migration and caller-side error handling. No production Codex monitor or new capabilities are enabled. |

## Implementation and validation status

Implemented in the shared bridge path. Plain-browser `call` operations still resolve to `null`; `callOrThrow` identifies an unavailable Tauri runtime; native invocation failures reject with command-specific `BridgeCallError` and one of two stable kinds. Raw invoke error values are discarded and never logged. `syncBroker` uses the same command helper. No UI callers or frontend state authority were migrated in this slice.

Local validation passed: `npm run test:bridge-errors` (5), `npm run test:view-store` (6), `npm run test:shell` (12), `npm run test:capabilities` (5), `npm run test:identity`, `npm run test:resources` (2), `npm exec -- tsc --noEmit`, `npm exec -- vite build`, `pwsh -NoProfile -File docs/check-markdown.ps1` (3 changed Markdown files), and `git diff --check`. The Vite-backed Node tests printed a non-fatal WebSocket port-24678-in-use warning from the existing desktop environment; all assertions passed. Rust files and dependencies are unchanged, so previously passing Stage 5 Rust evidence remains applicable. Hosted CI, PR review, and integration remain pending.
