# Stage 4 packet 3 — accessible motion and deferred controls

## Packet identity and scope

| Field | Value |
|---|---|
| Packet | Stage 4, packet 3: accessible motion and deferred controls |
| Release profile | Monitor MVP foundation |
| Task IDs | SHELL-07 only |
| Branch / PR | `work/stage-4-accessible-settings` → Anti-Scrolling-Notch `main` |
| Dependencies | Stage 2 BASE-05 owner-accepted; Stage 3 compatibility packets integrated; Stage 4 packets 1–2 integrated, packet 2 merge `acaece8379b191a31e42c7f0e6f7f727dfdc8c89` |
| User outcome | Preserve the inherited Coucou/Mochi experience by default, offer an opt-in reduced-motion setting, and keep unimplemented Codex chat/file actions visibly unavailable until their verified adapters ship. |
| In scope | Persist a migration-safe reduced-motion preference, expose it in General settings, apply it live to CSS and canvas motion while preserving status and sound, and leave the default-off Coucou motion path unchanged. Keep the Ask and Drop destinations navigable for context, but disable submission/drop processing unless the backend registry enables `codex.managedSession` and `codex.attachmentDelivery`, respectively. Fail closed in the Rust commands too. Reuse the existing sound preferences and capability registry. |
| UI/assets contract | Keep inherited layout, icons, Mochi appearance, expressions, sounds, and default animation timings. The reduced-motion option is opt-in and affects motion only. Keep deferred destinations visually consistent and explain the direct-Codex fallback; do not redesign the island. No asset changes. |
| Excluded | SHELL-08/09; Stage 5 reducer/storage architecture; Stage 6 production monitor; Stage 8 App Server implementation; Stage 10 attachment transport; notifications, GitHub workflows, integrations, and later stages. Do not enable either disabled capability. |
| Validation selection | Rust migration/round-trip and capability denial tests; frontend reduced-motion engine/state and capability-guard tests; TypeScript/Vite build; locked workspace tests, Clippy, formatting, resource hashes, documentation checks and CI. Reuse Stage 4 packet 2's inherited 28-sound hash result unless sound-related inputs change. No interactive-runtime claim. |
| Ownership | Coordinator owns all packet files, validation, staging, commits, push, PR and integration. |

## Capability and behavior contract

`codex.managedSession` and `codex.attachmentDelivery` remain disabled because their adapters are not implemented or verified. The UI may explain the unavailable route and direct the user to Codex, but it must not submit text to the inherited Anthropic API, copy a dropped file into the inbox, or claim Codex delivery. Rust command checks use the current backend-discovered registry and its generation; stale, missing, fixture, or disabled registry entries reject before provider or file operations.

Reduced motion is a local presentation preference, defaulting to off so the existing Coucou animation and timing path is unchanged for users who do not opt in. When enabled, CSS transitions/keyframes, island geometry springs, greeting choreography, Mochi tween/particle/ambient motion, and mini-avatar movement settle to stable visual states. Mochi status colors, badges, expressions, and enabled sound playback remain available. The preference is persisted with the existing settings JSON and applies immediately between the island and settings window.

## Acceptance evidence

| Requirement | Evidence |
|---|---|
| Reduced motion defaults off and survives old settings migration plus JSON round trip | Rust settings tests |
| Enabling reduced motion settles the Mochi engine, suppresses particles, and completes greeting choreography immediately; disabling it restores tween behavior | `motion.test.mjs` loads the actual TypeScript modules through the installed Vite SSR loader; production build covers island/settings wiring and CSS. WebView2 rendering remains runtime-unverified. |
| Existing sound toggle/volume remain independent, persisted preferences; sound asset hashes remain unchanged | Existing settings/resource checks |
| Chat and file controls present visible truthful fallback text and do not invoke Anthropic or ingest files when the corresponding Codex capability is absent/disabled | Frontend capability tests, Rust command guard tests, and source review showing the Anthropic chat command/client are removed |
| Unverified capabilities stay disabled and no Codex event schema or adapter is added | Existing Rust capability registry tests and scoped diff review |
| No asset bytes, inherited sound behavior, or default animation timings change | Resource hash test and final diff audit |

## Local implementation and validation — 2 October 2026

SHELL-07 adds `reducedMotion`, default off, to the existing settings object. The Rust settings migration supplies `false` for older JSON, and the JSON round-trip test verifies that reduced motion persists independently of sound mute and volume. General settings exposes the option and sends changes through the existing `settings-changed` event so the island and settings window update immediately.

The default-off Coucou path remains intact. With the preference enabled, CSS transitions/keyframes settle, island geometry and mini-avatar positions jump to their final targets, the Mochi engine clears active tweens/particles and snaps to stable state, and the greeting completes without timed choreography. Sounds remain enabled or muted only by the existing sound preference. No asset files, sound files, inherited animation constants, or default transition timings changed.

The managed-chat and attachment capabilities remain disabled in the backend registry. The Ask composer disables its input/send action and shows a visible direct-Codex fallback; Drop explains that file delivery is deferred. File-drop handling and the inbox command check the backend capability generation before doing work. The inherited Anthropic chat Tauri command and HTTP client are removed, so entering text cannot call Claude. The legacy Claude settings fields remain a later SHELL-09 audit item and do not enable chat.

Local checks: `npm run test:shell` passed 7/7; `npm run test:capabilities` passed 5/5; `npm run build --ignore-scripts` passed TypeScript and Vite production build; `npm run test:resources` passed 2/2 and verified all 28 WAV hashes; `npm run test:identity` passed; the Markdown checker validated all three changed documents; and `git diff --check` passed. The motion test imports the real source modules through the existing Vite dependency; it does not launch WebView2. `rustup run stable cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked`, and scoped Rust formatting passed earlier on this same unchanged Rust diff. Clippy retained one inherited warning in `hooks.rs`. Runtime interaction, actual WebView reduced-motion appearance, and sound playback remain runtime-unverified; no user settings, credentials, startup state or Codex configuration were touched.

The Impeccable detector reported two pre-existing bounce-easing warnings in the inherited stylesheet. Those lines are unchanged and were retained to preserve the requested Coucou motion behavior. The asset/resource manifest and all inherited asset bytes are unchanged.

## PR and completion gate

Merge this packet only after the relevant settings, capability, motion, build, formatting, documentation, and CI checks pass. Record unavailable manual animation/runtime observations as runtime-unverified. Do not activate SHELL-08 or SHELL-09 until this packet is integrated.
