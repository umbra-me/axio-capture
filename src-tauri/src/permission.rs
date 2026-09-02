//! macOS Screen Recording permission. Without it `CGWindowListCreateImage`
//! does not fail: it returns the wallpaper with no windows, so the overlay
//! would show an empty desktop. Check first and drive the fix instead.
//!
//! macOS never lets an app grant itself the permission, but everything
//! around that one click is automated here: a stale entry (left by a build
//! with a different signature) is reset, the system prompt is triggered, the
//! grant is watched for, and the app relaunches itself because macOS applies
//! a new grant only to a fresh process.

#[cfg(target_os = "macos")]
mod mac {
    use std::time::{Duration, Instant};

    use tauri::AppHandle;
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }

    const WATCH_FOR: Duration = Duration::from_secs(5 * 60);

    pub fn granted() -> bool {
        // SAFETY: plain CoreGraphics calls with no arguments.
        unsafe { CGPreflightScreenCaptureAccess() }
    }

    /// Registers the app in the Screen Recording list and shows the system
    /// prompt (once per entry). Returns the state after the request.
    fn request() -> bool {
        // SAFETY: as above.
        unsafe { CGRequestScreenCaptureAccess() }
    }

    /// Drop this app's own entry so the system prompt appears again and a
    /// grant recorded against an older signature stops shadowing the real
    /// state. `tccutil` only touches the named bundle and needs no root for
    /// the current user.
    fn reset_own_entry(app: &AppHandle) {
        let identifier = app.config().identifier.clone();
        match std::process::Command::new("tccutil")
            .args(["reset", "ScreenCapture", &identifier])
            .output()
        {
            Ok(output) if output.status.success() => {}
            Ok(output) => eprintln!(
                "axio-capture: tccutil: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
            Err(error) => eprintln!("axio-capture: tccutil: {error}"),
        }
    }

    pub fn ensure(app: &AppHandle) -> Result<(), ()> {
        if granted() {
            return Ok(());
        }
        reset_own_entry(app);
        if request() {
            return Ok(());
        }

        let open = app
            .dialog()
            .message(
                "Axio Capture needs Screen Recording permission to see the screen; without it macOS \
                 only hands over the wallpaper.\n\n\
                 Turn on Axio Capture in the Screen Recording list. The app will notice and \
                 relaunch itself to apply the change.",
            )
            .title("Screen Recording permission needed")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Open System Settings".into(),
                "Not now".into(),
            ))
            .blocking_show();
        if !open {
            return Err(());
        }
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn();
        watch_and_relaunch(app.clone());
        Err(())
    }

    /// Poll until the toggle is on, then relaunch. A relaunch is the only way
    /// a running process picks the grant up.
    fn watch_and_relaunch(app: AppHandle) {
        std::thread::spawn(move || {
            let started = Instant::now();
            while started.elapsed() < WATCH_FOR {
                std::thread::sleep(Duration::from_secs(1));
                if granted() {
                    app.dialog()
                        .message(
                            "Screen Recording is on. Axio Capture will relaunch now to apply it; \
                             press the capture shortcut again afterwards.",
                        )
                        .title("Permission granted")
                        .blocking_show();
                    app.restart();
                }
            }
        });
    }
}

/// `Ok(())` when capture may proceed. On refusal the user has already seen
/// the explanation; the caller only has to abort quietly.
pub fn ensure_screen_capture(app: &tauri::AppHandle) -> Result<(), ()> {
    #[cfg(target_os = "macos")]
    {
        mac::ensure(app)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(())
    }
}
