# September 5 change set

## Attachment export and 0.1.1 preparation — 2026-09-05

The editor's **Export attachment** writes the final annotated PNG and a JSON
manifest beneath `Attachments/` in the configured save directory. The PNG is named
by SHA-256, and the manifest is `<digest>.axio-capture.json`. Both files must travel
together. Existing identical files are reused; differing bytes are refused.
The version 1 contract contains `schema_version`, `media_type: image/png`, `file`,
`sha256`, `width`, and `height`; exports accept PNGs up to 32 MiB.
In the companion desktop command palette, choose **Start Codex with Capture**,
select the manifest, then send your first prompt in the new terminal. Export
itself sends nothing externally. MCP, automatic redaction and uploaders remain
future work; this is a file handoff, not a live capture API.

Source versions in package.json, Cargo.toml, Cargo.lock and Tauri are 0.1.1.
The release workflow checks frontend build, Rust formatting/tests/Clippy and tag
agreement through `scripts/check-release.py` before packaging. Version 0.1.1 is
prepared, not tagged or published by this change. Existing 0.1.0 release/runtime
observations below are historical and do not prove the new attachment flow.

Scheduled update checks skip disabled polls without ending the worker, so enabling
checks later resumes at the next scheduled poll (up to six hours later). Debug
builds still do not schedule checks. The interactive tray action remains separate.

Remaining gates: an actual 0.1.0 → 0.1.1 download/install update, Windows/Linux
runtime acceptance, attachment export/import in the real apps, and public macOS
Developer ID signing/notarisation. No updater feed or installed app is changed
by publishing these source commits.

Validation on September 5: 12 Rust tests, Clippy with denied warnings, formatting, frontend build and v0.1.1 version agreement passed.
