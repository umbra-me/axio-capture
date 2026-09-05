# Axio Capture (crate: `axio-capture`)

Tauri v2 desktop app: a region screenshot tool with an annotation editor, for
macOS, Windows and Linux. Rust owns the pixels and every side effect; the two
web pages only draw. Optional component `axio/capture` of the Axio workspace:
it installs onto a workstation and deploys nothing, like Deck and Analyst.

`docs/handoff.md` is the pick-up-here page: locations, keys, release
procedure, decisions, verification state, next steps. Keep it true.

## Commands

```bash
pnpm install                    # once; esbuild's postinstall is approved in pnpm-workspace.yaml
pnpm app                        # tauri dev: the app against the Vite dev server
pnpm app:build                  # tauri build: installers under src-tauri/target/release/bundle
pnpm lint                       # ESLint source checks
pnpm typecheck                  # standalone TypeScript check
pnpm test                       # frontend behavior tests (Vitest/jsdom)
pnpm build                      # frontend only: tsc --noEmit && vite build
cd src-tauri && cargo check     # fastest Rust loop
cd src-tauri && cargo test      # selection mapping and crop tests
cd src-tauri && cargo clippy --all-targets
cd src-tauri && cargo fmt -- --check
```

A running app accepts a second launch as a command: `axio-capture` alone
starts a capture, `axio-capture --open shot.png` opens a file in the editor.

A signed local release on macOS (the form that keeps the Screen Recording
grant across builds):

```bash
APPLE_SIGNING_IDENTITY="Apple Development: …" \
  TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/axio-capture.key)" \
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" pnpm app:build
```

## Architecture

- `src-tauri/src/lib.rs` wires the plugins (single instance, dialog, global
  shortcut), the tray and the one hotkey, `CAPTURE_SHORTCUT`. Closing the
  editor never quits: `RunEvent::ExitRequested` is prevented because this is a
  tray application.
- `capture.rs` is the only module that touches `xcap`. `grab_screens`
  captures every monitor and encodes each to PNG on its own thread.
  `map_selection` turns a CSS-pixel selection into image pixels through the
  image size and the overlay's viewport size, so the platform's DPI model
  never leaks past this file. **Geometry units differ by platform**: xcap
  reports macOS monitors in logical points and Windows/X11 monitors in
  physical pixels, and `overlay.rs` places windows with `LogicalPosition` on
  macOS and `PhysicalPosition` elsewhere for exactly that reason.
- `overlay.rs` is the capture flow: hide the editor, freeze the screens, one
  borderless always-on-top window per monitor (`overlay-<n>`, opened hidden
  and revealed by `overlay_show` once the page has drawn, so nothing flashes),
  then `finish` tears them down. On macOS the overlay is raised to
  screen-saver level so it covers the menu bar and the Dock. `show_editor`
  opens or reuses the single `editor` window; closing it destroys it (the
  owner did not want a hidden window lingering), and only the capture
  survives in `AppState`.
- `commands.rs` is the IPC surface. Images cross the bridge as raw bytes
  (`tauri::ipc::Response`), never base64. `export_png` takes the editor's PNG
  as a raw request body and reads the action from the `x-axio-action` header;
  it is `async` so the save dialog and disk writes never run on the main
  thread.
- `export.rs` owns the clipboard (arboard; on Linux a thread keeps the offer
  alive) and the captures folder, `~/Pictures/Axio Capture`.
- `cli.rs` parses `--capture` and `--open <path>` for both the first launch
  and forwarded second launches.
