use anyhow::{Context, Result};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbaImage};

use crate::state::{Screen, ScreenGeometry};

/// Capture every monitor.
///
/// Geometry units differ by platform and the overlay code relies on it:
/// xcap reports macOS monitors in logical points (`CGDisplayBounds`) and
/// Windows and X11 monitors in physical pixels. The captured image is always
/// physical pixels. The overlay maps its selection back through the image
/// size and its own viewport size, so it never needs to know the ratio.
pub fn grab_screens() -> Result<Vec<Screen>> {
    let monitors = xcap::Monitor::all().context("listing monitors")?;
    if monitors.is_empty() {
        anyhow::bail!("no monitors found");
    }

    let mut captured = Vec::with_capacity(monitors.len());
    for (index, monitor) in monitors.iter().enumerate() {
        let name = monitor
            .name()
            .unwrap_or_else(|_| format!("monitor {index}"));
        let image = monitor
            .capture_image()
            .with_context(|| format!("capturing {name}"))?;
        let geometry = ScreenGeometry {
            index,
            x: monitor.x().context("monitor x")?,
            y: monitor.y().context("monitor y")?,
            width: monitor.width().context("monitor width")?,
            height: monitor.height().context("monitor height")?,
            scale_factor: monitor.scale_factor().unwrap_or(1.0),
        };
        eprintln!(
            "axio-capture: monitor {index} {name:?} at ({}, {}) {}x{} scale {} -> image {}x{}",
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height,
            geometry.scale_factor,
            image.width(),
            image.height()
        );
        captured.push((geometry, image));
    }

    // Encoding is the slow part; do the monitors in parallel.
    let handles: Vec<_> = captured
        .into_iter()
        .map(|(geometry, image)| {
            std::thread::spawn(move || -> Result<Screen> {
                let png = encode_png(&image)?;
                Ok(Screen {
                    index: geometry.index,
                    geometry,
                    image,
                    png,
                })
            })
        })
        .collect();

    let mut screens = Vec::with_capacity(handles.len());
    for handle in handles {
        screens.push(
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("encoder thread panicked"))??,
        );
    }
    Ok(screens)
}

/// Fast PNG: these bytes only cross the local IPC bridge. The file the user
/// keeps is encoded by the editor from its canvas.
pub fn encode_png(image: &RgbaImage) -> Result<Vec<u8>> {
    encode_png_with(image, CompressionType::Fast)
}

/// Smaller output for a file the user keeps.
pub fn encode_png_final(image: &RgbaImage) -> Result<Vec<u8>> {
    encode_png_with(image, CompressionType::Default)
}

fn encode_png_with(image: &RgbaImage, compression: CompressionType) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity((image.width() * image.height()) as usize);
    let encoder = PngEncoder::new_with_quality(&mut out, compression, FilterType::Sub);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .context("encoding png")?;
    Ok(out)
}

/// A selection made in a viewport of `view_width` x `view_height` CSS pixels,
/// mapped onto the physical image and clamped to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn map_selection(
    image_width: u32,
    image_height: u32,
    view_width: f64,
    view_height: f64,
    sel: Selection,
) -> Option<PixelRect> {
    if view_width <= 0.0 || view_height <= 0.0 || sel.width <= 0.0 || sel.height <= 0.0 {
        return None;
    }
    let sx = f64::from(image_width) / view_width;
    let sy = f64::from(image_height) / view_height;
    let x0 = (sel.x * sx).round().clamp(0.0, f64::from(image_width));
    let y0 = (sel.y * sy).round().clamp(0.0, f64::from(image_height));
    let x1 = ((sel.x + sel.width) * sx)
        .round()
        .clamp(0.0, f64::from(image_width));
    let y1 = ((sel.y + sel.height) * sy)
        .round()
        .clamp(0.0, f64::from(image_height));
    let w = (x1 - x0) as u32;
    let h = (y1 - y0) as u32;
    if w == 0 || h == 0 {
        return None;
    }
    Some(PixelRect {
        x: x0 as u32,
        y: y0 as u32,
        width: w,
        height: h,
    })
}

/// Decode an image file the user opened or dropped, normalised to PNG bytes.
pub fn load_file(path: &std::path::Path) -> Result<Vec<u8>> {
    let decoded = image::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .into_rgba8();
    encode_png(&decoded)
}

pub fn crop(image: &RgbaImage, rect: PixelRect) -> RgbaImage {
    image::imageops::crop_imm(image, rect.x, rect.y, rect.width, rect.height).to_image()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(x: f64, y: f64, width: f64, height: f64) -> Selection {
        Selection {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn maps_retina_selection_to_physical_pixels() {
        let rect = map_selection(2880, 1800, 1440.0, 900.0, sel(10.0, 20.0, 100.0, 50.0)).unwrap();
        assert_eq!(
            rect,
            PixelRect {
                x: 20,
                y: 40,
                width: 200,
                height: 100
            }
        );
    }

    #[test]
    fn clamps_to_the_image() {
        let rect =
            map_selection(1000, 500, 1000.0, 500.0, sel(950.0, 480.0, 200.0, 200.0)).unwrap();
        assert_eq!(
            rect,
            PixelRect {
                x: 950,
                y: 480,
                width: 50,
                height: 20
            }
        );
    }

    #[test]
    fn rejects_empty_selections() {
        assert!(map_selection(1000, 500, 1000.0, 500.0, sel(10.0, 10.0, 0.0, 5.0)).is_none());
        assert!(map_selection(1000, 500, 1000.0, 500.0, sel(1000.0, 10.0, 5.0, 5.0)).is_none());
        assert!(map_selection(1000, 500, 0.0, 500.0, sel(1.0, 1.0, 5.0, 5.0)).is_none());
    }

    #[test]
    fn crops_the_requested_pixels() {
        let mut image = RgbaImage::new(4, 4);
        image.put_pixel(2, 3, image::Rgba([1, 2, 3, 4]));
        let out = crop(
            &image,
            PixelRect {
                x: 2,
                y: 3,
                width: 2,
                height: 1,
            },
        );
        assert_eq!(out.dimensions(), (2, 1));
        assert_eq!(out.get_pixel(0, 0), &image::Rgba([1, 2, 3, 4]));
    }
}
