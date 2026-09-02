//! IPC surface. Every command is thin: validate, hand off, answer.

use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::settings::{AfterCapture, Settings};
use crate::state::{AppState, Capture};
use crate::{capture, export, hotkey, naming, overlay, settings};

#[derive(Serialize)]
pub struct AppInfo {
    pub version: &'static str,
    pub shortcut: String,
    pub save_dir: String,
    pub default_save_dir: String,
    pub settings_path: String,
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    let settings = state.settings();
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        shortcut: settings.shortcut.clone(),
        save_dir: export::save_dir(&settings).display().to_string(),
        default_save_dir: export::default_dir().display().to_string(),
        settings_path: settings::path(&app)
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

/// Validate, apply (the hotkey re-registers immediately), persist.
#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings, String> {
    settings.validate()?;
    let previous = state.settings();
    if settings.shortcut != previous.shortcut {
        if let Err(error) = hotkey::register(&app, &settings.shortcut) {
            // Put the old one back so the app never ends up without a hotkey.
            let _ = hotkey::register(&app, &previous.shortcut);
            return Err(error);
        }
    }
    settings::save(&app, &settings).map_err(|e| format!("{e:#}"))?;
    if settings.launch_at_login != previous.launch_at_login
        || settings.show_in_dock != previous.show_in_dock
    {
        settings::apply_system(&app, &settings);
    }
    *state.settings.lock().map_err(|e| e.to_string())? = settings.clone();
    Ok(settings)
}

