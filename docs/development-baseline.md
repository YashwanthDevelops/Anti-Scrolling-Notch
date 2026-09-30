# Windows development baseline

Status: Stage 2 Windows baseline packet in progress. This file records verified machine/toolchain state and exact command results; it does not imply that the application has built or run.

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
| Node.js / npm used for this packet | Node v22.23.2 / npm 10.9.8 from an isolated official ZIP at `C:\Users\Yashwanth\AppData\Local\Programs\Anti-Scrolling-Notch\node-v22.23.2-win-x64\node-v22.23.2-win-x64`; SHA-256 `1177b4137ba5adaa56354ae40f1080c7450e8ae09cecb47da459d1c52ac99f97`. Node 26.4.0 remains the machine's global installation. |
| Rust / Cargo | rustup 1.29.1 with stable `x86_64-pc-windows-msvc`, rustc 1.98.1 (`48a229cea`, 2026-09-01), Cargo 1.98.1; rustfmt and Clippy components installed. Cargo's user bin directory was added only to command-process PATH, not the persistent machine PATH. |
| C++ / Windows SDK | Still absent. Official Microsoft Visual Studio Build Tools 2022 v17.14.41 was invoked for `Microsoft.VisualStudio.Workload.VCTools`, x64 MSVC tools and `Microsoft.VisualStudio.Component.Windows11SDK.26100`. Windows elevation returned installer code 1602; `cl`, `link`, `vswhere` and SDK headers remain unavailable. |
| WebView2 | Runtime remains present at `C:\Program Files (x86)\Microsoft\EdgeWebView\Application\154.0.4258.37\msedgewebview2.exe`. |
| GitHub CLI / Codex | `gh` absent (optional); `codex-cli 0.157.1` available. |
| NSIS | `makensis` absent; installer bundler has not been tested. |

No CI result substitutes for a local Windows Tauri start or local installer build.

## Checks and evidence

| Check | Command / machine | Result |
|---|---|---|
| Git/Node/npm/Codex initial probe | Stage 2 workstation, PowerShell | Git 2.54.0.windows.1; Node v26.4.0; npm 11.17.0; codex-cli 0.157.1 |
| Windows prerequisites | Command discovery, filesystem and registry checks | WebView2 present; Rust/Cargo, MSVC C++ and Windows SDK missing |
| Node 22 / Rust MSVC setup | Official Node.js v22.23.2 ZIP checksum and rustup installer checksum verified before install | Node/npm and rustc/Cargo versions above are available in user-scoped locations; Node archive SHA-256 is recorded above. Winget's Rustup installer was not applicable, so the official rustup executable was downloaded and checked against its winget manifest SHA-256 before running. |
| Visual Studio C++ / SDK setup | Official Build Tools 2022 v17.14.41 with C++ workload, x64 MSVC and Windows SDK 26100; standard Windows elevation flow | Installer exited with code 1602; no compiler or SDK was installed. No elevation bypass or repeated installer attempt was used. |
| BASE-03 locked dependency install | `npm ci` in `windows`, Node v22.23.2/npm 10.9.8 | Passed in 7.5 seconds; 18 packages added; audit reported 0 vulnerabilities. |
| Frontend type and production bundle | `node_modules/.bin/tsc.cmd --noEmit`, then `node_modules/.bin/vite.cmd build` in `windows` | Passed in 2.9 seconds; Vite transformed 35 modules and produced the production bundle. |
| Rust tests | `cargo test --workspace --locked` in `windows`, rustc 1.98.1 | Blocked after dependency resolution/compilation began: `link.exe` not found for the MSVC target. No tests executed; this is an environment prerequisite failure, not a test assertion failure. |
| Rust formatting baseline | `cargo fmt --all -- --check` in `windows`, rustfmt 1.9.0-stable | Existing formatting drift in 11 inherited files: `hook/src/main.rs`, `hook/src/win.rs`, and `src-tauri/src/{claude,files,hooks,integrations,island,lib,log,pipe,win_user}.rs`. No unrelated formatting edits were made. |
| Native desktop behavior | Deferred because the local MSVC/SDK prerequisite is missing | Not run; do not infer behavior or visual reference states from CI. |
| NSIS installer | Deferred because the native build prerequisite is missing | Not run; NSIS availability remains unverified. |
| Windows pull request CI | `.github/workflows/windows-ci.yml` added | First hosted run parsed the workflow and passed scope selection plus the frontend job. The Rust test job exposed the required `cargo build --release -p coucou-hook` resource prebuild; the docs job exposed PowerShell's refusal to cast the string PR flag to Boolean. Both corrections are in the follow-up commit; hosted rerun is pending. Local changed-document checks pass for the four packet Markdown files. An initial whole-repository scan also found an existing broken link in untouched `docs/upstream-README.md` (target `windows/README.md`), so CI checks only changed documents. |
