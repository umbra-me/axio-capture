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


## Developer ID signing checkpoint

The owner issued a Developer ID Application certificate for team N7447R8KW2 on
September 5. Its public key matched the prepared private key; both were imported
into the login Keychain. The certificate expires September 6, 2031. Key material
is stored outside repositories and is not part of this receipt.

Source `31080b5e685dc46ca3f7e53b253d124655a6b950` produced the genuine
`me.umbra.axio-capture` 0.1.1 app and signed updater archive. Strict recursive
codesign verification passed, showing Developer ID Application, Developer ID CA
and Apple Root CA authorities, team N7447R8KW2, hardened runtime and trusted
timestamp September 5 at 16:30:16 AWST. Archive SHA-256:
`841da47ba15fca08fbb5896b0f17d6e2ab30703963084feabc6184647b6d7e3f`.

Build log: `/tmp/axio-capture-developer-id-build.log`. The app/updater artifacts
are under `src-tauri/target/release/bundle/macos/`. Notarization was explicitly
skipped because no notary authentication was configured. This checkpoint does
not establish notarization, stapling, Gatekeeper acceptance or public-feed
delivery, and did not replace the installed application.

## Notarized Apple Silicon candidate and fresh updater signature

Apple accepted submission `9f60e343-7718-4cf6-a305-64dd061fa229` in the reviewed
one-off [run 33956732732](https://github.com/notzenco/lexis-apple-build/actions/runs/33956732732),
workflow source `b638cc80951aa6094f168f75f974374965432b68`. Its decision was
Accepted / Ready for distribution, with no issues. The private input archive
matched the `841da47b…` hash above. Encrypted results were HMAC-verified before
decryption. Stapler validation, strict recursive codesign, and Gatekeeper
`source=Notarized Developer ID` passed on both the runner and the owner's Mac.

Final stapled updater archive SHA-256:
`478b3f282ee3b8ae1f97088cae1d9e1075388c4622e415f081c1cbe72658712e`.
The existing local Tauri updater key signed that final archive again; independent
`minisign-verify` 0.2.5 verification passed and a one-byte tampered payload was
rejected. The original pre-stapling signature must not be reused.

Protected local evidence/artifacts are under
`~/.local/share/axio-capture-notarization/20260905/run-33956732732/`:
`decrypted/capture-results/` contains the archive, fresh `.sig`, Apple JSON and
acceptance logs; `stapled/Axio Capture.app` is the verified app; `signature-proof/`
contains the separate updater proof. No signing keys are stored in this repo.

The signed runtime source remains `31080b5e685dc46ca3f7e53b253d124655a6b950`.
Every later change through this release-preparation checkpoint is documentation
only, verified by the Git file diff. Product versions remain 0.1.1. A v0.1.1 tag
triggers four jobs in the existing public release workflow and creates a DRAFT.
The Apple Silicon CI artifact must be replaced with this notarized candidate,
its fresh signature, and the matching feed entry. Do not publish the draft while
Intel macOS is ad-hoc/unnotarized or remaining platform/runtime gates are open.
This checkpoint does not replace `/Applications/Axio Capture.app` or establish
public-feed delivery. Windows/Linux installer runtime, Wayland/mixed-DPI,
macOS screen selection, cancellation/refusal and attachment import remain gated.

## Verified cross-platform draft assets

Release run `33957014450` passed all four jobs at tag `v0.1.1`, exact source
`3313dcd1d1960da4c013a878ba3d7d36bec93997`. Runtime source remains identical to
signed `31080b5`. Subsequent receipt changes do not rebuild the release.
The draft remains unpublished; the installed application was not replaced.

Packaging notarization run `33957685514` used workflow
`913464f507e6296eeed860bda321d4072dc36a53`. Apple accepted Intel app submission
`eb32e870-72c6-44ee-a132-4fd63066aa26`, Intel DMG
`1c23bd85-f421-4bf9-9650-aaf84b10fb6c`, and Apple Silicon DMG
`bf5ecab5-605c-4a74-9565-1dc46061523a`. Stapling, strict signatures and Gatekeeper
passed on the runner and locally. Intel architecture/version were x86_64/0.1.1;
Developer ID and hardened runtime signing preceded submission. Both DMGs contain
the matching signed app and Applications link. Intel runtime was not exercised.

The Intel result tar contained AppleDouble `._` sidecars that broke strict
codesign after portable extraction. Native extraction restored metadata;
`COPYFILE_DISABLE=1 tar -czf ...` repacked without sidecars, preserving the valid
app and stapled ticket. The FINAL Intel archive received a fresh updater signature.
Apple Silicon had zero sidecars and retained its verified archive/signature.
Both workflow tar commands were fixed in build-repository commit
`5788ffd856ffbd2e9e73cee886ca386f747f4c16`; executed workflow SHAs remain as above.
No notarization was repeated for this packaging correction.

All 17 draft assets were independently downloaded. Seven macOS/feed replacements
matched local hashes. Both downloaded updater signatures passed independent
minisign verification and rejected a one-byte tamper. Both archives had zero
sidecars; Python 3.13 safe extraction, strict codesign, stapler validation,
architecture and Gatekeeper checks passed. Plain and `-app` macOS feed aliases
match their final signatures and v0.1.1 URLs; all seven non-macOS feed entries
were preserved exactly. Public-feed upgrade and manual platform acceptance remain
separate. The draft has concise release notes documenting those limits.

### Downloaded asset receipt

| Asset | Bytes | SHA-256 |
| --- | ---: | --- |
| `Axio.Capture-0.1.1-1.x86_64.rpm` | 3573589 | `18acd5e9cdc562f529b3c0e22c67c753d61d3431958b9e18a4220661de83368a` |
| `Axio.Capture-0.1.1-1.x86_64.rpm.sig` | 424 | `eb287640558a00553053f11675cffc72dcede20e28e8c8e9b51abc00117fadbe` |
| `Axio.Capture_0.1.1_aarch64.dmg` | 3426488 | `2e5fc07d4e774c9963bcd32878d62aa83b02a5463c2b78b9a7e1eb82175ce815` |
| `Axio.Capture_0.1.1_amd64.AppImage` | 78776824 | `7ad9d0c4e7aff6ecaccbee82ea319856f2746672ecef41e5d237c292d21e3a78` |
| `Axio.Capture_0.1.1_amd64.AppImage.sig` | 424 | `ee4a0f2c4e0da28214ccf6f2758a0e76fa46ce7eae06cb3cbe96e60d78b94ee9` |
| `Axio.Capture_0.1.1_amd64.deb` | 3572444 | `d478f9f498778f6bfe6c8d0b269958793d40ab3c07529617f7347275f91ec8b5` |
| `Axio.Capture_0.1.1_amd64.deb.sig` | 420 | `3eeaa0fd8ef4e5c6606d2e76f5f9c160e78c22a1ffaf5de0bb7982f73947962c` |
| `Axio.Capture_0.1.1_x64-setup.exe` | 2174363 | `3a7b9606084644fe00a93a0f9d09df1570ec3e0b9ff08ba208a67d49ce8fc7ca` |
| `Axio.Capture_0.1.1_x64-setup.exe.sig` | 424 | `47fcea23a8f6f92a27dc7b5ebba083a772994a20ca779361a5e5c2269cb7d320` |
| `Axio.Capture_0.1.1_x64.dmg` | 3770582 | `ff2a0573a0ca45dc5ee5cc06139be288b11c14eebe5985c77940d60ff143ee23` |
| `Axio.Capture_0.1.1_x64_en-US.msi` | 2994176 | `ca0da51497a8c272151c3c092d9dbd4fa23a4d960b0fffdd262ca632c70d5057` |
| `Axio.Capture_0.1.1_x64_en-US.msi.sig` | 424 | `2e9abcf21fdf41df46cce665b87ca1444272e170a0be805b2e7c869795b687c3` |
| `Axio.Capture_aarch64.app.tar.gz` | 2878782 | `478b3f282ee3b8ae1f97088cae1d9e1075388c4622e415f081c1cbe72658712e` |
| `Axio.Capture_aarch64.app.tar.gz.sig` | 424 | `b7ca37839d91a76b0655b9b4bf8584985e5245ce337ec55d99583c5e9ebb28ca` |
| `Axio.Capture_x64.app.tar.gz` | 3167292 | `9c3909b2e703caee40d5f9a85683259448c8f9245ec1554865b2d75e7969c610` |
| `Axio.Capture_x64.app.tar.gz.sig` | 444 | `fac88f675266de28bc25c0fb886187c9a47f81f5be20590eafdc9370a2128d7b` |
| `latest.json` | 6735 | `92a3dae8f377e4d528fe7b9764f119fae689b79a4250d433571ba71b701b8689` |
