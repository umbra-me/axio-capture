# Axio Capture handoff

For the September 5 implementation and its remaining gates, see
[the current change set](change-set-2026-09-05.md). Earlier verification entries
below apply to their recorded revisions, not automatically to this change.

The pick-up-here page. Written 2026-09-02 at the 0.1.0 release. If a fact
here disagrees with the code or the release page, the code and the release
page win; update this file.

## What it is

A ShareX-style region screenshot tool with an annotation editor, for macOS,
Windows and Linux. Built as a subproduct of Axio, the developer-tools brand,
because the intended differentiator is agent-facing: an MCP server,
structured capture metadata, accessibility-tree capture, redaction. None of
that exists yet. By owner decision the first cut is the ShareX basics, and
they ship first.

Stack: Tauri 2, Rust for everything that touches the OS, plain TypeScript
and Vite for the two web pages (overlay, editor). No frontend framework.
Apache-2.0, public from day one.

## Where everything is

| Thing | Location |
| --- | --- |
| Source | `umbra-me/axio-capture`, branch `main`; checkout at `brands/axio/repos/axio-capture` under the Umbra control plane |
| Workspace declaration | `brands/axio/workspace.yaml` and `workspace.lock`, component `axio/capture`, `required: false`, pinned by full commit |
| Control-plane records | `catalog/repositories.json` (`product-app`, public, no production dependency), `docs/repository-registry.md`, `docs/project-status.md` (Axio → Axio Capture), `docs/changelog.md` |
| Release | https://github.com/umbra-me/axio-capture/releases/tag/v0.1.0, tag at `4180fa2` |
| Update feed | `https://github.com/umbra-me/axio-capture/releases/latest/download/latest.json`, declared in `src-tauri/tauri.conf.json` under `plugins.updater` |
| Updater signing key | Private: `~/.tauri/axio-capture.key` on the owner's Mac (no password); backed up as a note in Proton Pass, vault **Umbra**, "axio-capture updater signing key"; set as repository secret `TAURI_SIGNING_PRIVATE_KEY`. Public half is in `tauri.conf.json`. **Losing the private key means no installed copy can ever update again.** |
| Code-signing identity | `Developer ID Application: KIAN PHILIP WATKINS (N7447R8KW2)` in the owner's login Keychain; verified September 5 signed build, expires September 6, 2031. The Apple Silicon 0.1.1 candidate is notarized/stapled through the reviewed one-off release workflow. CI still has no Developer ID certificate secrets; its macOS outputs require replacement or separate signing/notarization. |
| Installed copy | `/Applications/Axio Capture.app` on the owner's Mac, a local signed build of 0.1.0 |
| Settings file | `~/Library/Application Support/me.umbra.axio-capture/settings.json` (macOS); platform config dir elsewhere |
| Captures | `~/Pictures/Axio Capture` by default; configurable |

## How to work on it

```bash
cd brands/axio/repos/axio-capture
pnpm install                    # esbuild approval is in pnpm-workspace.yaml (pnpm 10+)
pnpm app                        # dev: Vite + debug build; app lives in the menu bar
cd src-tauri && cargo test && cargo clippy --all-targets && cargo fmt -- --check
pnpm typecheck && pnpm build    # frontend
```

A running app takes a second launch as a command: `axio-capture` starts a
capture, `axio-capture --open shot.png` opens a file in the editor. That is
also how to poke it from a terminal.

Signed local release, which is the form that keeps the macOS Screen
Recording grant across rebuilds:

```bash
APPLE_SIGNING_IDENTITY="Apple Development: KIAN PHILIP WATKINS (48XQCW9X5U)" \
  TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/axio-capture.key)" \
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" pnpm app:build
```

Install it with quit, `rm -rf "/Applications/Axio Capture.app"`, `cp -R` from
`src-tauri/target/release/bundle/macos/`, `open`. If the DMG step fails on a
stale `/Volumes/dmg.*` mount, eject it or pass `--bundles app`.

