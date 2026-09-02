//! The one global shortcut, registered from settings and re-registered when
//! the user changes it.

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::overlay;

pub fn parse(text: &str) -> Result<Shortcut, String> {
    text.trim()
        .parse::<Shortcut>()
        .map_err(|error| format!("\"{text}\" is not a valid shortcut: {error}"))
}

/// Replace whatever is registered with `text`. On failure the previous
/// registration is already gone, so the caller should fall back to a known
/// good value.
pub fn register(app: &AppHandle, text: &str) -> Result<(), String> {
    let shortcut = parse(text)?;
    let manager = app.global_shortcut();
    manager.unregister_all().map_err(|e| e.to_string())?;
    manager
        .on_shortcut(shortcut, |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                overlay::begin_capture(app.clone());
            }
        })
        .map_err(|error| format!("could not register \"{text}\": {error}"))
}
