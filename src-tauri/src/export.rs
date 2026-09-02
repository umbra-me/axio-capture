use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

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

/// Where a new capture of `width` x `height` goes, per the settings.
pub fn new_capture_path(settings: &crate::settings::Settings, width: u32, height: u32) -> PathBuf {
    crate::naming::resolve(
        &save_dir(settings),
        &settings.file_pattern,
        crate::naming::Context { width, height },
    )
}

/// Width and height from a PNG's header, without decoding the pixels.
pub fn png_dimensions(png: &[u8]) -> (u32, u32) {
    image::ImageReader::new(std::io::Cursor::new(png))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .unwrap_or((0, 0))
}

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
