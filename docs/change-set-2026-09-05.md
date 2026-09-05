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

## Native checkpoint

On September 5, macOS, Windows 11 and isolated Debian bookworm/aarch64 each
passed 12 Rust tests, strict all-target Clippy and a standalone `custom-protocol`
build. Formatting, frontend build and v0.1.1 version agreement passed. Windows
used an isolated temporary checkout/state directory. Linux ran in an isolated
OrbStack Docker container with WebKitGTK 4.1 and X11 dependencies; this is not
acceptance on a user's Wayland desktop.

The Linux native editor rendered the bundled 128×128 icon via `--open`. Clicking
**Export attachment** produced the PNG and schema-1 manifest with matching SHA-256
`a99a28064cc2e5887fa37c33d742ddbd4d0488138867b06a22d5ba82f250a492`.
The native status displayed the ready attachment path. This proves X11 editor
rendering and attachment export, but not screen selection, mixed-DPI, clipboard
ownership after exit, Wayland portals or companion-app import.

macOS 0.1.1 `.app`, updater `.app.tar.gz` and `.sig` artifacts were built using
the existing Apple Development identity and the external updater key at
`~/.tauri/axio-capture.key`. Key contents remain outside source and logs. No
Developer ID identity was available; the build skipped notarization. These are
local acceptance artifacts, not a completed public macOS release. The installed
0.1.0 application and public updater feed were left unchanged.

Remaining gates: source publication and supported release packaging; actual
0.1.0 → 0.1.1 signed download/install/restart; Windows native editor/selection;
macOS 0.1.1 editor/selection; Wayland and mixed-DPI; companion-app import; public
macOS Developer ID signing/notarisation. Local build success does not close these
gates. No release tag or updater feed was published by this checkpoint.
