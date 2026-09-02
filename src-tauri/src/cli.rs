//! Command-line arguments, for the first launch and for every launch after it
//! (the single-instance plugin forwards those to the running process).
//!
//! `--capture` starts a region capture, `--open <image>` opens a file in the
//! editor. Nothing else is recognised.

use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::overlay;

pub fn handle(
    app: &AppHandle,
    args: impl Iterator<Item = String>,
    cwd: Option<&Path>,
    second_launch: bool,
) {
    let args: Vec<String> = args.collect();
    let mut iter = args.iter();
    let mut acted = false;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--capture" => {
                overlay::begin_capture(app.clone());
                acted = true;
            }
            "--open" => {
                if let Some(path) = iter.next() {
                    let path = PathBuf::from(path);
                    let path = match (path.is_absolute(), cwd) {
                        (false, Some(cwd)) => cwd.join(path),
                        _ => path,
                    };
                    overlay::open_file(app, &path);
                    acted = true;
                }
            }
            _ => {}
        }
    }
    if second_launch && !acted {
        overlay::begin_capture(app.clone());
    }
}
