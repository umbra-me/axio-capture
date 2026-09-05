# Axio Capture

Current source changes and verification gates: [September 5 change set](docs/change-set-2026-09-05.md).

Region screenshots with an annotation editor, for macOS, Windows and Linux.
Press the hotkey, drag a rectangle, draw on it, copy or save. Part of the
[Axio](https://axio.sh) family of developer tools.

This is the first cut. It captures a region, lets you annotate it, and keeps
itself updated. Window and full-screen capture, recording, uploaders and the
agent-facing API (an MCP server, live capture control, redaction) are
later milestones, not missing pieces of this one.

## Status

0.1.0 is released for all three platforms, and this is what that claim rests
on as of 2026-09-02:

- **Verified on macOS**, by the owner on an Apple Silicon Mac: region capture
  and overlay placement, the Screen Recording recovery flow, the editor and
  its tools, settings persistence, real window close, and menu-bar mode. The
  signed bundle keeps its permission grant across rebuilds.
- **Built in CI, never run:** the Windows and Linux installers compile,
  bundle and sign, but nobody has launched them yet. Linux on Wayland is the
  least certain: capture goes through the desktop portal and the hotkey
  depends on the compositor implementing the GlobalShortcuts portal.
- **Wired, not exercised:** in-app updating (the feed is live, but nothing
  older than 0.1.0 exists to update from; the first `v0.1.1` tag is the
  test), launch at login, notifications, and the move-to-Applications offer.
- **Known rough edges:** macOS bundles are signed but not notarised, so
  another Mac needs right-click, Open on first launch; the local DMG step
  occasionally fails on a stale mount (CI builds the DMGs).

## Use

| Action | How |
| --- | --- |
| Capture a region | `Ctrl+Shift+2` (`⌘⇧2` on macOS), the tray icon, or run `axio-capture` again |
| Cancel a capture | `Esc` or right-click |
| Open an image in the editor | `axio-capture --open shot.png`, or drop a file on the editor |
| Settings, updates, captures folder | The tray menu |

The editor has arrows, lines, rectangles, ellipses, freehand pen, highlighter,
text, numbered steps and blur (pixelate). Hold `Shift` to constrain a shape.
Keys `A L R E P H T N B` pick tools; `Ctrl/⌘+Z` undoes; `Ctrl/⌘+C` copies the
result to the clipboard; `Ctrl/⌘+S` saves a PNG named by the file-name
pattern into the save folder; `Ctrl/⌘+Shift+S` picks a location. Closing the
editor discards its annotations; the capture itself stays until the next one,
so **Open editor** in the tray brings it back clean.

## Settings

The gear button in the editor, or **Settings…** in the tray menu, opens the
settings panel:

- **After a capture**: open the editor (default), copy to the clipboard, save
  to the folder, or both. The last three skip the editor entirely.
- **Close the editor after copying** (default on) and **after saving**
  (default off).
- **Save folder**, defaulting to `~/Pictures/Axio Capture`.
- **File name pattern**, relative to the save folder, where `/` creates
  subfolders. The default is `capture-%datetime%`; something like
  `%year%/%month%/%day%/%time%-%width%x%height%` sorts captures by day. The
  panel shows a live example and the full token list: `%year%`, `%yy%`,
  `%month%`, `%monthname%`, `%day%`, `%weekday%`, `%hour%`, `%hour12%`,
  `%ampm%`, `%minute%`, `%second%`, `%ms%`, `%date%`, `%time%`, `%datetime%`,
  `%unix%`, `%width%`, `%height%`, `%random%`, `%user%`, and `%n%` for the
  smallest number that keeps the name unused. Tokens are case-insensitive and
  `.png` is added when missing.
- **Capture shortcut**, in Tauri's accelerator syntax such as
  `CmdOrCtrl+Shift+2` or `Alt+F9`. It takes effect as soon as you save.
- **Launch at login**, a Login Item on macOS, a Run key on Windows, an
  autostart desktop entry on Linux.
- **Notify** with a desktop notification when a capture is copied or saved
  without the editor.
- **Dock icon** (macOS): never (default), always, or only while the editor is
  open. The default keeps Axio Capture a pure menu-bar app; the editor still
  opens and takes focus. The while-open mode makes macOS treat each editor
  session as a regular app launch and park a tile in the Dock's recents.

The last tool, colour and stroke width are remembered automatically. Settings
live in one JSON file in the platform config directory (on macOS
`~/Library/Application Support/me.umbra.axio-capture/settings.json`); the
panel shows the exact path.

macOS asks for Screen Recording permission the first time. The app checks
before every capture; when the permission is missing it resets its own stale
entry, triggers the system prompt, opens the Screen Recording pane, and once
you flip the toggle it relaunches itself to apply the grant. The one thing
macOS reserves for you is that toggle.

## Updates

Installed copies check GitHub Releases shortly after launch and every six
hours, and offer to update and restart when a newer version exists. **Check
for updates…** in the tray menu does it on demand, and the automatic check can
be turned off in settings. Every update artifact is signature-checked against
the public key in `src-tauri/tauri.conf.json` before it is installed.

On macOS the updater replaces whichever copy is running, so keep one: an app
launched from a disk image, Downloads, or a build folder offers to move
itself into `/Applications`, replacing the copy there.

### Releasing

1. Bump `version` in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and
   `package.json`, and commit.
2. Tag it: `git tag v0.1.1 && git push --tags`.
3. The release workflow builds macOS (Apple Silicon and Intel), Windows and
   Linux, signs the updater artifacts, and opens a **draft** release with
   `latest.json`. Review and publish it; the app polls `releases/latest`.

The workflow needs two repository secrets: `TAURI_SIGNING_PRIVATE_KEY`, the
contents of the private key, and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, empty
if the key has none. Losing the private key means no existing install can
ever update again, so keep it somewhere durable.

A local release build must be able to sign the updater artifacts too, and
on macOS it should also code-sign the bundle with a real identity, because
macOS ties the Screen Recording grant to the signature and an ad-hoc
signature changes with every build:

```bash
APPLE_SIGNING_IDENTITY="Apple Development: Your Name (TEAMID)" \
  TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/axio-capture.key)" \
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" pnpm app:build
```

`security find-identity -v -p codesigning` lists the identities available.
An Apple Development certificate is enough for a stable local install;
distribution to other Macs needs Developer ID plus notarisation.

## Build

Prerequisites: Rust 1.85+, Node 22+, pnpm, and the
[Tauri platform dependencies](https://tauri.app/start/prerequisites/) for your
OS (Xcode Command Line Tools on macOS; WebView2 and the MSVC toolchain on
Windows; `webkit2gtk-4.1`, `libappindicator3`, `librsvg2` and `patchelf` on
Debian-family Linux, plus `libxcb`, `libxrandr`, `libdbus-1`, `libpipewire-0.3`,
`libgbm`, `libdrm` and `libegl1-mesa` development packages for capture; the
release workflow has the exact `apt-get` line).

```bash
pnpm install
pnpm app            # run in development
pnpm app:build      # installers under src-tauri/target/release/bundle
```

Without `APPLE_SIGNING_IDENTITY` the macOS bundle is only ad-hoc signed; see
Releasing above for why that matters. Notarisation is not set up yet, so a
bundle from another Mac opens after a right-click, Open.

## Layout

```
src-tauri/src/      Rust: capture, overlay and editor windows, export, naming,
                    settings, hotkey, tray, CLI, updater, permission, install
src-tauri/Info.plist  LSUIElement: a menu-bar app with no Dock icon
src/overlay/        the selection overlay page
src/editor/         the annotation editor and settings panel
src/shared/ipc.ts   the typed command layer between the two sides
.github/workflows/  the tagged-release build
```

See `AGENTS.md` for the architecture and the platform gotchas, and
`docs/handoff.md` for the pick-up-here page: where everything lives, how to
release, what is proven, and what comes next.

## License

Apache-2.0. See `LICENSE` and `NOTICE`.
