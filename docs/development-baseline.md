# Windows development baseline

Status: Stage 2 is in progress. The workstation toolchain is installed, local frontend/Rust checks and native packaging pass, and the Tauri development process launches. The user-facing app interaction/reference check remains incomplete; do not infer tray or visual behavior from a successful process launch.

## Initial machine inventory

Observed from the project workstation before installing dependencies:

| Tool / component | Initial observation | Required state |
|---|---|---|
| Windows | Windows 11 Home, 25H2, build 26200, AMD64 | Supported x64 workstation |
| Git | 2.54.0.windows.1 | Available |
| Node.js | 26.4.0, global | Use Node 22 to match the checked-in Windows CI before validating the baseline |
| npm | 11.17.0 | Record version under Node 22 |
| Rust / Cargo | `rustc` and `cargo` not found | Stable x86_64-pc-windows-msvc |
| GitHub CLI | Not installed | Optional; repository is reachable using git and GitHub integration |
| Codex | `codex-cli 0.157.1` | Record; do not install hooks in Stage 2 |
| MSVC C++ build tools / linker | `cl`, `link`, `vswhere` and Visual Studio Build Tools not found | Desktop development with C++, x86/x64 MSVC tools |
| Windows SDK | `C:\Program Files (x86)\Windows Kits\10` not found | Windows SDK component required by Tauri |
| WebView2 | Runtime executable at `C:\Program Files (x86)\Microsoft\EdgeWebView\Application\154.0.4258.37\msedgewebview2.exe` | Already installed |
| NSIS | `makensis` and common NSIS paths not found | Verify Tauri's local installer build can use/provision its bundler dependency |

## Verified Stage 2 workstation state

| Tool / component | Verified state |
|---|---|
| Node.js / npm | Node v22.23.2 / npm 10.9.8 from an isolated official ZIP at `C:\Users\Yashwanth\AppData\Local\Programs\Anti-Scrolling-Notch\node-v22.23.2-win-x64\node-v22.23.2-win-x64`; SHA-256 `1177b4137ba5adaa56354ae40f1080c7450e8ae09cecb47da459d1c52ac99f97`. Node 26.4.0 remains the machine's global installation. |
| Rust / Cargo | rustup 1.29.1 with stable `x86_64-pc-windows-msvc`, rustc 1.98.1 (`48a229cea`, 2026-09-01), Cargo 1.98.1; rustfmt and Clippy components installed. Cargo's user bin directory was added only to command-process PATH, not the persistent machine PATH. |
| Visual Studio / MSVC | Visual Studio Community 2026 version 18.10.3 (`VisualStudio/18.10.3+12224.181`), installed at `C:\Program Files\Microsoft Visual Studio\18\Community`; MSVC tools 14.51.36231, x64 compiler 19.51.36260 and linker 14.51.36260. `vswhere.exe` is at `C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe`. User-provided Developer Command Prompt output and independent `vswhere`, compiler and linker checks confirm the installation. |
| Windows SDK | Windows SDK 10.0.26100.0 is installed under `C:\Program Files (x86)\Windows Kits\10`; x64 headers and `kernel32.lib` are present. `VsDevCmd.bat -arch=x64` initializes successfully and locates `cl`, `link` and `rc`. |
| WebView2 | Runtime remains present at `C:\Program Files (x86)\Microsoft\EdgeWebView\Application\154.0.4258.37\msedgewebview2.exe`. |
| GitHub CLI / Codex | `gh` absent (optional); `codex-cli 0.157.1` available. No Codex hooks/configuration or service credentials were installed for this baseline. |
| NSIS | Global `makensis` remains absent. Tauri downloaded NSIS 3.11 and `nsis_tauri_utils v0.5.3`, verified their hashes, then built the local installer successfully. |

## Checks and evidence

