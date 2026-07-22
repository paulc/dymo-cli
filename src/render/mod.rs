pub mod dither;
pub mod encode;
pub mod text;

use image::{imageops, DynamicImage, GrayImage, ImageBuffer, Luma};

use crate::error::Result;
use crate::fonts::FontData;

pub use encode::PRINT_HEIGHT;

#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub font: Option<String>,
    /// Variable font weight axis (100–900).
    pub weight: f32,
    /// Explicit pixel size. None = auto-fit.
    pub size: Option<f32>,
    pub italic: bool,
    pub invert: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            font: None,
            weight: 400.0,
            size: None,
            italic: false,
            invert: false,
        }
    }
}

/// Render text lines → smooth grayscale (for preview or print pipeline).
pub fn render_text_lines(lines: &[&str], opts: &RenderOptions) -> Result<GrayImage> {
    let font_name = opts.font.as_deref().unwrap_or("roboto");
    let font_data: FontData = crate::fonts::resolve(font_name, opts.italic)?;
    let mut img = text::render_text(lines, font_data.bytes(), opts.size, opts.weight)?;
    if opts.invert {
        invert_image(&mut img);
    }
    Ok(img)
}

/// Load an image source (path or URL) → smooth grayscale scaled to PRINT_HEIGHT.
pub fn render_image_smooth(source: &str) -> Result<GrayImage> {
    let dyn_img = load_image(source)?;
    Ok(scale_to_print_height(dyn_img))
}

/// Load an image source → dithered 1-bit grayscale scaled to PRINT_HEIGHT.
pub fn render_image(source: &str) -> Result<GrayImage> {
    let gray = render_image_smooth(source)?;
    Ok(dither::floyd_steinberg(&gray))
}

/// Convert a smooth grayscale image → print-ready 1-bit (dithered), padded to PRINT_HEIGHT.
pub fn to_print_bitmap(img: &GrayImage) -> GrayImage {
    let padded = pad_to_height(img);
    dither::floyd_steinberg(&padded)
}

/// Pad/crop a grayscale image to exactly PRINT_HEIGHT rows, centred vertically.
pub fn pad_to_height(img: &GrayImage) -> GrayImage {
    let (w, h) = img.dimensions();
    if h == PRINT_HEIGHT {
        return img.clone();
    }
    let mut out: GrayImage = ImageBuffer::from_pixel(w, PRINT_HEIGHT, Luma([255u8]));
    let copy_h = h.min(PRINT_HEIGHT);
    let y_offset = (PRINT_HEIGHT.saturating_sub(h)) / 2;
    for y in 0..copy_h {
        for x in 0..w {
            out.put_pixel(x, y + y_offset, *img.get_pixel(x, y));
        }
    }
    out
}

/// Legacy alias used by CLI preview/print paths.
pub fn to_label_bitmap(img: &GrayImage) -> GrayImage {
    to_print_bitmap(img)
}

// ── Private helpers ──────────────────────────────────────────────────────────

fn load_image(source: &str) -> Result<DynamicImage> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let bytes = crate::fetch::fetch_bytes(source)?;
        Ok(image::load_from_memory(&bytes)?)
    } else {
        Ok(image::open(source)?)
    }
}

fn scale_to_print_height(img: DynamicImage) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let new_w = ((w as f32 / h as f32) * PRINT_HEIGHT as f32).max(1.0) as u32;
    img.resize_exact(new_w, PRINT_HEIGHT, imageops::FilterType::Lanczos3)
        .to_luma8()
}

fn invert_image(img: &mut GrayImage) {
    for p in img.pixels_mut() {
        p[0] = 255 - p[0];
    }
}
