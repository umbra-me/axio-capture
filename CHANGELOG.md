# Changelog

## Unreleased

- Saving never replaces an earlier capture. Save and the save after-capture
  action create each file exclusively and move on to `-2`, `-3`, ... (or the
  next `%n%`) when the name is taken, so two captures in one second or a
  fixed pattern keep every file. Save As still replaces the file you pick.
- A file-name pattern can no longer reach outside the save folder through a
  backslash (`..\outside`) or a token's value; the name is checked after
  `\` becomes `/`, and again once expanded.
- Changing the shortcut no longer leaves the new one registered when the
  settings file cannot be written; the previous shortcut is restored.

- September 5: Prepare 0.1.1 with annotated PNG/JSON attachment export, release preflight checks and resumable scheduled update polling. No tag published. See [change set](docs/change-set-2026-09-05.md).

- New app icon: the Axio family mark, the slate tile with the selection
  corners and a mint point, replacing the off-palette violet. Source of truth
  is `tools/brand/marks/capture.svg` in the Axio workspace; `icons/source.svg`
  is its copy. The menu-bar template is unchanged.

## 0.1.0 (2026-09-02)

First release.

- Region capture from a hotkey (`Ctrl+Shift+2`, `⌘⇧2`), the tray, or a second
  launch; one overlay per monitor.
- Editor: arrow, line, rectangle, ellipse, pen, highlighter, text, numbered
  steps, blur; undo and redo; copy, save, save as.
- Settings: after-capture action, close on copy or save, save folder,
  file-name pattern with date, size, random and counter tokens, shortcut,
  launch at login, notifications, Dock icon mode, automatic update checks.
- Self-updating from signed GitHub Releases; a tagged release builds macOS
  (Apple Silicon and Intel), Windows and Linux.
- macOS: recovers a missing or stale Screen Recording grant around the one
  toggle the system reserves for the user; offers to move itself into
  `/Applications`; runs as a menu-bar app.
