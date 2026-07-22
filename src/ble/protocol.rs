use image::GrayImage;

use crate::render::encode;

const MAGIC: [u8; 2] = [0xFF, 0xF0];
const MAGIC2: [u8; 2] = [0x12, 0x34];
const MAX_CHUNK_DATA: usize = 500;

// Commands: escape byte + command byte
const CMD_START:       [u8; 6] = [0x1B, 0x73, 0x9A, 0x02, 0x00, 0x00];
const CMD_MEDIA_TYPE:  [u8; 3] = [0x1B, 0x4D, 0x00]; // last byte = media type (0 = default)
const CMD_DENSITY:     [u8; 3] = [0x1B, 0x43, 0x02]; // density = 2 (normal)
const CMD_FORM_FEED:   [u8; 2] = [0x1B, 0x45];
const CMD_STATUS:      [u8; 2] = [0x1B, 0x41];
const CMD_END:         [u8; 2] = [0x1B, 0x51];

/// Build the full payload (header + body) for printing an image.
/// Returns (header_bytes, chunks) ready to be written to the BLE characteristic.
pub fn build_print_payload(image: &GrayImage) -> (Vec<u8>, Vec<Vec<u8>>) {
    let col_bytes = encode::encode(image);
    let (width, height) = encode::encoded_dims(image);

    // PRINT_DATA command: 1B 44 01 02 WIDTH[4-LE] HEIGHT[4-LE] IMAGE_DATA
    let mut print_data = vec![0x1B, 0x44, 0x01, 0x02];
    print_data.extend_from_slice(&(width as u32).to_le_bytes());
    print_data.extend_from_slice(&(height as u32).to_le_bytes());
    print_data.extend_from_slice(&col_bytes);

    // Body = START + MEDIA_TYPE + DENSITY + PRINT_DATA + FORM_FEED + STATUS + END
    let mut body = Vec::new();
    body.extend_from_slice(&CMD_START);
    body.extend_from_slice(&CMD_MEDIA_TYPE);
    body.extend_from_slice(&CMD_DENSITY);
    body.extend_from_slice(&print_data);
    body.extend_from_slice(&CMD_FORM_FEED);
    body.extend_from_slice(&CMD_STATUS);
    body.extend_from_slice(&CMD_END);

    let header = build_header(body.len() as u32);
    let chunks = chunk_body(&body);

    (header, chunks)
}

/// Build the 9-byte header: FF F0 12 34 LEN[4-LE] CHECKSUM
fn build_header(payload_len: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(9);
    h.push(MAGIC[0]);    // FF
    h.push(MAGIC[1]);    // F0
    h.push(MAGIC2[0]);   // 12
    h.push(MAGIC2[1]);   // 34
    h.extend_from_slice(&payload_len.to_le_bytes());
    let checksum: u8 = h.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    h.push(checksum);
    h
}

/// Split body into 500-byte chunks. Each chunk: [INDEX(1)] + [DATA(≤500)].
/// Final chunk: [INDEX(1)] + [DATA] + [12 34].
fn chunk_body(body: &[u8]) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    let mut index: u8 = 0;
    let mut offset = 0;

    while offset < body.len() {
        let end = (offset + MAX_CHUNK_DATA).min(body.len());
        let is_last = end == body.len();

        let mut chunk = Vec::with_capacity(1 + (end - offset) + if is_last { 2 } else { 0 });
        chunk.push(index);
        chunk.extend_from_slice(&body[offset..end]);
        if is_last {
            chunk.extend_from_slice(&MAGIC2);
        }
        chunks.push(chunk);
        index = index.wrapping_add(1);
        offset = end;
    }

    chunks
}