## How to release

1. Bump `version` in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`,
   `package.json`; add a `CHANGELOG.md` entry; commit and push.
2. `git tag -a vX.Y.Z -m "Axio Capture X.Y.Z" && git push origin vX.Y.Z`.
3. `.github/workflows/release.yml` builds macOS (aarch64, x86_64), Windows
   and Linux (Ubuntu 24.04), signs the updater artifacts, and opens a
   **draft** release with `latest.json`. About eight minutes.
4. Publish: `gh release edit vX.Y.Z --draft=false --latest --notes-file …`.
   Installed apps poll `releases/latest`, so the draft is invisible until
   this step.
5. In the control plane: move `brands/axio/workspace.lock` to the release
   commit, commit and push the workspace, then move the gitlink, note the
   release in `docs/changelog.md`, commit and push.

Re-tagging is acceptable only while the release is still a draft; that is
how the two CI fixes on the way to 0.1.0 were handled.

## Architecture in one screen

Read `AGENTS.md` for the module map. The shape:

- **Capture** (`capture.rs`): xcap grabs every monitor; each is PNG-encoded on
  its own thread. Selection mapping from CSS pixels to image pixels goes
  through the image size and the overlay's viewport size, so per-platform
  DPI models never leak.
- **Overlay** (`overlay.rs`): one borderless always-on-top window per
  monitor, positioned through the *builder* in logical units (setting
  position then size after creation pushes the top edge off-screen on
  macOS), raised to screen-saver level to cover the menu bar. Revealed only
  once the page has drawn the frozen screen.
- **Editor** (`src/editor/main.ts`): an immutable `shapes` array re-rendered
  onto the bitmap; undo is a stack of snapshots; blur pixelates whatever is
  already composited under it. Closing destroys the window; the capture
  survives in `AppState`.
- **IPC** (`commands.rs`, `src/shared/ipc.ts`): images cross as raw bytes,
  never base64; export takes a raw PNG body with the action in a header.
- **Settings** (`settings.rs`): one JSON file, `#[serde(default)]` on every
  field. `apply_system` pushes the login item and the Dock policy.
- **Naming** (`naming.rs`): `%year%/%month%/shot-%time%` patterns; unknown
  tokens survive verbatim so typos are visible.
- **Updater** (`updater.rs`), **permission** (`permission.rs`), **install**
  (`install.rs`), **hotkey**, **tray**, **cli**: each a small file.

## Decisions and why

- **Axio, not Zenco.** The brand is developer tools and agents; Deck and
  Analyst set the precedent for optional workstation components. The retired
  `i.zenco.cc` upload host (ADR 0016) is history, not a home.
- **Public.** Adoption for a ShareX alternative comes from being public, and
  agent-facing features are stronger if third parties can build on them.
- **`CmdOrCtrl+Shift+2`.** `⌘⇧3/4/5` are the system's; `⌘⇧A` collided with
  Chrome, VS Code, Slack and Xcode; `2` is free on every platform and
  crosses over as one string.
- **Menu-bar app, Dock icon never (default).** The while-editor-open mode
  works but every switch to a regular app parks a tile in the Dock's
  recents, which the owner did not want. `LSUIElement` in
  `src-tauri/Info.plist` plus the initial activation policy set on `App`
  in `setup` (tao applies it after setup, so a runtime call there is
  overwritten).
- **Close destroys.** Hidden windows lingered; the owner wanted them gone.
- **Signed with an Apple Development identity.** macOS ties the Screen
  Recording grant to the code signature; an ad-hoc signature changes every
  build and silently loses the grant while the toggle still looks on.
- **Permission recovery automated around the one toggle.** `tccutil reset`
  of the app's own entry, the system prompt, the settings pane, a poll for
  the grant, and a self-relaunch, because a running process never picks a
  new grant up.

