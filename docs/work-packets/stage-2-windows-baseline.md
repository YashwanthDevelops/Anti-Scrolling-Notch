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
| Validation selection | Exact pinned Node v22.23.2/npm 10.9.8 and Rust 1.98.1, locked npm/Cargo dependencies, affected type/build/Rust checks and actual app/installer verification. Reuse the versioned Cargo cache; no unrelated lifecycle, network service or hook configuration checks. |
| Ownership | Root agent coordinates changes, commits and pushes; no application modules are assigned outside this packet. |

## Task status

| Task | Status | Evidence / remaining work |
|---|---|---|
| BASE-01 | Complete | Windows 11 Home 25H2 x64; Git 2.54.0.windows.1; isolated Node v22.23.2/npm 10.9.8; Rust 1.98.1/Cargo 1.98.1/rustup 1.29.1; Codex CLI 0.157.1; `gh` absent (optional); WebView2 154.0.4258.37. Exact install paths and provenance are in `docs/development-baseline.md`. |
| BASE-02 | Complete | Visual Studio Community 2026 18.10.3 is installed at `C:\Program Files\Microsoft Visual Studio\18\Community`; Desktop development with C++, MSVC 14.51.36231 (compiler 19.51.36260/linker 14.51.36260) and Windows SDK 10.0.26100.0 are verified. `VsDevCmd.bat -arch=x64`, `vswhere`, `cl`, `link` and `rc` work. Rust 1.98.1 MSVC, WebView2 and isolated Node v22.23.2/npm 10.9.8 are present. The earlier Build Tools 2022 attempt ending 1602 is historical; the user later completed the VS2026 installation. |
| BASE-03 | Complete | `npm ci` succeeded with Node v22.23.2/npm 10.9.8 using the checked-in lockfile: 18 packages added, 0 vulnerabilities, approximately 7.5 seconds. |
| BASE-04 | Complete | Local `npm ci`, TypeScript/Vite build, release hook prebuild, and `cargo test --workspace --locked` passed on Node 22/Rust 1.98.1; all 11 Rust tests passed. `cargo clippy --workspace --all-targets --locked` passed in 44.72 seconds with three inherited warnings recorded in `docs/development-baseline.md`. `cargo fmt --all -- --check` still reports formatting drift in 11 inherited files; no broad reformat was applied. |
| BASE-05 | Incomplete | `npm run tauri dev` successfully launched `windows\target\debug\coucou.exe` after a 40.05-second debug build. That proves process startup only. Tray, hidden/compact/expanded/greeting states, settings, sound, startup toggle, file drop and fixed-DPI visual/motion reference remain unverified. The supported computer-use helper failed its documented `@oai/sky` initialization and bounded retry/reset with a kernel-assets path error, preventing actual UI inspection. No Codex hooks/configuration or service credentials were added. |
| BASE-06 | Complete | `npm run pack` passed, including TypeScript/Vite, Rust release build and Tauri NSIS bundling. Tauri downloaded and hash-validated NSIS 3.11 and `nsis_tauri_utils v0.5.3`. Two ignored local setup artifacts were produced; each is 4,216,930 bytes with SHA-256 `7F3B0A22CDDE63B59F597B5FB8E831E28670FF2F7FC64ACF80F40FD15C0549B3`. They remain local baseline artifacts; installer was not run and no release was published. |
| BASE-07 | In progress | The existing PR #4 workflow's run 2 passed scope selection, docs checks (3.0 s), frontend (33.5 s), locked Rust tests (269.5 s), and the inherited Build workflow at `9e56fe89b4feebc9df5ca4c5c80da1662f5e9e2c`. This packet now pins Node v22.23.2/npm 10.9.8 and Rust 1.98.1, checks those versions in each job, and keys Cargo cache by Rust version. CI for those changed inputs is pending. |

## Gate

This packet remains incomplete: BASE-05 UI interaction/reference verification is pending, and BASE-07's exact pinned-toolchain CI run is in progress. BASE-02, BASE-04 and BASE-06 now have local evidence. Capture and verify the existing UI states and transitions when the supported UI helper is available; do not infer them from the process launch. Do not start Stage 3 until BASE-05 is complete and pinned CI passes.