- `settings.rs` is the one JSON settings file (`app_config_dir()/settings.json`)
  with `#[serde(default)]` everywhere so older files keep loading; `validate`
  is the only gate, and `set_settings` re-registers the hotkey through
  `hotkey.rs` before persisting, restoring the previous one on failure. The
  `after_capture` setting is honoured in `overlay_confirm`; `close_on_*` in
  `export_png`, which hides the window from Rust so the page needs no window
  permission. The editor's last tool, colour and width are saved from the page
  through the same command, debounced. `apply_system` pushes the two settings
  the OS holds a copy of (login item via `tauri-plugin-autostart`, Dock icon
  via the activation policy) at startup and whenever they change.
  `apply_dock_icon` is also called when the editor window is created and
  destroyed, because the `dock_icon` while-open mode shows the icon only while
  that window exists (not the default: each switch to Regular puts a tile in
  the Dock's recents); the policy is set to Regular *before* the window is
  built so macOS will focus it. The **initial** policy must go through
  `App::set_activation_policy` in `setup`: tao applies that value in
  `applicationDidFinishLaunching`, after `setup`, so a runtime call made
  there is overwritten and the Dock icon reappears. `src-tauri/Info.plist`
  additionally declares `LSUIElement`, so LaunchServices itself starts the
  app as a menu-bar app; the runtime policy then only ever adds the icon.
- `naming.rs` expands file-name patterns (`%year%/%month%/shot-%time%`). It
  is pure except `resolve`, which touches the folder to satisfy `%n%`. Add a
  token in `TOKENS` **and** `token()`; the panel lists `TOKENS` and the tests
  pin the format of every existing one. Unknown tokens are kept verbatim by
  design, so a typo shows up in the file name instead of vanishing.
- Frontend is plain TypeScript and Vite, no framework. `overlay.html` +
  `src/overlay/main.ts` draw the frozen screen, the dimmed selection and the
  size label. `index.html` + `src/editor/main.ts` hold the annotation editor:
  an immutable `shapes` array re-rendered onto the bitmap after every change,
  with undo as a stack of snapshots. Blur pixelates whatever is already
  composited beneath it, so it can cover earlier annotations. `src/shared/ipc.ts`
  is the typed `invoke` layer; keep every command name in that one file.

- `updater.rs` polls the endpoint in `tauri.conf.json` (`plugins.updater`)
  twenty seconds after launch and every six hours, off in debug builds and
  when `check_updates` is false; the tray item is the interactive path. The
  dialogs are blocking, so every entry point runs on a plain thread and uses
  `block_on` for the async plugin calls. `install.rs` is the macOS
  move-to-Applications offer that keeps the updater pointed at one copy.
- `.github/workflows/release.yml` builds and signs on a `v*` tag into a draft
  release with `latest.json`; the public repo gets Actions minutes for free.
  Linux builds on `ubuntu-24.04`: xcap's PipeWire bindings do not compile
  against 22.04's libpipewire 0.3.48, and its Wayland path links `gbm`,
  `drm` and `EGL`, so the apt line in the workflow is the authoritative
  Linux dependency list. Plugin builder methods that exist on
  one platform only (`macos_launcher`) must stay behind `cfg`, or the
  Windows job fails to compile.

## Verification

Treat these as the truth about what has been proven, and move items up as
they are exercised:

- Proven on macOS (owner, Apple Silicon, 2026-09-02): capture, overlay
  placement, permission recovery, editor tools, settings, real close,
  menu-bar mode, signed local build, CI release for all platforms.
- September 5 native checkpoint: Windows 11 and Debian/aarch64 pass tests,
  strict Clippy and standalone debug builds. The isolated Linux X11 editor
  renders and exports a verified PNG/manifest attachment. Windows runtime,
  Wayland capture/hotkey and mixed-DPI remain unverified. Historical 0.1.0
  CI packages do not establish 0.1.1 installer or updater acceptance.
- September 5 isolated macOS fixture: genuine 0.1.0 source updated through a
  signed localhost feed to 0.1.1, restarted and exported a verified attachment.
  Production-feed update and cancellation/refusal remain open; a tampered update was rejected.
- September 5 release candidate: Apple Silicon 0.1.1 is Developer ID signed,
  notarized and stapled; local and runner Gatekeeper checks accept it. The final
  archive has a fresh independently verified updater signature. Intel and both DMGs are now notarized/stapled too; all four CI jobs passed
  and verified candidates replaced the draft macOS assets. v0.1.1 is published; all seven unique public updater payload signatures and
  both public DMG hashes are verified. Manual runtime cases remain distinct. See the dated change-set receipt.
- Never exercised: launch at login,
  notifications, the move-to-Applications offer, the settings panel's
  interactions inside the real webview (layout and the file round-trip were
  checked from a browser and from the written JSON).
- Automated: 12 Rust tests (selection mapping, crop, settings defaults and
  validation, file-name patterns and `%n%`), clippy, fmt, `tsc`, and the
  release workflow on a `v*` tag.

## Gotchas

- `tauri build` on macOS sometimes leaves the DMG's temporary image mounted
  (`/Volumes/dmg.xxxxxx`) after `bundle_dmg.sh` fails, and every later build
  then fails the same way while the `.app` is complete. Eject it
  (`hdiutil detach /Volumes/dmg.* -force`) or build with `--bundles app` when
  only the bundle is needed.

- `bundle.createUpdaterArtifacts` is on, so `tauri build` fails at the very
  end without `TAURI_SIGNING_PRIVATE_KEY` (the key's contents; the CLI 2.11
  ignores the `_PATH` variant despite `signer generate` advertising it) **and**
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (empty string for this key; unset, the
  CLI prompts and dies without a TTY) in the environment. The private key is not in this repository and must never be;
  the public half is in `tauri.conf.json`.

- macOS returns a wallpaper-only image, not an error, when Screen Recording
  permission is missing, so `permission.rs` preflights with
  `CGPreflightScreenCaptureAccess` before every capture. When it is missing it
  runs `tccutil reset ScreenCapture <bundle id>` (drops a grant recorded
  against an older signature), calls `CGRequestScreenCaptureAccess` for the
  system prompt, opens the pane, polls the preflight for up to five minutes,
  and calls `app.restart()` once it flips, because a running process never
  picks a new grant up.
  A dev build (`pnpm app`) is judged as the terminal or the dev binary; a
  packaged build as the app. **The permission is tied to the code signature**:
  the bundle is only ad-hoc signed unless `APPLE_SIGNING_IDENTITY` is set at
  build time, and an ad-hoc signature changes with every build, so macOS
  silently drops the grant (the toggle can still look on). Sign with a real
  identity for a stable grant, or remove and re-add the entry after a rebuild.
- The tray icon on macOS is a template (`icons/tray/`), monochrome black with
  alpha; the coloured app icon would render as a filled square. Regenerate
  both sets with `pnpm tauri icon` from `icons/source.svg` and `icons/tray.svg`;
  delete the `android/` and `ios/` outputs.
- `tauri.conf.json` declares no windows. Both windows are created from Rust,
  and the capability file lists them by label (`editor`, `overlay-*`).
- Dropping a file on the editor is handled in Rust (`WindowEvent::DragDrop`);
  the JavaScript `drop` listener only fires in a plain browser, where it lets
  the editor be developed against `pnpm dev` without the app.
- Wayland captures go through the desktop portal (xcap); the global shortcut
  needs a compositor that implements the GlobalShortcuts portal. X11 works
  without either. Multi-monitor mixed-DPI on Linux is untested.

Current integration, schema and release boundaries: [September 5 change set](docs/change-set-2026-09-05.md). Keep its completed and pending validation separate.
