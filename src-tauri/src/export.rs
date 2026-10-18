use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

/// The configured folder, or the default.
pub fn save_dir(settings: &crate::settings::Settings) -> PathBuf {
    match &settings.save_dir {
        Some(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => default_dir(),
    }
}

/// `~/Pictures/Axio Capture`, falling back to the home directory and then the
/// working directory when the platform reports neither.
pub fn default_dir() -> PathBuf {
    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Axio Capture")
}

/// The name a new capture of `width` x `height` would get, per the settings.
/// Only a suggestion (the Save As dialog starts from it); `save_new` is what
/// claims a name.
pub fn new_capture_path(
    settings: &crate::settings::Settings,
    width: u32,
    height: u32,
) -> Result<PathBuf> {
    crate::naming::resolve(
        &save_dir(settings),
        &settings.file_pattern,
        crate::naming::Context { width, height },
    )
    .map_err(|error| anyhow!(error))
}

/// Width and height from a PNG's header, without decoding the pixels.
pub fn png_dimensions(png: &[u8]) -> (u32, u32) {
    image::ImageReader::new(std::io::Cursor::new(png))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .unwrap_or((0, 0))
}

/// Save a new capture under the first name from the pattern that nothing
/// occupies. Each name is created exclusively, so an existing file is never
/// truncated, a symbolic link is never followed, and two saves racing for
/// one name both succeed under different names.
pub fn save_new(
    png: &[u8],
    settings: &crate::settings::Settings,
    width: u32,
    height: u32,
) -> Result<PathBuf> {
    let candidates = crate::naming::candidates(
        &save_dir(settings),
        &settings.file_pattern,
        crate::naming::Context { width, height },
    )
    .map_err(|error| anyhow!(error))?;
    for path in candidates {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("creating {}", path.display()));
            }
        };
        if let Err(error) = file.write_all(png).and_then(|()| file.sync_all()) {
            drop(file);
            // The name is ours; leave no half-written capture behind.
            let _ = std::fs::remove_file(&path);
            return Err(error).with_context(|| format!("writing {}", path.display()));
        }
        return Ok(path);
    }
    bail!("every numbered name for this pattern is taken")
}

/// Write to a path the user picked in the Save As dialog, replacing what is
/// there: the dialog has already asked whether to overwrite.
pub fn save(png: &[u8], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, png).with_context(|| format!("writing {}", path.display()))
}

/// Put the PNG on the clipboard as an image.
pub fn copy(png: &[u8]) -> Result<()> {
    let decoded = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .context("decoding png for the clipboard")?
        .into_rgba8();
    let (width, height) = decoded.dimensions();
    let data = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: std::borrow::Cow::Owned(decoded.into_raw()),
    };

    #[cfg(target_os = "linux")]
    {
        // X11 and Wayland clipboards are served by the owning process; this
        // thread keeps the offer alive until another program takes ownership.
        std::thread::spawn(move || {
            use arboard::SetExtLinux;
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set().wait().image(data);
            }
        });
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut clipboard = arboard::Clipboard::new().context("opening the clipboard")?;
        clipboard.set_image(data).context("writing the clipboard")?;
        Ok(())
    }
}

pub fn reveal(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("explorer");
        c.arg(path);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };
    command
        .spawn()
        .with_context(|| format!("opening {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn settings_in(dir: &Path, pattern: &str) -> Settings {
        Settings {
            save_dir: Some(dir.display().to_string()),
            file_pattern: pattern.into(),
            ..Settings::default()
        }
    }

    fn scratch() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("axio-export-{}", crate::naming::random_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn auto_save_never_overwrites_an_existing_file() {
        let dir = scratch();
        std::fs::write(dir.join("shot.png"), b"old").unwrap();
        let path = save_new(b"new", &settings_in(&dir, "shot"), 1, 1).unwrap();
        assert_eq!(path, dir.join("shot-2.png"));
        assert_eq!(std::fs::read(dir.join("shot.png")).unwrap(), b"old");
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn two_saves_with_the_default_pattern_keep_both() {
        let dir = scratch();
        let settings = settings_in(&dir, crate::naming::DEFAULT_PATTERN);
        let first = save_new(b"one", &settings, 1, 1).unwrap();
        let second = save_new(b"two", &settings, 1, 1).unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(&first).unwrap(), b"one");
        assert_eq!(std::fs::read(&second).unwrap(), b"two");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn numbered_patterns_skip_taken_numbers() {
        let dir = scratch();
        std::fs::write(dir.join("shot-1.png"), b"old").unwrap();
        let path = save_new(b"new", &settings_in(&dir, "shot-%n%"), 1, 1).unwrap();
        assert_eq!(path, dir.join("shot-2.png"));
        assert_eq!(std::fs::read(dir.join("shot-1.png")).unwrap(), b"old");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn concurrent_saves_reserve_distinct_names() {
        let dir = scratch();
        let settings = settings_in(&dir, "shot");
        let saves: Vec<_> = (0..8u8)
            .map(|index| {
                let settings = settings.clone();
                std::thread::spawn(move || (index, save_new(&[index], &settings, 1, 1).unwrap()))
            })
            .collect();
        let mut paths = Vec::new();
        for save in saves {
            let (index, path) = save.join().unwrap();
            assert_eq!(std::fs::read(&path).unwrap(), [index]);
            paths.push(path);
        }
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), 8);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn auto_save_does_not_write_through_a_symlink() {
        let dir = scratch();
        let outside = scratch();
        let target = outside.join("target.png");
        std::fs::write(&target, b"keep").unwrap();
        std::os::unix::fs::symlink(&target, dir.join("shot.png")).unwrap();
        let path = save_new(b"new", &settings_in(&dir, "shot"), 1, 1).unwrap();
        assert_eq!(path, dir.join("shot-2.png"));
        assert_eq!(std::fs::read(&target).unwrap(), b"keep");
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn auto_save_rejects_a_pattern_that_leaves_the_folder() {
        let dir = scratch();
        assert!(save_new(b"new", &settings_in(&dir, "..\\outside"), 1, 1).is_err());
        assert!(!dir.parent().unwrap().join("outside.png").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
