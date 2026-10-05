# Stage 5 packet 15 — CORE-11 architecture and contract guards

## Packet identity and scope

| Field | Required value |
|---|---|
| Packet | Stage 5, packet 15: CORE-11 architecture and contract guards |
| Release profile | Monitor MVP state-authority foundation |
| Task IDs | CORE-11 — available architecture and contract-test slice |
| Branch / PR | `work/stage-5-core-11-contract-guards` / [PR #30](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch/pull/30), merged at `0fe12a132375de05207b9d91b969aa0bdf2faedc` |
| Status | Integrated; final-head Windows CI #74 and Build #83 passed |
| Base | Clean `main` and `origin/main` at PR #29 merge `39ba05b3cde971684deb7b323bace69ad631327b` |
| User outcome | Frontend changes cannot silently bypass the bridge or mutate broker-owned state, and current broker/request failure semantics have an executable cross-component regression test |
| In scope | Frontend AST architecture tests; a test-only request-router/broker contract test for failed delivery and authoritative external resolution; Windows CI registration; plan and ledger updates |
| Excluded | New production dispatcher, Tauri command/event or adapter; Codex/GitHub monitor; source event schema; capability enablement; Stage 6; SHELL-08/09; UI, asset, animation, sound, or request behavior changes |
| Validation | Focused new frontend and Rust tests; relevant frontend suites, typecheck/build; locked Rust workspace tests, Clippy and formatting; Markdown and diff checks; final-head Windows CI and Build |

## Verified boundary and limits

The frontend currently imports Tauri APIs only from `src/core/bridge.ts`. Its only `fetch` call loads local inherited sound assets. The `BrokerViewStore` keeps its snapshot private and exposes it through `DeepReadonly`; legacy Mochi `State` has no session, turn, tool-item, pending-request, or repository collection. The allowed local integration display payload remains presentation state.

The Rust request router, normalized reducer, and broker stream are executable contracts, not a production Codex reply pipeline. The Codex observer still provides only its four accepted anonymous lifecycle names; it does not provide a pending-request producer, source identity, or verified response transport. Request capabilities remain disabled in production. Therefore the new cross-component test uses a test-only enabled capability fixture and a simulated adapter rejection, then an explicit terminal broker update representing another client resolving the request. It proves the existing contracts compose and do not infer resolution from failed delivery; it does not claim live dispatch, network delivery, or a captured external-resolution event.

## Acceptance evidence

| Dimension | Required evidence |
|---|---|
| Architecture boundary | AST tests reject direct frontend Tauri access outside the bridge, direct process/API clients, non-asset network calls, and writes to or mutable access through the broker snapshot |
| Failure path | The request-router/broker test proves a rejected-before-send outcome leaves the exact request pending in the backend snapshot |
| External resolution | The test proves only a terminal normalized source update clears the router entry and removes the request from a subsequent authoritative snapshot |
| Scope control | No production command, event, adapter, schema, or enabled capability is added; unsupported live dispatch remains documented |
| Automated verification | Focused tests, relevant frontend and Rust checks, documentation checker, diff review, Windows CI and Build pass |

## Local validation results

The new architecture suite passed 2/2; capability gates passed 5/5; shell/motion/sequence tests passed 12/12; broker view-store tests passed 7/7; typed bridge tests passed 8/8; resource checks passed 2/2; and runtime identity validation passed. TypeScript checking and the Vite production build passed.

The locked Rust workspace suite passed 114 tests (99 app, 7 relay, 2 runtime identity, 6 hook-contract); 2 installed-app tests were ignored. The focused failed-delivery/external-resolution test passed after the final source edit. Workspace Clippy passed with the existing `hooks.rs:467` warning. `rustfmt --check` on the edited router module reports only the inherited formatting difference in an unchanged test near line 807; the added test hunk has no formatting diff, and no unrelated formatting was applied. The Markdown checker validated four changed Markdown files and `git diff --check` passed. Final PR head `55cfd91563d88e496f084d969c400f895ef49bd2` passed Windows CI run #74 and Build run #83. PR #30 had no review submissions or unresolved review threads and merged at `0fe12a132375de05207b9d91b969aa0bdf2faedc`. Local `main`, `origin/main`, and the fetched remote `main` ref match the merge commit; the worktree is clean. The packet's test/guard slice is integrated; CORE-11 remains open for the source-backed live intent/adapter flow.

## PR outcome and remaining completion gate

The scoped test/guard slice was committed, pushed and merged after final-head CI passed. CORE-11 remains open for its source-backed live intent/adapter flow; no test fixture may be described as runtime evidence.
