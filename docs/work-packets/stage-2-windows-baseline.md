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
| BASE-04 | Partial | Direct TypeScript check and Vite production bundle passed (2.9 seconds; 35 modules). `cargo test --workspace --locked` stopped during compilation because `link.exe` is missing; no Rust tests executed. `cargo fmt --all -- --check` reports upstream formatting changes in 11 inherited Rust files; details are recorded below and no broad reformat was applied. |
| BASE-05 | Blocked | Native Tauri app was not launched because MSVC and the Windows SDK are missing. Tray, hidden/compact/expanded/greeting states, settings, sound, startup and file-drop behavior/reference capture remain unverified. No Codex hooks/configuration or service credentials were added. |
| BASE-06 | Blocked | Local installer was not built because the native app/toolchain baseline is blocked. NSIS readiness is unverified; no release publisher or distribution action was run. |
| BASE-07 | Partial | Added affected-path Windows CI with separate frontend and Rust-test jobs, lockfile caches, PR supersession and a docs-only whitespace/link/fence check. Staged whitespace validation and changed-document link/fence checks passed locally; workflow syntax inspection and hosted PR results remain pending. Formatting/Clippy enforcement is intentionally omitted while inherited formatting drift is unresolved. |

## Gate

This packet is not complete: BASE-02 and the native portions of BASE-04 through BASE-07 remain blocked or pending. Complete the missing administrator-approved C++/SDK installation, then run only the affected native app, Rust, installer and hosted-CI checks. Do not mark Stage 2 complete or start Stage 3.
