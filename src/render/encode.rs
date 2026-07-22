use image::GrayImage;

pub const PRINT_HEIGHT: u32 = 30;
const DATA_HEIGHT: u32 = 32; // 4 bytes × 8 bits per column

/// Convert a 1-bit (0 or 255) grayscale image → Dymo column bytes.
///
/// Protocol: each column = 4 bytes (32 bits).
/// Pixel (x=col, y=0) → byte[3] bit 7 of that column's 4 bytes.
/// i.e. row 0 is MSB of the last byte in each column group.
/// Per spec: single pixel at (0,0) → [00 00 00 80].
///
/// We skip row 0 (per "first row will be missed" warning) by offsetting
/// the image down one pixel when encoding.
///
/// The image is also stretched 2× horizontally (each column duplicated).
pub fn encode(image: &GrayImage) -> Vec<u8> {
    let (w, h) = image.dimensions();
    // Each output column (after 2× stretch): 4 bytes
    // Total output size: w * 2 columns * 4 bytes/column
    let mut out = Vec::with_capacity((w * 2 * 4) as usize);

    for x in 0..w {
        let col_bytes = encode_column(image, x, h);
        // Stretch 2×: write each column twice
        out.extend_from_slice(&col_bytes);
        out.extend_from_slice(&col_bytes);
    }
    out
}

/// Encode a single column (x) of the image into 4 bytes.
/// Row 0 of the image → bit 7 of byte[3] (skipping physical row 0 via offset).
fn encode_column(image: &GrayImage, x: u32, h: u32) -> [u8; 4] {
    let mut bits: u32 = 0;
    let usable = h.min(PRINT_HEIGHT);

    for row in 0..usable {
        // Skip physical row 0 by placing image row 0 → print row 1
        let print_row = row + 1;
        if print_row >= DATA_HEIGHT {
            break;
        }
        let pixel = image.get_pixel(x, row)[0];
        if pixel < 128 {
            // Black pixel → set bit. Row 0 → bit 31 (MSB of 32-bit word).
            // print_row 1 → bit 30, etc.
            bits |= 1 << (DATA_HEIGHT - 1 - print_row);
        }
    }

    // Big-endian byte order matches [byte3, byte2, byte1, byte0] → [00 00 00 80] for (0,0)
    // bits as u32 big-endian: MSB first
    bits.to_be_bytes()
}

/// Return (width_columns, height_rows) for the encoded image data.
/// width here is after 2× stretch.
pub fn encoded_dims(image: &GrayImage) -> (u32, u32) {
    (image.width() * 2, DATA_HEIGHT)
}
