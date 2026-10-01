# Codex compatibility and capability evidence

This is the versioned evidence matrix for the Stage 3 hard capability gate. A production capability stays disabled until its version/surface, sanitized fixtures, runnable integration test, failure behavior, and fallback are recorded. The [Codex Hooks reference](https://developers.openai.com/codex/hooks/) is a documentation reference, not a replacement for installed-version/runtime evidence.

| Surface / capability | Version | Transport / evidence | Failure behavior / fallback | MVP state |
|---|---|---|---|---|
| Codex CLI observing-hook boundary | `codex-cli 0.157.1`; installed `hooks` feature reported stable/enabled | A successful loopback CLI-boundary turn delivered `SessionStart`, `UserPromptSubmit`, `Stop`, and `SessionEnd`. The sanitized recorder projection is preserved in [`captured-projections.jsonl`](../tests/compat/codex-hooks/captured-projections.jsonl). The isolated Codex home and original diagnostic artifacts remain local under `%TEMP%\codex-hook-probe-a11f54287efa496c854aac8b774923c2`; no raw prompt, path, or identifier is committed. | Invalid input, oversized input, unknown event names, absent app, busy pipe, or a timed-out relay return neutral success with no hook output. No raw event payload fields cross the adapter boundary; only the verified event name becomes an internal enum. The user can continue in Codex if the companion is closed or unavailable. | Four lifecycle event discriminators accepted by the adapter; all other inputs remain disabled. |
| Codex Desktop session/hooks surface | Session/protocol compatibility remains unestablished; installed app package is `OpenAI.Codex` 26.928.2636.0 | No Desktop session or hook probe was performed. The app activation probe below verifies only launching the generic app surface. | Keep Desktop session monitoring and controls disabled. | Unverified / disabled. |
| Windows Codex app activation | Windows 11 Home 10.0.26200; package `OpenAI.Codex` 26.928.2636.0; Start Apps label `ChatGPT`; AUMID `OpenAI.Codex_2p2nqsd0c76g0!App` | `IApplicationActivationManager::ActivateApplication` with `AO_NOERRORUI`; the local Rust integration test activated the registered app and received a nonzero process id. A foreground-window read found `ChatGPT.exe` under the installed `OpenAI.Codex_26.928.2636.0` package. | An unknown/uninstalled AUMID returns an HRESULT error without modal OS UI. Use the generic Codex app landing surface; no conversation/session deep link is claimed. | Generic app launch verified on this Windows/package version; no Codex session capability is inferred. |
| Managed Codex App Server session | Separate COMPAT-05 gate before Stage 8 | No managed session transport is implemented by this packet. | No managed-session capability is exposed. | Deferred / disabled. |
| Existing desktop/shared-session attachment | Not selected or verified | Separate extension-only COMPAT-06 investigation. | No attachment or control is available; open Codex/project as fallback. | Deferred / disabled. |

## Captured loopback contract

The evidence came from Codex CLI 0.157.1 in a disposable Codex home and a disposable local repository, using a loopback-only fake Responses endpoint. It did not use the owner's regular Codex authentication, OpenAI credentials, or production configuration. The hook recorder returned success and emitted no hook output. The probe's sanitized projection records:

| Observed event | Sanitized observed detail |
|---|---|
| `SessionStart` | `source` was `startup`. |
| `UserPromptSubmit` | A `prompt` member was present; its contents were not recorded. |
| `Stop` | Event name was recorded; no additional property is claimed here. |
| `SessionEnd` | `reason` was `other`. |

These are the only runtime-confirmed event names. The projection is intentionally not represented as the complete Codex payload schema: it contains only values selected by the probe recorder, and `prompt_present` is a sanitized boolean derived by that recorder. The production adapter requires only a JSON object with a string `hook_event_name` equal to one of the four confirmed names. It discards all other input members and sends the app a private versioned message containing only `wire_version` and `event`.

The current adapter does not claim stable session IDs, project paths, prompt text, tool-use IDs, tool input/output, parent/agent identity, compaction, interruption, approval, final-response status, or Desktop behavior. Multiple-session separation and activity summaries therefore remain unsupported until their required inputs are independently observed. The documented `PreToolUse`, `PostToolUse`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, and `Interrupt` event names were configured during the loopback probe but were not delivered in that turn; the adapter rejects them as unverified. Permission hooks are not part of this observing adapter.

The Ollama-specific smoke test was not performed because Ollama is not installed. No Ollama installation or further provider investigation was performed. This limitation does not change the accepted loopback evidence for the four events above.

## Adapter tests

The portable contract tests run with `cargo test -p codex-hook-contract`. The Windows backend and hook tests run with `cargo test -p coucou -p coucou-hook`. The separate Windows pipe replay check is `powershell -ExecutionPolicy Bypass -File windows/scripts/test-codex-hook-transport.ps1` after `cargo build --release -p coucou-hook`; it reads the committed sanitized projection rather than duplicating the captured event names in the script.

The transport uses a Codex-only named pipe and never enters the inherited Claude frontend hook handler or permission-response path. The receiver currently records only the four event names in the local app log; Stage 5 owns authoritative reduction, persistence, event ordering and snapshots, and Stage 6 owns user-facing monitoring workflows. The adapter by itself does not enable those later capabilities.

The installed Codex home and original local diagnostic artifacts remain isolated and are not committed. No hooks were installed into the normal Codex home, and no normal Codex configuration or credentials were read or changed for the implementation.

## Navigation boundary

Windows registered the stable Codex package as `OpenAI.Codex_2p2nqsd0c76g0` version `26.928.2636.0`; its Start Apps label is `ChatGPT`, and its application id is `App`. The native activation command uses the resulting AUMID through Windows' packaged-app activation manager and returns an error when activation fails. It opens the generic app surface only. It does not claim to select a Codex thread, CLI session, terminal, or project.

The activation route follows Microsoft's [`IApplicationActivationManager::ActivateApplication`](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-iapplicationactivationmanager-activateapplication) contract. Windows' [`Get-StartApps`](https://learn.microsoft.com/en-us/powershell/module/startlayout/get-startapps?view=windowsserver2025-ps) output supplies the current user's registered app name and AppUserModelID; the test recorded that identity without reading Codex settings or credentials.

The inherited project-opening backend can open a caller-supplied folder in VS Code and falls back to Explorer. The verified four-event hook envelope contains no project path or session identity, so those actions are not yet associated with a Codex session. Keep that session-specific navigation unavailable until a later captured contract supplies a verified path.
