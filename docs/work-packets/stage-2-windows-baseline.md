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
| BASE-05 | Complete — project-owner accepted | The source audit establishes the implemented tray, island states/transitions, settings, sound, startup and file-drop behavior; see the audited implementation paths summarized in `docs/development-baseline.md`. The existing native Tauri launch is recorded there. The project owner directly reports manually verifying the app UI and interactions and accepts BASE-05 on that basis. This runtime evidence is owner-reported, not independently automated. No independent fixed-DPI screenshot or runtime motion measurement was produced; source timings remain configured values, not measured durations. No Codex hooks/configuration or service credentials were added. |
| BASE-06 | Complete | `npm run pack` passed, including TypeScript/Vite, Rust release build and Tauri NSIS bundling. Tauri downloaded and hash-validated NSIS 3.11 and `nsis_tauri_utils v0.5.3`. Two ignored local setup artifacts were produced; each is 4,216,930 bytes with SHA-256 `7F3B0A22CDDE63B59F597B5FB8E831E28670FF2F7FC64ACF80F40FD15C0549B3`. They remain local baseline artifacts; installer was not run and no release was published. |
| BASE-07 | Complete | PR #4 Windows CI run 5 passed at `74ab268bab64760ca1aa279ed40c2617f2989643`: scope (5 s), docs (7 s), frontend (29 s) and Rust tests (265 s, including setup/cache and the release-hook prebuild). Both exact Node/npm and Rust/Cargo assertions passed. The inherited Build workflow run 14 also passed (105 s). The workflow pins Node v22.23.2/npm 10.9.8 and Rust 1.98.1; the Cargo cache key includes the Rust version. |

## Gate

This packet is accepted complete for the Monitor MVP foundation. BASE-01 through BASE-04, BASE-06 and BASE-07 have local or hosted evidence. BASE-05 is accepted by the project owner based on the source audit, existing successful native launch and direct manual verification of the app UI/interactions. Runtime interaction evidence is owner-reported rather than independently automated; no fixed-DPI capture or motion-duration measurement is claimed. No product behavior was changed to satisfy evidence requirements. Stage 3 may proceed under its own bounded packet.
