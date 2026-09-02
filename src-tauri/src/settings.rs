//! User settings, one JSON file in the platform config directory.
//! Every field has a default so an older file keeps loading.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AfterCapture {
    /// Open the editor (the default).
    #[default]
    Edit,
    /// Copy the region to the clipboard and stop.
    Copy,
    /// Save the region to the captures folder and stop.
    Save,
    /// Both.
    CopySave,
}

/// macOS only: when the Dock shows the app.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DockIcon {
    Always,
    /// Only while the editor window exists. Every switch to a regular app
    /// also lands the app in the Dock's recent-applications area, which is
    /// why this is not the default.
    WhileEditorOpen,
    /// A pure menu-bar app; the editor still opens and takes focus.
    #[default]
    Never,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct EditorPrefs {
    pub tool: String,
    pub color: String,
    pub width: u8,
}

impl Default for EditorPrefs {
    fn default() -> Self {
        Self {
            tool: "arrow".into(),
            color: "#ff3b30".into(),
            width: 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct Settings {
    pub after_capture: AfterCapture,
    pub close_on_copy: bool,
    pub close_on_save: bool,
    /// `None` means `~/Pictures/Axio Capture`.
    pub save_dir: Option<String>,
    /// Relative to `save_dir`; see `naming::TOKENS`.
    pub file_pattern: String,
    pub shortcut: String,
    pub launch_at_login: bool,
    /// Desktop notification when a capture is copied or saved without the editor.
    pub notify: bool,
    /// macOS only: when the Dock icon is shown.
    pub dock_icon: DockIcon,
    /// Look for a new version after launch and every few hours.
    pub check_updates: bool,
    /// Set when the user declined the macOS "move to Applications" offer.
    pub skip_move_prompt: bool,
    pub editor: EditorPrefs,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            after_capture: AfterCapture::Edit,
            close_on_copy: true,
            close_on_save: false,
            save_dir: None,
            file_pattern: crate::naming::DEFAULT_PATTERN.into(),
            shortcut: crate::CAPTURE_SHORTCUT.into(),
            launch_at_login: false,
            notify: true,
            dock_icon: DockIcon::default(),
            check_updates: true,
            skip_move_prompt: false,
            editor: EditorPrefs::default(),
        }
    }
}

impl Settings {
    /// Reject values the rest of the app cannot act on.
    pub fn validate(&self) -> Result<(), String> {
        crate::hotkey::parse(&self.shortcut)?;
        if let Some(dir) = &self.save_dir {
            if dir.trim().is_empty() {
                return Err("save folder is empty; leave it unset for the default".into());
            }
        }
        crate::naming::validate(&self.file_pattern)?;
        if !(1..=12).contains(&self.editor.width) {
            return Err("stroke width must be between 1 and 12".into());
        }
        Ok(())
    }
}

/// Push the settings the operating system holds a copy of: the login item
/// and, on macOS, the Dock icon. Failures are logged, never fatal.
pub fn apply_system(app: &AppHandle, settings: &Settings) {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    let result = match (settings.launch_at_login, autolaunch.is_enabled()) {
        (true, Ok(false)) => autolaunch.enable(),
        (false, Ok(true)) => autolaunch.disable(),
        (_, Err(error)) => Err(error),
        _ => Ok(()),
    };
    if let Err(error) = result {
        eprintln!("axio-capture: launch at login: {error}");
    }

    #[cfg(target_os = "macos")]
    apply_dock_icon(app, settings.dock_icon);
}

/// The policy the app must start with. tao applies this in
/// `applicationDidFinishLaunching`, after `setup` has run, so a runtime call
/// from `setup` is overwritten; the initial value has to go through `App`.
#[cfg(target_os = "macos")]
pub fn initial_activation_policy(settings: &Settings) -> tauri::ActivationPolicy {
    match settings.dock_icon {
        DockIcon::Always => tauri::ActivationPolicy::Regular,
        DockIcon::WhileEditorOpen | DockIcon::Never => tauri::ActivationPolicy::Accessory,
    }
}

/// Show or hide the Dock icon per the setting and whether the editor exists.
/// Runs on the main thread; safe to call from anywhere.
#[cfg(target_os = "macos")]
pub fn apply_dock_icon(app: &AppHandle, mode: DockIcon) {
    let editor_open = app
        .get_webview_window(crate::overlay::EDITOR_LABEL)
        .is_some();
    let show = match mode {
        DockIcon::Always => true,
        DockIcon::WhileEditorOpen => editor_open,
        DockIcon::Never => false,
    };
    let policy = if show {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    };
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Err(error) = handle.set_activation_policy(policy) {
            eprintln!("axio-capture: activation policy: {error}");
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn apply_dock_icon(_app: &AppHandle, _mode: DockIcon) {}

pub fn path(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("no config directory on this platform")?;
    Ok(dir.join("settings.json"))
}

/// Load the file, or defaults when it is missing. A corrupt file is reported
/// and replaced by defaults rather than blocking startup.
pub fn load(app: &AppHandle) -> Settings {
    let Ok(path) = path(app) else {
        return Settings::default();
    };
    match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
            Ok(settings) => settings,
            Err(error) => {
                eprintln!("axio-capture: {}: {error}; using defaults", path.display());
                Settings::default()
            }
        },
        Err(_) => Settings::default(),
    }
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<()> {
    let path = path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(settings)?;
    std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_file_is_all_defaults() {
        let parsed: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed, Settings::default());
    }

    #[test]
    fn partial_files_keep_their_values() {
        let parsed: Settings =
            serde_json::from_str(r#"{"close_on_copy": false, "after_capture": "copy-save"}"#)
                .unwrap();
        assert!(!parsed.close_on_copy);
        assert_eq!(parsed.after_capture, AfterCapture::CopySave);
        assert_eq!(parsed.shortcut, crate::CAPTURE_SHORTCUT);
    }

    #[test]
    fn validation_rejects_bad_shortcuts_and_widths() {
        let mut settings = Settings {
            shortcut: "NotAKey".into(),
            ..Default::default()
        };
        assert!(settings.validate().is_err());
        settings.shortcut = "Alt+F9".into();
        settings.editor.width = 0;
        assert!(settings.validate().is_err());
        settings.editor.width = 6;
        assert!(settings.validate().is_ok());
    }
}
