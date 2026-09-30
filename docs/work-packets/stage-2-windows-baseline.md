# Stage 2 packet — Reproducible Windows baseline

| Field | Value |
|---|---|
| Packet | Stage 2, packet 1: Windows baseline |
| Release profile | Monitor MVP foundation |
| Task IDs | BASE-01 through BASE-07 |
| Branch / PR | `work/windows-baseline` → Anti-Scrolling-Notch `main` |
| Dependencies | Stage 1 import and audited source 3cc3333203f60f63326ee949b7b86c7549992a1f |
| User outcome | Reproduce the inherited Windows shell and installer from a clean checkout, establish baseline behavior and checks, then add scoped Windows pull-request CI. |
| UI/assets contract | Retain Coucou/Mochi visuals and animation behavior as requested; capture and compare the existing appearance. |
| Excluded | Codex capability probes/hooks, backend or UI changes, branding/art replacement, optional service work and Stages 3–13. |
| Validation selection | Pinned Node 22 setup, locked npm/Cargo dependencies, affected type/build/Rust checks and actual app/installer verification. Reuse the CI toolchain lock cache; no unrelated lifecycle, network service or hook configuration checks. |
| Ownership | Root agent coordinates changes, commits and pushes; no application modules are assigned outside this packet. |

## Task status

| Task | Status | Evidence / remaining work |
|---|---|---|
| BASE-01 | Complete | Windows 11 Home 25H2 x64; Git 2.54.0.windows.1; isolated Node v22.23.2/npm 10.9.8; Rust 1.98.1/Cargo 1.98.1/rustup 1.29.1; Codex CLI 0.157.1; `gh` absent (optional); WebView2 154.0.4258.37. Exact install paths and provenance are in `docs/development-baseline.md`. |
| BASE-02 | Blocked | User-scoped Node and the stable MSVC-targeted Rust toolchain are installed; WebView2 already exists. Official Visual Studio Build Tools 2022 v17.14.41 was launched with the C++ workload, x64 MSVC tools and Windows 11 SDK 26100. Windows elevation ended with installer exit code 1602; `cl.exe`, `link.exe`, `vswhere.exe` and Windows SDK headers remain absent. Requires a successful administrator-approved install before native work. |
| BASE-03 | Complete | `npm ci` succeeded with Node v22.23.2/npm 10.9.8 using the checked-in lockfile: 18 packages added, 0 vulnerabilities, approximately 7.5 seconds. |
| BASE-04 | Complete | Direct TypeScript check and Vite production bundle passed locally (2.9 seconds; 35 modules). The locked Rust workspace tests passed in hosted Windows CI on Rust 1.98.1 after the required release-hook prebuild; that job took 269.5 seconds including setup/cache work. The local command was blocked by missing link.exe, recorded as a workstation prerequisite issue. The formatter reports upstream changes in 11 inherited Rust files; this baseline is tracked without broad reformatting. Clippy enforcement remains deferred per the plan until recorded upstream issues are resolved. |
| BASE-05 | Blocked | Native Tauri app was not launched because MSVC and the Windows SDK are missing. Tray, hidden/compact/expanded/greeting states, settings, sound, startup and file-drop behavior/reference capture remain unverified. No Codex hooks/configuration or service credentials were added. |
| BASE-06 | Blocked | Local installer was not built because the native app/toolchain baseline is blocked. NSIS readiness is unverified; no release publisher or distribution action was run. |
| BASE-07 | Complete | Windows CI passed on PR #4, run 2 at commit 9e56fe89b4feebc9df5ca4c5c80da1662f5e9e2c: docs checks 3.0 seconds, frontend 33.5 seconds, locked Rust tests 269.5 seconds. Scope selection passed; frontend and Rust checks are separate; Cargo dependencies/build output are cached; new commits supersede the same PR's old run; docs-only changes run whitespace/link/fence checks. The first run exposed two issues and the second validated their fixes: docs flag binding and Tauri hook-resource build order. Clippy/format enforcement is deferred while inherited formatting drift remains tracked. |

## Gate

This packet remains incomplete: BASE-02, BASE-05 and BASE-06 are blocked or pending. Complete the administrator-approved C++/Windows SDK installation, then verify the local native app and installer. Keep the successful CI results and tracked formatter baseline; do not start Stage 3 until the full Stage 2 gate passes.
