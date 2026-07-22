use ab_glyph::{Font, FontVec, PxScale, ScaleFont, VariableFont};
use image::{GrayImage, ImageBuffer, Luma};

use crate::error::{Error, Result};
use crate::render::PRINT_HEIGHT;

const MARGIN_X: u32 = 2;
const MARGIN_Y: u32 = 1;

const TAG_WGHT: [u8; 4] = *b"wght";

/// Render 1–3 lines of text into a grayscale image of height PRINT_HEIGHT.
pub fn render_text(
    lines: &[&str],
    font_data: &[u8],
    size_pt: Option<f32>,
    weight: f32,
) -> Result<GrayImage> {
    if lines.is_empty() || lines.len() > 3 {
        return Err(Error::Font("1–3 text lines required".into()));
    }

    let mut font = FontVec::try_from_vec(font_data.to_vec())
        .map_err(|_| Error::Font("failed to parse font".into()))?;

    set_weight(&mut font, weight);

    let n = lines.len() as u32;
    let gap: u32 = if n > 1 { 2 } else { 0 };
    let total_text_h = PRINT_HEIGHT.saturating_sub(MARGIN_Y * 2 + gap * (n - 1));
    let line_h = total_text_h / n;

    let px = size_pt.unwrap_or(line_h as f32);
    let scale = PxScale::from(px);
    let scaled = font.as_scaled(scale);

    let line_widths: Vec<u32> = lines
        .iter()
        .map(|line| measure_line(&scaled, line))
        .collect();
    let total_w = line_widths.iter().copied().max().unwrap_or(1) + MARGIN_X * 2;
    let total_w = total_w.max(1);

    let mut img: GrayImage = ImageBuffer::from_pixel(total_w, PRINT_HEIGHT, Luma([255u8]));

    let mut y_cursor = MARGIN_Y as f32;
    for (i, line) in lines.iter().enumerate() {
        draw_line(&mut img, &scaled, line, MARGIN_X as f32, y_cursor);
        y_cursor += line_h as f32;
        if i + 1 < lines.len() {
            y_cursor += gap as f32;
        }
    }

    Ok(img)
}

fn set_weight(font: &mut FontVec, weight: f32) {
    for axis in font.variations() {
        if axis.tag == TAG_WGHT {
            let clamped = weight.clamp(axis.min_value, axis.max_value);
            font.set_variation(&TAG_WGHT, clamped);
            break;
        }
    }
}

fn measure_line<F: Font>(scaled: &ab_glyph::PxScaleFont<F>, text: &str) -> u32 {
    let mut x = 0.0f32;
    let mut last = None;
    for ch in text.chars() {
        let glyph_id = scaled.glyph_id(ch);
        if let Some(prev) = last {
            x += scaled.kern(prev, glyph_id);
        }
        x += scaled.h_advance(glyph_id);
        last = Some(glyph_id);
    }
    x.ceil() as u32
}

fn draw_line<F: Font>(
    img: &mut GrayImage,
    scaled: &ab_glyph::PxScaleFont<F>,
    text: &str,
    x_start: f32,
    y_top: f32,
) {
    let mut x = x_start;
    let mut last = None;

    for ch in text.chars() {
        let glyph_id = scaled.glyph_id(ch);
        if let Some(prev) = last {
            x += scaled.kern(prev, glyph_id);
        }
        let glyph = scaled.scaled_glyph(ch);
        if let Some(outlined) = scaled.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            let gx = x + bounds.min.x;
            let gy = y_top + scaled.ascent() + bounds.min.y;
            outlined.draw(|rx, ry, cov| {
                let px = (gx + rx as f32) as u32;
                let py = (gy + ry as f32) as u32;
                if px < img.width() && py < img.height() {
                    // Blend coverage onto existing pixel (white background = 255)
                    let cur = img.get_pixel(px, py)[0] as f32;
                    let blended = (cur * (1.0 - cov)).round() as u8;
                    img.put_pixel(px, py, Luma([blended]));
                }
            });
        }
        x += scaled.h_advance(glyph_id);
        last = Some(glyph_id);
    }
}