| Check | Command / machine | Result |
|---|---|---|
| Git/Node/npm/Codex initial probe | Stage 2 workstation, PowerShell | Git 2.54.0.windows.1; Node v26.4.0; npm 11.17.0; codex-cli 0.157.1. |
| Windows prerequisites | VS2026 Developer Command Prompt, `vswhere`, filesystem checks | VS Community 2026 18.10.3, MSVC x64 compiler/linker and SDK 10.0.26100.0 verified; WebView2 present. The earlier 2022 Build Tools installer attempt ended with code 1602; the user subsequently completed the VS2026 installation through the normal installer. |
| Node 22 / Rust MSVC setup | Official Node.js v22.23.2 ZIP checksum and rustup installer checksum verified before install | Node/npm and rustc/Cargo versions above are available in user-scoped locations; Node archive SHA-256 is recorded above. The official rustup executable was checked against its winget manifest SHA-256 before running. |
| BASE-03 locked dependency install | `npm ci` in `windows`, Node v22.23.2/npm 10.9.8 | Passed in 7.5 seconds; 18 packages added; audit reported 0 vulnerabilities. |
| Frontend type and production bundle | `tsc --noEmit`, `vite build` in `windows`, Node v22.23.2/npm 10.9.8 | Passed; Vite transformed 35 modules. The independent `npm run pack` run also passed TypeScript and Vite production builds (35 modules, 370 ms). |
| Release hook prebuild and Rust tests | `cargo build --release -p coucou-hook && cargo test --workspace --locked` in `windows`, Visual Studio 2026 x64 environment, rustc/Cargo 1.98.1 | Both passed. Release hook prebuild took 13.21 seconds; workspace tests passed 8 library tests and 3 hook tests (11 total); test-profile compile and run took 1 minute 27 seconds. |
| Rust formatting baseline | `cargo fmt --all -- --check` in `windows`, rustfmt 1.9.0-stable | Existing formatting drift remains in 11 inherited files: `hook/src/main.rs`, `hook/src/win.rs`, and `src-tauri/src/{claude,files,hooks,integrations,island,lib,log,pipe,win_user}.rs`. No unrelated formatting edits were made. |
| Clippy baseline | `cargo clippy --workspace --all-targets --locked` in `windows`, rustc 1.98.1 | Passed in 44.72 seconds with three inherited warnings: `needless_range_loop` at `src-tauri/src/hooks.rs:419`, `manual_is_multiple_of` at `src-tauri/src/island.rs:293`, and `unnecessary_to_owned` at `src-tauri/src/island.rs:299`. |
| Native development launch | `npm run tauri dev` in `windows`, isolated Node 22 and VS2026 x64 environment | Vite became ready in 830 ms; Tauri built the debug app in 40.05 seconds and launched `windows\\target\\debug\\coucou.exe`. This verifies process startup only. Tray, hidden/compact/expanded/greeting states, settings, sound, startup and file drop were not verified. |
| UI interaction/reference capture | Supported computer-use helper through its documented `@oai/sky` initialization | Could not complete: helper initialization returned “failed to write kernel assets: The system cannot find the path specified,” including the prescribed bounded retry/reset. Therefore fixed-DPI screenshots, transitions and interactive behavior remain unverified. No alternate UI-automation route was used. |
| BASE-06 local installer | `npm run pack` in `windows`, isolated Node 22 and VS2026 x64 environment | Passed: TypeScript/Vite production build, Rust release build and Tauri NSIS bundling completed. Local artifacts `windows\\release\\Coucou-Windows-0.1.1-setup.exe` and `windows\\release\\Coucou-Windows-setup.exe` are each 4,216,930 bytes with SHA-256 `7F3B0A22CDDE63B59F597B5FB8E831E28670FF2F7FC64ACF80F40FD15C0549B3`. They are ignored local baseline artifacts, not distributable releases; installer was not run. |
| Windows pull request CI | `.github/workflows/windows-ci.yml`, PR #4, prior validated commit `9e56fe89b4feebc9df5ca4c5c80da1662f5e9e2c` | Prior workflow run 2 passed scope selection, docs (3.0 s), frontend (33.5 s), locked Rust tests (269.5 s), and the inherited Build workflow. Current changes pin the exact Node/Rust versions and are awaiting a new CI run. |
| Changed documentation | `pwsh -NoProfile -File docs/check-markdown.ps1` and `git diff --check` | Run for the current packet before publication. An earlier whole-repository diagnostic found a pre-existing broken link in untouched `docs/upstream-README.md` targeting `windows/README.md`; CI checks changed documents only. |

The successful build and installer establish the native toolchain/package baseline, but do not substitute for BASE-05 interaction and visual-reference verification. Stage 3 remains gated on completion of the whole Stage 2 packet.
