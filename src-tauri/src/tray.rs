use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::{export, overlay};

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let shortcut = app.state::<crate::state::AppState>().settings().shortcut;
    let capture = MenuItem::with_id(
        app,
        "capture",
        "Capture region",
        true,
        Some(shortcut.as_str()),
    )?;
    let editor = MenuItem::with_id(app, "editor", "Open editor", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", "Check for updates…", true, None::<&str>)?;
    let folder = MenuItem::with_id(app, "folder", "Open captures folder", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Axio Capture", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &capture,
            &editor,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &folder,
            &updates,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Axio Capture")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture" => overlay::begin_capture(app.clone()),
            "editor" => {
                if app
                    .state::<crate::state::AppState>()
                    .capture
                    .lock()
                    .map(|c| c.is_some())
                    .unwrap_or(false)
                {
                    overlay::show_editor(app);
                } else {
                    overlay::begin_capture(app.clone());
                }
            }
            "settings" => overlay::show_settings(app),
            "updates" => crate::updater::check_now(app.clone()),
            "folder" => {
                let dir = export::save_dir(&app.state::<crate::state::AppState>().settings());
                let _ = std::fs::create_dir_all(&dir);
                if let Err(error) = export::reveal(&dir) {
                    eprintln!("axio-capture: open folder: {error:#}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });

    #[cfg(target_os = "macos")]
    {
        // Menu-bar icons are monochrome templates that macOS tints itself.
        let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray/128x128.png"))?;
        builder = builder.icon(icon).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(icon) = app.default_window_icon().cloned() {
            builder = builder.icon(icon);
        }
    }

    builder.build(app)?;
    Ok(())
}
