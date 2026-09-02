//! First-launch hygiene on macOS: an app run from a disk image, the Downloads
//! folder or a build directory is a second copy waiting to happen, and the
//! updater replaces whichever copy is running. Offer to move it into
//! `/Applications` once, replacing what is there.

#[cfg(target_os = "macos")]
pub fn offer_move_to_applications(app: &tauri::AppHandle) {
    use std::path::{Path, PathBuf};
    use tauri::Manager;
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

    if cfg!(debug_assertions) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    // <bundle>.app/Contents/MacOS/<exe>
    let Some(bundle) = exe.ancestors().nth(3).map(Path::to_path_buf) else {
        return;
    };
    if bundle.extension().and_then(|e| e.to_str()) != Some("app") {
        return;
    }
    let applications = PathBuf::from("/Applications");
    let user_applications = dirs::home_dir().map(|h| h.join("Applications"));
    if bundle.starts_with(&applications)
        || user_applications
            .as_ref()
            .is_some_and(|dir| bundle.starts_with(dir))
    {
        return;
    }
    let state = app.state::<crate::state::AppState>();
    if state.settings().skip_move_prompt {
        return;
    }

    let name = bundle
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Axio Capture.app".into());
    let target = applications.join(&name);
    let from = bundle
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let existing = if target.exists() {
        " The copy already there will be replaced."
    } else {
        ""
    };
    let ok = app
        .dialog()
        .message(format!(
            "Axio Capture is running from {from}.\n\nMove it to the Applications folder so there is one copy to update?{existing}"
        ))
        .title("Move to Applications?")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Move".into(),
            "Keep here".into(),
        ))
        .blocking_show();
    if !ok {
        let mut settings = state.settings();
        settings.skip_move_prompt = true;
        if let Err(error) = crate::settings::save(app, &settings) {
            eprintln!("axio-capture: settings: {error:#}");
        }
        *state.settings.lock().expect("settings lock") = settings;
        return;
    }

    if target.exists() {
        if let Err(error) = std::fs::remove_dir_all(&target) {
            fail(
                app,
                &format!("could not replace {}: {error}", target.display()),
            );
            return;
        }
    }
    // `ditto` copies a bundle faithfully (resource forks, symlinks, permissions).
    let copied = std::process::Command::new("ditto")
        .arg(&bundle)
        .arg(&target)
        .status();
    match copied {
        Ok(status) if status.success() => {}
        Ok(status) => {
            fail(app, &format!("ditto exited with {status}"));
            return;
        }
        Err(error) => {
            fail(app, &format!("could not run ditto: {error}"));
            return;
        }
    }
    // Launch the moved copy after this one has exited, or the single-instance
    // guard would hand the launch straight back to us.
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("sleep 1; open \"{}\"", target.display()))
        .spawn();
    app.exit(0);
}

#[cfg(target_os = "macos")]
fn fail(app: &tauri::AppHandle, message: &str) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
    eprintln!("axio-capture: move to Applications: {message}");
    app.dialog()
        .message(format!("The app could not be moved.\n\n{message}"))
        .title("Move failed")
        .kind(MessageDialogKind::Error)
        .blocking_show();
}

#[cfg(not(target_os = "macos"))]
pub fn offer_move_to_applications(_app: &tauri::AppHandle) {}
