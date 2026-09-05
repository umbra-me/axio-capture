//! Portable attachment: only the final annotated PNG and explicit metadata.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub fn export(png: &[u8], directory: &Path) -> Result<PathBuf> {
    ensure!(
        png.len() <= 32 * 1024 * 1024 && png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "expected a PNG under 32 MiB"
    );
    let (width, height) = crate::export::png_dimensions(png);
    ensure!(width > 0 && height > 0, "invalid image dimensions");
    let digest = format!("{:x}", Sha256::digest(png));
    std::fs::create_dir_all(directory)?;
    let file = format!("{digest}.png");
    let path = directory.join(&file);
    match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
    {
        Ok(mut output) => {
            output.write_all(png)?;
            output.sync_all()?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            ensure!(
                std::fs::read(&path)? == png,
                "attachment filename already holds different bytes"
            );
        }
        Err(e) => return Err(e.into()),
    }
    let manifest = directory.join(format!("{digest}.axio-capture.json"));
    let value = serde_json::json!({"schema_version": 1, "media_type": "image/png", "file": file,
        "sha256": digest, "width": width, "height": height});
    let bytes = serde_json::to_vec_pretty(&value)?;
    match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&manifest)
    {
        Ok(mut output) => {
            output.write_all(&bytes)?;
            output.sync_all()?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            ensure!(
                std::fs::read(&manifest)? == bytes,
                "attachment manifest already differs"
            );
        }
        Err(e) => return Err(e.into()),
    }
    Ok(manifest)
}
