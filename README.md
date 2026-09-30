# Anti-Scrolling-Notch

A planned Windows companion for OpenAI Codex, inspired by Coucou's top-of-screen island workflow.

Repository: [YashwanthDevelops/Anti-Scrolling-Notch](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch).

## Current status

The audited Coucou source and its Git history have been imported as the development baseline. The updated plan was published on 1 October 2026; the Windows build and Codex feature implementation are pending. The inherited application still implements Coucou/Claude behavior. This repository does not yet contain a completed Codex companion.

## Project documentation

- [Updated implementation plan](docs/implementation-plan.md)
- [Execution ledger](docs/execution-ledger.md)
- [Complete Coucou feature and architecture audit](docs/coucou-analysis.md)
- [Inventory of all audited repository files](docs/coucou-source-inventory.md)
- [Source provenance and licensing](docs/source-provenance.md)
- [Original upstream README](docs/upstream-README.md)

## Implementation approach

Use Coucou's existing Windows Tauri/Rust/TypeScript shell as the foundation and select inherited behaviors by their usefulness for Codex. Execute the master plan through one bounded stage/PR at a time.

Initially retain Coucou's existing UI, Coucou/Mochi visual assets, character animations, sounds, layout and interaction timing for development/prototyping while adapting the data and controls to Codex. Treat inherited assets as replaceable resources. A UI redesign is later work only if the user requests it. Use affected checks and reusable valid evidence during development; broader integration/package validation happens at the relevant boundaries and releases.

The monitor MVP includes the original shell/settings, independent Codex sessions/activity, Git/worktree and GitHub PR/CI monitoring, Windows notifications, basic history and reload/reconnect recovery. Interactive v1 adds managed App Server chat, streaming, actual approvals/questions, interrupt/steer, attachments and detailed history. Other services, voice, shared desktop, hosted webhooks and WSL/ARM64 are selected extensions.

Production capabilities need verified supported-version behavior, tests, failure handling and documented fallback. The Rust backend owns authoritative state; UI reads snapshots and sends typed intent. A screen alone does not complete a feature. Each released profile must pass the plan's QA and delivery gates; the complete core project is the verified interactive v1 release.

## Development and publication

The Windows project is in the windows directory. Read [AGENTS.md](AGENTS.md), the implementation plan and [stage packet template](docs/stage-packet-template.md) before working. The next packet is Stage 2 Windows baseline, not feature implementation. Make small verified commits and push each completed change to this repository. The upstream remote is used to fetch and review Coucou changes; its push URL is disabled in the development checkout.

Inherited visuals/animations remain the development prototype's active appearance. Release scripts must have their destination/runtime identity reviewed, and assets must pass the explicit pre-distribution review. Do not publish unreviewed Coucou-branded installers or run an upstream-targeting release script for this product.

## Attribution and assets

Based on [Coucou by Louis Raillé](https://github.com/louis-cfm/coucou), audited at commit 3cc3333203f60f63326ee949b7b86c7549992a1f. Source reuse follows the [MIT license](LICENSE). Coucou/Mochi names, character artwork/expressions/animations, icons, sounds and media have [separate asset restrictions](LICENSE-ASSETS.md). Initially retained development assets do not grant distribution rights. Before any public/distributable release, perform an explicit asset/license review and either obtain appropriate rights or replace the affected assets with original ones. This release gate is separate from the initial retained UI prototype.

This project is independent and is not presented as an official OpenAI or Coucou product.