## What is proven and what is not

Proven on the owner's Apple Silicon Mac: capture, overlay placement,
permission recovery, editor tools, settings, real close, menu-bar mode,
signed local build. In CI: all four platform builds and the release feed.

Not run anywhere: the Windows and Linux installers. Wayland is the least
certain (portal capture, GlobalShortcuts portal for the hotkey).

Proven September 5: an isolated 0.1.0 fixture accepted a signed localhost update
to 0.1.1 and rejected a tampered archive. The final Apple Silicon candidate is
Developer ID signed, notarized, stapled and accepted by Gatekeeper. These checks
do not establish public-feed delivery, Intel notarization or installer runtime.
Launch at login, notifications and the move-to-Applications offer remain manual
gates. See the dated change-set receipt for exact provenance.

## Next steps, in order

1. Prepare the v0.1.1 cross-platform draft. Replace its Apple Silicon CI output
   with the verified notarized candidate and fresh updater signature, updating
   the matching feed entry. Intel and both DMGs now pass notarization; the verified draft awaits final
   release-coordinator review before public promotion.
2. Run Windows and Linux installers on real machines. Verify Windows selection,
   Linux X11, Wayland portal capture/hotkey, mixed-DPI and attachment import.
3. Verify public-feed upgrade and cancellation/refusal, plus remaining macOS
   screen-selection and system-integration behavior in an isolated fixture.
4. Then the roadmap the product was started for, roughly in this order:
   window and full-screen capture; the MCP server and CLI mirror; the
   structured sidecar and model-ready output profile; accessibility-tree
   capture with numbered markers; redaction before handoff; an event stream;
   recording; uploaders with ShareX `.sxcu` compatibility. The first-party
   upload destination question from ADR 0016 (Worker or Compose service on
   the VPS) comes back with the uploaders.

## Traps worth knowing before touching anything

- pnpm 11 ignores `package.json`'s `pnpm` field; build-script approval is
  `allowBuilds` in `pnpm-workspace.yaml`. Deck still uses the old field.
- `tauri build` needs `TAURI_SIGNING_PRIVATE_KEY` as the key *contents* and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` set even when empty; the `_PATH`
  variant is advertised and ignored, and an unset password makes the CLI
  prompt and die without a TTY.
- The Linux CI job needs Ubuntu 24.04 (xcap's PipeWire bindings) and the
  Mesa and Wayland dev packages; the workflow's `apt-get` line is the
  authoritative list.
- Plugin builder methods that exist on one platform only (`macos_launcher`)
  must sit behind `cfg` or the Windows job fails to compile.
- macOS returns a wallpaper-only image, not an error, without the Screen
  Recording grant.
- A previous build's default can be persisted into the user's settings file
  by the editor's preference writes. Changing a default does not change an
  existing file; check the JSON when behaviour looks stale.

## Portable updater archive acceptance

Both downloaded final archives contain no AppleDouble sidecars. Test portable
extraction with Python 3.13 or later; macOS tar alone can merge sidecars and hide
a broken archive. Set ARCHIVE to the downloaded tar and EXTRACTED to a fresh
local temporary directory, then run:

```sh
python3.13 - "$ARCHIVE" "$EXTRACTED" <<'PY'
import pathlib, sys, tarfile
with tarfile.open(sys.argv[1]) as archive:
    assert not any(pathlib.PurePosixPath(m.name).name.startswith("._") for m in archive.getmembers())
    archive.extractall(sys.argv[2], filter="data")
PY
codesign --verify --deep --strict "$EXTRACTED/Axio Capture.app"
xcrun stapler validate "$EXTRACTED/Axio Capture.app"
spctl --assess --type execute "$EXTRACTED/Axio Capture.app"
```

Repeat for both architectures and independently verify each updater signature.
The September 5 receipt records the downloaded checks, asset hashes and preserved
feed entries. Repacking always requires a fresh updater signature.
