use crate::error::{Error, Result};

// Embedded variable fonts
static ROBOTO:             &[u8] = include_bytes!("../../assets/fonts/Roboto[wdth,wght].ttf");
static ROBOTO_ITALIC:      &[u8] = include_bytes!("../../assets/fonts/Roboto-Italic[wdth,wght].ttf");
static ROBOTO_MONO:        &[u8] = include_bytes!("../../assets/fonts/RobotoMono[wght].ttf");
static ROBOTO_MONO_ITALIC: &[u8] = include_bytes!("../../assets/fonts/RobotoMono-Italic[wght].ttf");
static ROUTED_GOTHIC:      &[u8] = include_bytes!("../../assets/fonts/routed-gothic.ttf");
static ROUTED_GOTHIC_NARROW: &[u8] = include_bytes!("../../assets/fonts/routed-gothic-narrow.ttf");
static ROUTED_GOTHIC_WIDE:   &[u8] = include_bytes!("../../assets/fonts/routed-gothic-wide.ttf");

pub enum FontData {
    Static(&'static [u8]),
    Owned(Vec<u8>),
}

impl FontData {
    pub fn bytes(&self) -> &[u8] {
        match self {
            FontData::Static(b) => b,
            FontData::Owned(v)  => v,
        }
    }
}

/// Resolve a font name/path/URL into raw TTF bytes.
pub fn resolve(name: &str, italic: bool) -> Result<FontData> {
    // External file or URL
    if name.starts_with("http://") || name.starts_with("https://") {
        let bytes = crate::fetch::fetch_bytes(name)?;
        return Ok(FontData::Owned(bytes));
    }
    if std::path::Path::new(name).exists() {
        let bytes = std::fs::read(name)?;
        return Ok(FontData::Owned(bytes));
    }

    // Built-in names
    let data: &'static [u8] = match name.to_lowercase().as_str() {
        "roboto" | "roboto-regular" => {
            if italic { ROBOTO_ITALIC } else { ROBOTO }
        }
        "roboto-mono" | "roboto mono" => {
            if italic { ROBOTO_MONO_ITALIC } else { ROBOTO_MONO }
        }
        "routed-gothic" | "routed gothic" => ROUTED_GOTHIC,
        "routed-gothic-narrow" | "routed gothic narrow" => ROUTED_GOTHIC_NARROW,
        "routed-gothic-wide"   | "routed gothic wide"   => ROUTED_GOTHIC_WIDE,
        _ => return Err(Error::Font(format!(
            "unknown font '{}'. Built-in fonts: roboto, roboto-mono, routed-gothic, \
             routed-gothic-narrow, routed-gothic-wide", name
        ))),
    };
    Ok(FontData::Static(data))
}

