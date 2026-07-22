use ab_glyph::{Font, FontVec, PxScale, PxScaleFont, ScaleFont, VariableFont};
use image::{GrayImage, ImageBuffer, Luma};

use crate::error::{Error, Result};
use crate::render::PRINT_HEIGHT;

const MARGIN_X: u32 = 2;
const MARGIN_Y: u32 = 1;

const TAG_WGHT: [u8; 4] = *b"wght";

/// Render 1-3 lines of text into a grayscale image of height PRINT_HEIGHT.
///
/// `size` = desired cap-letter height in pixels per line (None = auto-fit to tape).
/// The font is scaled so that capital letters (measured via 'H') fill that height,
/// giving intuitive behaviour: --size 28 fills a 30-px tape (leaving 1 px margin).
pub fn render_text(
    lines: &[&str],
    font_data: &[u8],
    size: Option<f32>,
    weight: f32,
) -> Result<GrayImage> {
    if lines.is_empty() || lines.len() > 3 {
        return Err(Error::Font("1-3 text lines required".into()));
    }

    let mut font = FontVec::try_from_vec(font_data.to_vec())
        .map_err(|_| Error::Font("failed to parse font".into()))?;

    set_weight(&mut font, weight);

    let n = lines.len() as u32;
    let gap: u32 = if n > 1 { 2 } else { 0 };

    // Available vertical space for text (excl. top+bottom margins and inter-line gaps).
    let content_h = PRINT_HEIGHT.saturating_sub(MARGIN_Y * 2 + gap * (n - 1));
    let per_line_cap = size.unwrap_or((content_h / n) as f32);

    // Scale font so the 'H' glyph bounding box height ~ per_line_cap px.
    let scale = cap_to_scale(&font, per_line_cap);
    let scaled = font.as_scaled(scale);

    let actual_cap = glyph_cap_height(&scaled);

    let line_widths: Vec<u32> = lines
        .iter()
        .map(|line| measure_line(&scaled, line))
        .collect();
    let total_w = line_widths.iter().copied().max().unwrap_or(1) + MARGIN_X * 2;

    let mut img: GrayImage = ImageBuffer::from_pixel(total_w, PRINT_HEIGHT, Luma([255u8]));

    // Vertically centre the text block within the printable area.
    let block_h = actual_cap * n as f32 + gap as f32 * (n - 1) as f32;
    let v_pad = ((PRINT_HEIGHT as f32 - 2.0 * MARGIN_Y as f32 - block_h) / 2.0).max(0.0);
    let y0 = MARGIN_Y as f32 + v_pad;

    for (i, line) in lines.iter().enumerate() {
        // baseline_y: the font baseline for this line in image pixels.
        // Cap letters extend from (baseline_y - actual_cap) to baseline_y.
        let baseline_y = y0 + actual_cap + i as f32 * (actual_cap + gap as f32);
        draw_line(&mut img, &scaled, line, MARGIN_X as f32, baseline_y);
    }

    Ok(img)
}

// -- Font scaling helpers ------------------------------------------------------

fn set_weight(font: &mut FontVec, weight: f32) {
    for axis in font.variations() {
        if axis.tag == TAG_WGHT {
            let clamped = weight.clamp(axis.min_value, axis.max_value);
            font.set_variation(&TAG_WGHT, clamped);
            break;
        }
    }
}

/// Measure the pixel height of the 'H' glyph (cap height proxy).
fn glyph_cap_height<F: Font>(scaled: &PxScaleFont<F>) -> f32 {
    let g = scaled.scaled_glyph('H');
    if let Some(o) = scaled.outline_glyph(g) {
        let b = o.px_bounds();
        (b.max.y - b.min.y).max(1.0)
    } else {
        scaled.ascent().max(1.0)
    }
}

/// Return a PxScale whose 'H' glyph height ~ desired_cap_px.
fn cap_to_scale(font: &FontVec, desired_cap_px: f32) -> PxScale {
    let probe = PxScale::from(desired_cap_px);
    let cap = glyph_cap_height(&font.as_scaled(probe)).max(1.0);
    // Cap height is proportional to scale: new_scale = desired^2 / cap_at_probe.
    PxScale::from(desired_cap_px * desired_cap_px / cap)
}

// -- Drawing -------------------------------------------------------------------

fn measure_line<F: Font>(scaled: &PxScaleFont<F>, text: &str) -> u32 {
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

/// Draw `text` into `img` with the given `baseline_y` (pixel row of the font baseline).
/// Capital letters span upward from baseline; descenders extend below.
fn draw_line<F: Font>(
    img: &mut GrayImage,
    scaled: &PxScaleFont<F>,
    text: &str,
    x_start: f32,
    baseline_y: f32,
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
            // bounds are relative to the glyph origin (pen position = baseline-left).
            // bounds.min.y < 0 for ascenders/caps (above baseline).
            let gx = x + bounds.min.x;
            let gy = baseline_y + bounds.min.y;
            outlined.draw(|rx, ry, cov| {
                let px = (gx + rx as f32) as u32;
                let py = (gy + ry as f32) as u32;
                if px < img.width() && py < img.height() {
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
