use image::{GrayImage, ImageBuffer, Luma};

/// Floyd-Steinberg dithering -> 1-bit (0 or 255).
pub fn floyd_steinberg(src: &GrayImage) -> GrayImage {
    let (w, h) = src.dimensions();
    let mut buf: Vec<f32> = src.pixels().map(|p| p[0] as f32).collect();

    let idx = |x: u32, y: u32| (y * w + x) as usize;

    for y in 0..h {
        for x in 0..w {
            let old = buf[idx(x, y)];
            let new = if old < 128.0 { 0.0 } else { 255.0 };
            buf[idx(x, y)] = new;
            let err = old - new;

            if x + 1 < w {
                buf[idx(x + 1, y)] += err * 7.0 / 16.0;
            }
            if y + 1 < h {
                if x > 0 {
                    buf[idx(x - 1, y + 1)] += err * 3.0 / 16.0;
                }
                buf[idx(x, y + 1)] += err * 5.0 / 16.0;
                if x + 1 < w {
                    buf[idx(x + 1, y + 1)] += err * 1.0 / 16.0;
                }
            }
        }
    }

    ImageBuffer::from_fn(w, h, |x, y| {
        let v = buf[idx(x, y)].clamp(0.0, 255.0) as u8;
        Luma([v])
    })
}

