use std::io::Read;

use crate::error::{Error, Result};

/// Download bytes from a URL (http/https).
pub fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
    let resp = ureq::get(url)
        .call()
        .map_err(|e| Error::Fetch(e.to_string()))?;
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| Error::Fetch(e.to_string()))?;
    Ok(buf)
}
