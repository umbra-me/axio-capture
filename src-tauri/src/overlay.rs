//! The capture flow: freeze every monitor, put a borderless overlay window on
//! each, and turn the confirmed selection into the editor's image.

use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::capture;
use crate::state::{AppState, Capture, ScreenGeometry};

pub const EDITOR_LABEL: &str = "editor";
const OVERLAY_PREFIX: &str = "overlay-";

/// Hotkey, tray and second-instance entry point. Never blocks the caller.
pub fn begin_capture(app: AppHandle) {
    let state = app.state::<AppState>();
    if state.capturing.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        let state = app.state::<AppState>();

        if crate::permission::ensure_screen_capture(&app).is_err() {
            state.capturing.store(false, Ordering::SeqCst);
            return;
        }

        // The editor must not appear in its own screenshot.
        let editor_visible = app
            .get_webview_window(EDITOR_LABEL)
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        state
            .editor_was_visible
            .store(editor_visible, Ordering::SeqCst);
        if editor_visible {
            if let Some(editor) = app.get_webview_window(EDITOR_LABEL) {
                let _ = editor.hide();
            }
            std::thread::sleep(Duration::from_millis(250));
        }

        let screens = match capture::grab_screens() {
            Ok(screens) => screens,
            Err(error) => {
                state.capturing.store(false, Ordering::SeqCst);
                restore_editor(&app);
                app.dialog()
                    .message(format!(
                        "{error:#}\n\nOn macOS, allow Axio Capture under System Settings → Privacy & Security → Screen Recording."
                    ))
                    .title("Capture failed")
                    .kind(MessageDialogKind::Error)
                    .blocking_show();
                return;
            }
        };

        let geometries: Vec<ScreenGeometry> = screens.iter().map(|s| s.geometry).collect();
        *state.screens.lock().expect("screens lock") = screens;

        let handle = app.clone();
        let result = app.run_on_main_thread(move || {
            for geometry in geometries {
                if let Err(error) = open_overlay(&handle, geometry) {
                    eprintln!("axio-capture: overlay window {}: {error}", geometry.index);
                }
            }
        });
        if result.is_err() {
            finish(&app);
        }
    });
}

fn open_overlay(app: &AppHandle, geometry: ScreenGeometry) -> tauri::Result<()> {
    let label = format!("{OVERLAY_PREFIX}{}", geometry.index);
    let url = WebviewUrl::App(format!("overlay.html?monitor={}", geometry.index).into());

    // Size and position go through the builder, in logical units, so the
    // window is created at its final frame. Setting them afterwards is
    // asynchronous on macOS and `setContentSize` keeps the bottom-left corner
    // fixed, so a position-then-size sequence pushes the top edge off-screen.
    let scale = f64::from(geometry.scale_factor.max(0.01));
    #[cfg(target_os = "macos")]
    let (x, y, width, height) = (
        f64::from(geometry.x),
        f64::from(geometry.y),
        f64::from(geometry.width),
        f64::from(geometry.height),
    );
    #[cfg(not(target_os = "macos"))]
    let (x, y, width, height) = (
        f64::from(geometry.x) / scale,
        f64::from(geometry.y) / scale,
        f64::from(geometry.width) / scale,
        f64::from(geometry.height) / scale,
    );
    #[cfg(target_os = "macos")]
    let _ = scale;

    let window = WebviewWindowBuilder::new(app, &label, url)
        .title("Axio Capture")
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .accept_first_mouse(true)
        .focused(true)
        .position(x, y)
        .inner_size(width, height)
        // Shown by `overlay_show` once the page has drawn the frozen screen,
        // so the user never sees a blank window flash.
        .visible(false)
        .build()?;

    #[cfg(not(target_os = "macos"))]
    {
        // Physical units are exact on Windows and X11; size before position.
        window.set_size(tauri::PhysicalSize::new(geometry.width, geometry.height))?;
        window.set_position(tauri::PhysicalPosition::new(geometry.x, geometry.y))?;
    }
    #[cfg(target_os = "macos")]
    raise_above_menu_bar(&window);
    Ok(())
}