/// What a pattern would produce right now, for the settings panel.
#[tauri::command]
pub fn preview_file_name(state: State<'_, AppState>, pattern: String) -> Result<String, String> {
    naming::validate(&pattern)?;
    let settings = state.settings();
    let path = naming::resolve(
        &export::save_dir(&settings),
        &pattern,
        naming::Context {
            width: 1280,
            height: 720,
        },
    );
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn naming_tokens() -> Vec<(String, String)> {
    naming::TOKENS
        .iter()
        .map(|(token, help)| (token.to_string(), help.to_string()))
        .collect()
}

/// Native folder picker for the save location. Async: the dialog blocks.
#[tauri::command]
pub async fn pick_save_dir(app: AppHandle) -> Result<Option<String>, String> {
    let current = export::save_dir(&app.state::<AppState>().settings());
    let picked = app
        .dialog()
        .file()
        .set_directory(current)
        .blocking_pick_folder();
    match picked {
        Some(path) => Ok(Some(
            path.into_path()
                .map_err(|e| e.to_string())?
                .display()
                .to_string(),
        )),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn start_capture(app: AppHandle) {
    overlay::begin_capture(app);
}

/// The frozen screen behind one overlay, as PNG bytes.
#[tauri::command]
pub fn overlay_image(monitor: usize, state: State<'_, AppState>) -> Result<Response, String> {
    let screens = state.screens.lock().map_err(|e| e.to_string())?;
    let screen = screens
        .iter()
        .find(|s| s.index == monitor)
        .ok_or_else(|| format!("no frozen screen for monitor {monitor}"))?;
    Ok(Response::new(screen.png.clone()))
}

/// The overlay page has drawn the screen; reveal it.
#[tauri::command]
pub fn overlay_show(window: WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    overlay::log_placement(&window);
    Ok(())
}

#[tauri::command]
pub fn overlay_cancel(app: AppHandle) {
    overlay::finish(&app);
    overlay::restore_editor(&app);
}

/// A selection in the overlay's own CSS-pixel space.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn overlay_confirm(
    app: AppHandle,
    monitor: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    view_width: f64,
    view_height: f64,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cropped = {
        let screens = state.screens.lock().map_err(|e| e.to_string())?;
        let screen = screens
            .iter()
            .find(|s| s.index == monitor)
            .ok_or_else(|| format!("no frozen screen for monitor {monitor}"))?;
        let rect = capture::map_selection(
            screen.image.width(),
            screen.image.height(),
            view_width,
            view_height,
            capture::Selection {
                x,
                y,
                width,
                height,
            },
        )
        .ok_or_else(|| "selection is empty".to_string())?;
        capture::crop(&screen.image, rect)
    };

    let png = capture::encode_png(&cropped).map_err(|e| format!("{e:#}"))?;
    *state.capture.lock().map_err(|e| e.to_string())? = Some(Capture { png: png.clone() });

    overlay::finish(&app);
    state.editor_was_visible.store(false, Ordering::SeqCst);

    let settings = state.settings();
    let (copy, save) = match settings.after_capture {
        AfterCapture::Edit => {
            overlay::show_editor(&app);
            return Ok(());
        }
        AfterCapture::Copy => (true, false),
        AfterCapture::Save => (false, true),
        AfterCapture::CopySave => (true, true),
    };
    let mut summary = Vec::new();
    if copy {
        export::copy(&png).map_err(|e| format!("{e:#}"))?;
        summary.push("copied to the clipboard".to_string());
    }
    if save {
        let path = export::new_capture_path(&settings, cropped.width(), cropped.height());
        let final_png = capture::encode_png_final(&cropped).map_err(|e| format!("{e:#}"))?;
        export::save(&final_png, &path).map_err(|e| format!("{e:#}"))?;
        summary.push(format!("saved to {}", path.display()));
    }
    if settings.notify {
        use tauri_plugin_notification::NotificationExt;
        let _ = app
            .notification()
            .builder()
            .title(format!("Captured {}×{}", cropped.width(), cropped.height()))
            .body(summary.join(", "))
            .show();
    }
    Ok(())
}

/// The current capture for the editor, as PNG bytes.
#[tauri::command]
pub fn editor_image(state: State<'_, AppState>) -> Result<Response, String> {
    let capture = state.capture.lock().map_err(|e| e.to_string())?;
    let capture = capture
        .as_ref()
        .ok_or_else(|| "no capture yet".to_string())?;
    Ok(Response::new(capture.png.clone()))
}

#[derive(Serialize)]
pub struct ExportResult {
    pub action: String,
    pub path: Option<String>,
}

/// Raw PNG body from the editor's canvas; `x-axio-action` selects `copy`,
/// `save` (default folder, timestamped name) or `save-as` (native dialog).
/// Async so the dialog and the disk never block the main thread.
#[tauri::command]
pub async fn export_png(
    app: AppHandle,
    window: WebviewWindow,
    request: Request<'_>,
) -> Result<ExportResult, String> {
    let settings = app.state::<AppState>().settings();
    let action = request
        .headers()
        .get("x-axio-action")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("copy")
        .to_string();
    let png: Vec<u8> = match request.body() {
        InvokeBody::Raw(bytes) => bytes.clone(),
        InvokeBody::Json(_) => return Err("expected a raw PNG body".into()),
    };
    if png.is_empty() {
        return Err("empty image".into());
    }

    match action.as_str() {
        "copy" => {
            export::copy(&png).map_err(|e| format!("{e:#}"))?;
            if settings.close_on_copy {
                let _ = window.hide();
            }
            Ok(ExportResult { action, path: None })
        }
        "save" => {
            let (width, height) = export::png_dimensions(&png);
            let path = export::new_capture_path(&settings, width, height);
            export::save(&png, &path).map_err(|e| format!("{e:#}"))?;
            if settings.close_on_save {
                let _ = window.hide();
            }
            Ok(ExportResult {
                action,
                path: Some(path.display().to_string()),
            })
        }
        "save-as" => {
            let (width, height) = export::png_dimensions(&png);
            let suggested = export::new_capture_path(&settings, width, height);
            let directory = suggested
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| export::save_dir(&settings));
            let file_name = suggested
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "capture.png".into());
            let _ = std::fs::create_dir_all(&directory);
            let chosen = app
                .dialog()
                .file()
                .add_filter("PNG image", &["png"])
                .set_directory(directory)
                .set_file_name(file_name)
                .blocking_save_file();
            let Some(chosen) = chosen else {
                return Ok(ExportResult {
                    action: "cancelled".into(),
                    path: None,
                });
            };
            let path = chosen.into_path().map_err(|e| e.to_string())?;
            export::save(&png, &path).map_err(|e| format!("{e:#}"))?;
            if settings.close_on_save {
                let _ = window.hide();
            }
            Ok(ExportResult {
                action,
                path: Some(path.display().to_string()),
            })
        }
        "reveal" => {
            let dir = export::save_dir(&settings);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            export::reveal(&dir).map_err(|e| format!("{e:#}"))?;
            Ok(ExportResult {
                action,
                path: Some(dir.display().to_string()),
            })
        }
        other => Err(format!("unknown export action {other}")),
    }
}
