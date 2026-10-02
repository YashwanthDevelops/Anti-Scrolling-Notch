# Stage 5 packet 5 — bounded history and durable preferences

| Field | Value |
|---|---|
| Packet | Stage 5, packet 5: CORE-04 storage foundation |
| Release profile | Monitor MVP foundation |
| Task IDs | CORE-04 only |
| Branch / PR | `work/stage-5-core-04-storage` / PR pending |
| Dependencies | Integrated CORE-01–03 and the exact four-event Codex CLI observer contract; PR #18 merge `237728551ae06207d56f14f4ec20395fb1d4463d` |
| User outcome | Verified anonymous observer events can be retained locally within explicit count, payload, record and database-size bounds, while preferences survive safe migration and logs redact common secrets and local paths. |
| Status | Implementation and local checks complete; hosted CI, review and integration pending |

## In scope

- Add a SQLite history store under `%LOCALAPPDATA%\Anti-Scrolling-Notch\history.sqlite3` with a transactional `user_version` migration, no-follow open, a 10,000-row cap, a 16 MiB retained-payload cap, a 64 KiB per-record cap, a 32 MiB database page cap, and a 500-row read-page cap.
- Retain only the exact `wire_version`/event pair received by the existing Codex observer today. Its source remains anonymous: do not add session IDs, turn mapping, payload fields or any unverified hook event.
- Provide a storage API for existing normalized broker envelopes. Omit free-text fields by default; when `retainHistoryContent` is enabled, retain them only after credential/token and Windows-path redaction. This API is not connected to a production monitor or the frontend in this packet.
- Version the existing `%APPDATA%\Anti-Scrolling-Notch\settings.json` format, migrate its current unversioned form, use same-directory temporary-file replacement, and leave malformed or future-version files untouched. Default `retainHistoryContent` to false in Rust and the existing settings UI.
- Redact common credential/token formats and Windows/UNC paths from log lines, normalize control characters, bound incoming/line sizes, serialize rotation in-process, and retain at most a 1 MB current log plus one 1 MB rotated file.
- Continue the existing per-user local storage namespace. Do not alter secrets in Windows Credential Manager, hook installation ownership, inherited Coucou assets, or the event wire contract.

The SQLite driver is `rusqlite` with its `bundled` feature, which compiles its own SQLite instead of depending on a separately installed machine library; see the [upstream rusqlite build guidance](https://github.com/rusqlite/rusqlite#notes-on-building-rusqlite-and-libsqlite3-sys).

## Excluded

- Stage 6 production Codex monitor work or any new/unsupported Codex event interpretation.
- Converting anonymous hook names into sessions, turns, tool items, request state or Git state.
- Wiring normalized broker events to the view store, exposing history UI, or claiming CORE-03/CORE-10 complete.
- Event deduplication/coalescing, request routing/resolution, timer behavior, Stage 7 GitHub, and Stage 4 SHELL-08/09.
- Replacing/redesigning Coucou/Mochi UI, animations, sounds or asset bytes.

## Acceptance evidence

- Unit tests prove transactional history migration, future-schema rejection without replacement, persistence across reopen, exact accepted hook observation storage, bounded cursor reads, count/payload/record/page limits, and default-off versus opt-in redacted broker text.
- Settings tests prove legacy migration, atomic replacement without leftover temporary files, default-off content retention, and refusal to overwrite malformed or future-version files.
- Log tests prove credential/token/path/control-character redaction, line limits, rotation and backup bounds.
- Locked workspace tests, Clippy, an optimized Windows Rust build, TypeScript/Vite production build, scoped formatting, changed-document checks and `git diff --check` pass. The repository's exact release profile (`lto = true`, one codegen unit) crashed local rustc 1.98.1 with Windows `STATUS_ACCESS_VIOLATION`; the same locked release build passed with command-line-only overrides disabling LTO and using eight codegen units. No project profile was changed; hosted CI is pending.
- State that the current live observer supplies only four anonymous event names; session/turn history is unavailable until its separately scoped adapter and UI integration.

## Local validation outcome

- `cargo test --workspace --locked -- --skip files::tests::ingest_copies_and_never_overwrites`: 75 passed, 2 ignored, 1 filtered. The filtered test is a pre-existing file-ingest test whose default temporary-directory creation fails with Windows access denied in this sandbox; its source is unchanged.
- `cargo clippy --workspace --all-targets --locked`: passed with one inherited `needless_range_loop` warning in `src-tauri/src/hooks.rs:467`.
- `cargo build --release -p anti-scrolling-notch --locked --config profile.release.lto=false --config profile.release.codegen-units=8`: passed. The exact repository release profile separately failed when rustc exited with `0xc0000005` while compiling the app crate; this is recorded as an unresolved local toolchain/build limitation, not a source diagnostic or passing default-profile build.
- `npm run build --ignore-scripts`: TypeScript check and Vite production bundle passed. The first sandboxed invocation was denied access to the parent directory by esbuild; the approved elevated rerun passed.
- `npm run test:resources`: 2/2 passed; inherited visual/icon references and the byte-for-byte sound asset set remain intact.
- Scoped rustfmt check, `docs/check-markdown.ps1`, and `git diff --check`: passed.
- Hosted CI, PR review and integration are pending. The current Codex observer still supplies only the exact four verified event names; normalized session/turn history has no producer until later packets.

## Limitations and recovery

- Current hook history has event names and local receive timestamps only. It contains no prompt, path, session ID, turn ID, or inference about lifecycle success.
- The turn-text control prepares the explicit preference for normalized broker records; the current observer does not send text, and the broker-event storage method is not connected to a producer in this packet.
- Unsupported/future SQLite schemas disable history without schema rewrite. History storage failure is logged locally while app startup and neutral observer behavior continue.
- Existing log content written by older versions is not rewritten retroactively. Newly written lines follow the new redaction and rotation policy.
- The database is local and is not encrypted by SQLite; OS-protected credentials remain separate and are never inserted into history.