/// Log where an overlay actually ended up, for placement bugs.
pub fn log_placement(window: &WebviewWindow) {
    let position = window
        .outer_position()
        .map(|p| (p.x, p.y))
        .unwrap_or((0, 0));
    let size = window
        .outer_size()
        .map(|s| (s.width, s.height))
        .unwrap_or((0, 0));
    let scale = window.scale_factor().unwrap_or(0.0);
    eprintln!(
        "axio-capture: {} placed at physical ({}, {}) {}x{} scale {scale}",
        window.label(),
        position.0,
        position.1,
        size.0,
        size.1
    );
}

/// A borderless always-on-top window still sits below the menu bar and the
/// Dock. Screen-saver level covers both; the collection behaviour lets the
/// overlay appear over a full-screen app's Space instead of switching away.
#[cfg(target_os = "macos")]
fn raise_above_menu_bar(window: &WebviewWindow) {
    use objc2_app_kit::{NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior};

    let window = window.clone();
    let _ = window.clone().run_on_main_thread(move || {
        if let Ok(ptr) = window.ns_window() {
            // SAFETY: `ns_window` returns the live NSWindow backing this
            // tauri window, and we are on the main thread.
            let ns_window: &NSWindow = unsafe { &*ptr.cast::<NSWindow>() };
            ns_window.setLevel(NSScreenSaverWindowLevel);
            ns_window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
        }
    });
}

pub fn overlay_windows(app: &AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(OVERLAY_PREFIX))
        .map(|(_, window)| window)
        .collect()
}

/// Tear the overlays down and release the frozen screens.
pub fn finish(app: &AppHandle) {
    for window in overlay_windows(app) {
        let _ = window.destroy();
    }
    let state = app.state::<AppState>();
    state.screens.lock().expect("screens lock").clear();
    state.capturing.store(false, Ordering::SeqCst);
}

pub fn restore_editor(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.editor_was_visible.swap(false, Ordering::SeqCst) {
        if let Some(editor) = app.get_webview_window(EDITOR_LABEL) {
            let _ = editor.show();
            let _ = editor.set_focus();
        }
    }
}

/// Open the editor on the current capture, or tell an existing one to reload.
pub fn show_editor(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window(EDITOR_LABEL) {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
            let _ = window.emit("capture-new", ());
            return;
        }
        let built =
            WebviewWindowBuilder::new(&handle, EDITOR_LABEL, WebviewUrl::App("index.html".into()))
                .title("Axio Capture")
                .inner_size(1100.0, 760.0)
                .min_inner_size(640.0, 420.0)
                .center()
                .build();
        match built {
            Ok(window) => {
                let this = window.clone();
                window.on_window_event(move |event| match event {
                    // Closing hides: the annotations survive and reopening is instant.
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = this.hide();
                    }
                    // Dropping an image file onto the editor opens it.
                    tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                        if let Some(path) = paths.first() {
                            open_file(this.app_handle(), path);
                        }
                    }
                    _ => {}
                });
                let _ = window.set_focus();
            }
            Err(error) => eprintln!("axio-capture: editor window: {error}"),
        }
    });
}

/// Bring the editor up (with or without a capture) and open its settings panel.
pub fn show_settings(app: &AppHandle) {
    show_editor(app);
    let handle = app.clone();
    // The window may have just been created; give its page a moment to attach
    // the listener. Events to a window that has not loaded are dropped.
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        if let Some(window) = handle.get_webview_window(EDITOR_LABEL) {
            let _ = window.emit("open-settings", ());
        }
    });
}

/// Open an image file as the current capture and show the editor.
pub fn open_file(app: &AppHandle, path: &std::path::Path) {
    match capture::load_file(path) {
        Ok(png) => {
            let state = app.state::<AppState>();
            *state.capture.lock().expect("capture lock") = Some(Capture { png });
            show_editor(app);
        }
        Err(error) => {
            let message = format!("{error:#}");
            let app = app.clone();
            std::thread::spawn(move || {
                app.dialog()
                    .message(message)
                    .title("Could not open image")
                    .kind(MessageDialogKind::Error)
                    .blocking_show();
            });
        }
    }
}
