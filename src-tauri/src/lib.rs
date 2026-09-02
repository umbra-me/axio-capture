//! Axio Capture: region screenshots with an annotation editor.
//!
//! Rust owns every screen pixel and every side effect (capture, windows,
//! clipboard, files, hotkeys). The two web pages only draw: the overlay draws
//! the frozen screen and a selection rectangle, the editor draws the crop and
//! the annotations on top of it.

mod capture;
mod cli;
mod commands;
mod export;
mod hotkey;
mod install;
mod naming;
mod overlay;
mod permission;
mod settings;
mod state;
mod tray;
mod updater;

use tauri::Manager;

/// The default hotkey. `Cmd+Shift+3/4/5` belong to macOS and `PrintScreen`
/// does not exist on Mac keyboards; `2` sits in that row, is unassigned by the
/// system and by common apps, and crosses to Windows and Linux as
/// `Ctrl+Shift+2` with the same string.
pub const CAPTURE_SHORTCUT: &str = "CmdOrCtrl+Shift+2";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            // A second launch forwards its arguments here; with none it is a
            // capture request.
            cli::handle(app, args.into_iter().skip(1), Some(cwd.as_ref()), true);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state::AppState::default())
        .setup(|app| {
            let handle = app.handle();
            let loaded = settings::load(handle);
            if let Err(error) = hotkey::register(handle, &loaded.shortcut) {
                eprintln!("axio-capture: {error}; falling back to {CAPTURE_SHORTCUT}");
                hotkey::register(handle, CAPTURE_SHORTCUT)?;
            }
            *app.state::<state::AppState>()
                .settings
                .lock()
                .expect("settings lock") = loaded;
            tray::install(handle)?;
            cli::handle(handle, std::env::args().skip(1), None, false);
            let startup = handle.clone();
            std::thread::spawn(move || {
                install::offer_move_to_applications(&startup);
                updater::schedule(startup);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_capture,
            commands::overlay_image,
            commands::overlay_show,
            commands::overlay_cancel,
            commands::overlay_confirm,
            commands::editor_image,
            commands::export_png,
            commands::app_info,
            commands::get_settings,
            commands::set_settings,
            commands::pick_save_dir,
            commands::preview_file_name,
            commands::naming_tokens,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Axio Capture")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
                // Closing the editor must not quit a tray application.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = &event {
                // Dock icon click: reopen the editor if a capture exists.
                if let Some(window) = app.get_webview_window(overlay::EDITOR_LABEL) {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            #[cfg(not(target_os = "macos"))]
            let _ = app;
        });
}
