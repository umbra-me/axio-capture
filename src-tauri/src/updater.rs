//! In-app updates from the GitHub release feed declared in `tauri.conf.json`.
//! Every artifact is verified against the public key embedded there before it
//! is installed; the private key never leaves the release workflow.

use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(20);
const RECHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// Background checks: a first one shortly after launch, then every six hours.
/// Quiet unless an update exists. Debug builds never check on their own.
pub fn schedule(app: AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK_DELAY);
        loop {
            if !app
                .state::<crate::state::AppState>()
                .settings()
                .check_updates
            {
                return;
            }
            run(&app, false);
            std::thread::sleep(RECHECK_EVERY);
        }
    });
}

/// The tray's "Check for updates…": always reports an outcome.
pub fn check_now(app: AppHandle) {
    std::thread::spawn(move || run(&app, true));
}

fn run(app: &AppHandle, interactive: bool) {
    match tauri::async_runtime::block_on(find(app)) {
        Ok(Some(update)) => offer(app, update),
        Ok(None) => {
            if interactive {
                app.dialog()
                    .message(format!(
                        "Axio Capture {} is the latest version.",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .title("Up to date")
                    .blocking_show();
            }
        }
        Err(error) => {
            eprintln!("axio-capture: update check: {error}");
            if interactive {
                app.dialog()
                    .message(format!("Could not check for updates.\n\n{error}"))
                    .title("Update check failed")
                    .kind(MessageDialogKind::Error)
                    .blocking_show();
            }
        }
    }
}

async fn find(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    let updater = app
        .updater_builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    updater.check().await.map_err(|e| e.to_string())
}

fn offer(app: &AppHandle, update: tauri_plugin_updater::Update) {
    let notes = update
        .body
        .as_deref()
        .map(str::trim)
        .filter(|body| !body.is_empty())
        .map(|body| format!("\n\n{body}"))
        .unwrap_or_default();
    let install = app
        .dialog()
        .message(format!(
            "Version {} is available; you have {}.{notes}",
            update.version, update.current_version
        ))
        .title("Update available")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Update and restart".into(),
            "Later".into(),
        ))
        .blocking_show();
    if !install {
        return;
    }

    let result = tauri::async_runtime::block_on(update.download_and_install(|_, _| {}, || {}));
    match result {
        Ok(()) => app.restart(),
        Err(error) => {
            eprintln!("axio-capture: update install: {error}");
            app.dialog()
                .message(format!("The update could not be installed.\n\n{error}"))
                .title("Update failed")
                .kind(MessageDialogKind::Error)
                .blocking_show();
        }
    }
}

use tauri::Manager as _;
