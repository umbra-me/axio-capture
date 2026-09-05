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

Remaining gates: supported public release packaging and production-feed updater
acceptance; Windows native editor/selection; macOS screen selection; Wayland and
mixed-DPI; companion-app import; public
macOS Developer ID signing/notarisation. Local build success does not close these
gates. No release tag or updater feed was published by this checkpoint.

## Isolated macOS updater and editor acceptance

Published source checkpoint: `62dbcd9fca122f20d925dd08d2e3fa7003420582`.
A genuine `v0.1.0` source snapshot and the 0.1.1 runtime source were built with
the same external updater key and Apple Development identity. The fixture changed
only the product name, identifier (`me.umbra.axio-capture-acceptance-20260905`),
and updater endpoint to a loopback HTTP feed with an explicit fixture-only
insecure-transport override. None of those overrides entered repository source.
Settings/save paths were isolated; launch at login was disabled and the
move-to-Applications prompt suppressed for this disposable copy.

The native 0.1.0 editor rendered and saved the bundled 128×128 PNG. Its updater
fetched `/latest.json` and the signed 0.1.1 archive from `127.0.0.1:16907`,
installed it and restarted. The same fixture bundle's plist then reported 0.1.1;
`codesign --verify --deep --strict` passed. The restarted native editor exposed
**Export attachment**, which produced a verified 128×128 PNG and manifest with
SHA-256 `bede776b5987c1a992203540a8d8d60773bdc2d8e0b7365f2f56c6096e648a6f`.
The archive SHA-256 was
`79ad428439efe0499ee2bca4a0ced8eda74597d4440ea91d3986c3284eac9290`.

The updater refuses a launch through macOS's `/tmp` symlink; launching the same
fixture via canonical `/private/tmp` resolved that fixture-path restriction.
The update dialog appeared while settings was closing, so the precise prompt and
refusal interaction were not captured. Invalid-signature refusal is also not
claimed by this successful-install check. The real installed application remained
0.1.0 in `/Applications`, with its original process running. This proves an
isolated signed installer path and new native editor export, not public
Developer ID signing, notarization, production-feed delivery or screen permission
acceptance.

Reproducibility artifacts are under `/private/tmp/axio-capture-updater-20260905`:
`old/`, `new/`, `installed/`, `feed/`, and `captures/`. Build/runtime/HTTP logs use
`/tmp/axio-capture-updater-*-20260905.log`. These are temporary evidence paths,
not durable release artifacts. The private key was read directly into the build
process environment and was not copied into the fixture or logs.

### Signature rejection checkpoint

All 25 runtime/frontend/package/lock files used by the 0.1.1 fixture were compared
byte-for-byte with published `62dbcd9fca122f20d925dd08d2e3fa7003420582` and
matched. Fixture configuration overrides remain the only runtime configuration
differences. A preserved old bundle now exists at
`/private/tmp/axio-capture-updater-20260905/old-bundle/Axio Capture Acceptance.app`.

An isolated Rust harness at `signature-check/` uses `minisign-verify` 0.2.5 and
the same `PublicKey::decode`, `Signature::decode`, `verify(data, signature, true)`
sequence as tauri-plugin-updater 2.11.0. It accepts the signed archive and rejects
a copy with one changed byte: **The signature verification failed**. No private
key is needed by this verifier. Its lockfile and inputs are retained beside the
harness; output is `/tmp/axio-capture-updater-signature-20260905.log`.

The native negative test was reset to 0.1.0 and fetched the feed, but the
confirmation is hosted by macOS `com.apple.UserNotificationCenter`.
Computer Use explicitly rejected access to that app for safety reasons; no
alternate automation bypass was attempted. At that checkpoint, native rejection
awaited user interaction with the isolated **Axio Capture Acceptance** dialog;
the user-assisted completion is recorded below.
Before that interaction, version stayed 0.1.0 and the binary SHA-256 remained
`36aec777469de4b8969dff6d631e330eb2b8bad0a41b6fefd761694f2d3ee20e`.
The tampered archive SHA-256 is
`27b446420d369d480970097ce15a5c707a44ba4276d5eec626320cf9bf30b0d0`.
`negative-before.json`, `valid-0.1.1.app.tar.gz`, the altered `feed/` archive,
and `/tmp/axio-capture-updater-negative-20260905.log` retain that boundary.
The independent verifier alone did not establish the native UI result.

## Frontend behavior test gate

`pnpm test` now runs 4 Vitest/jsdom tests covering the real editor HTML/module with mocked native bridges: attachment rendering and reveal, literal hostile-path text, duplicate-export suppression, failure retry, cancelled saves, and settings cancellation/error recovery.
These exercise rendered controls and state transitions rather than source-text
patterns or copies of Rust policy. Canvas pixel rendering and operating-system
effects remain the responsibility of native acceptance; test native bridges are
explicitly mocked. Test dependencies are development-only and pinned by the
standalone component lockfile.

`pnpm test` and `pnpm build` passed on macOS and the isolated Windows checkout
(4 tests, no skipped tests). Windows logs are
`/tmp/axio-capture-windows-frontend-20260905.log`. No native source, installed app,
updater fixture, or public release was changed by this test-only checkpoint.

## User-assisted native signature rejection — 16:08 AWST

The user supplied a screenshot captured on 2026-09-05 at 16:08:29 AWST showing
the isolated updater's native **Update failed** dialog, followed by **The update
could not be installed.** and **The signature verification failed**. This is the
previously blocked negative-fixture interaction, now observed after user input;
no automation bypass was used.

The coordinating task then read the installed isolated fixture's
`CFBundleShortVersionString` and confirmed it remained **0.1.0**. Together with
the existing tampered-archive identity above, this closes the native negative
signature-verification gate: the tampered 0.1.1 update was rejected and the
installed version did not advance. A post-attempt full binary comparison was
not performed, so this receipt does not claim one.

Screenshot evidence is retained locally as
`Screenshot 2026-09-05 at 4.08.29 pm.png` in the user's temporary screenshot
folder; its SHA-256 is `d0883f1756a44eff26681ff9f9a3555960e83a6d5c62e3cf5559cad0ec31cc2e`.
The image includes unrelated desktop content and is not copied into this public
repository. The installed fixture remains separate from the real application
and public updater feed.

This result does not establish cancellation/refusal before installation,
production-feed delivery, Developer ID signing or notarization. Public release
packaging, Windows editor/selection, macOS screen selection, Wayland/mixed-DPI
and companion-app attachment import remain the separately recorded gates.
Only this acceptance receipt changed; no runtime source, signing asset, public
release, updater feed or installed application was changed by this documentation
checkpoint.
