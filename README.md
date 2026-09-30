# Anti-Scrolling-Notch

A planned Windows companion for OpenAI Codex, inspired by Coucou's top-of-screen island workflow.

Repository: [YashwanthDevelops/Anti-Scrolling-Notch](https://github.com/YashwanthDevelops/Anti-Scrolling-Notch).

## Current status

The audited Coucou source and its Git history have been imported as the development baseline. Planning documentation is being published; the Windows build and Codex feature implementation are pending. The inherited application still implements Coucou/Claude behavior. This repository does not yet contain a completed Codex companion.

## Project documentation

- [Updated implementation plan](docs/implementation-plan.md)
- [Execution ledger](docs/execution-ledger.md)
- [Complete Coucou feature and architecture audit](docs/coucou-analysis.md)
- [Inventory of all audited repository files](docs/coucou-source-inventory.md)
- [Source provenance and licensing](docs/source-provenance.md)
- [Original upstream README](docs/upstream-README.md)

## Implementation approach

Use the existing Windows Tauri/Rust/TypeScript shell, then add independent Codex session monitoring, managed Codex chat, actual approvals/questions, GitHub PR and CI updates, Windows notifications, process/connection health, and truthful file preparation. Preserve useful optional service integrations.

Application builds and compatibility probes come before feature claims. Existing desktop control remains capability-gated. The implementation plan specifies each stage's acceptance conditions.

## Development and publication

The Windows project is in the windows directory. Read [AGENTS.md](AGENTS.md) and the implementation plan before working. Make small verified commits and push each completed change to this repository. The upstream remote is used to fetch and review Coucou changes; its push URL is disabled in the development checkout.

Inherited release scripts, app identities and assets are references until adapted. Do not publish Coucou-branded installers or run an upstream-targeting release script for this product.

## Attribution and assets

Based on [Coucou by Louis Raillé](https://github.com/louis-cfm/coucou), audited at commit 3cc3333203f60f63326ee949b7b86c7549992a1f. Source reuse follows the [MIT license](LICENSE). Coucou/Mochi names, character artwork/expressions/animations, icons, sounds and media have [separate asset restrictions](LICENSE-ASSETS.md). Anti-Scrolling-Notch will use its own identity and assets before distribution.

This project is independent and is not presented as an official OpenAI or Coucou product.
