use std::fmt;

#[derive(Debug)]
pub enum Error {
    Ble(btleplug::Error),
    Io(std::io::Error),
    Image(image::ImageError),
    Font(String),
    NoPrinters,
    PrintFailed(String),
    Web(String),
    Fetch(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Ble(e)          => write!(f, "BLE error: {e}"),
            Error::Io(e)           => write!(f, "IO error: {e}"),
            Error::Image(e)        => write!(f, "Image error: {e}"),
            Error::Font(s)         => write!(f, "Font error: {s}"),
            Error::NoPrinters      => write!(f, "No Dymo printers found"),
            Error::PrintFailed(s)  => write!(f, "Print failed: {s}"),
            Error::Web(s)          => write!(f, "Web server error: {s}"),
            Error::Fetch(s)        => write!(f, "Fetch error: {s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<btleplug::Error> for Error {
    fn from(e: btleplug::Error) -> Self { Error::Ble(e) }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self { Error::Io(e) }
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self { Error::Image(e) }
}

pub type Result<T> = std::result::Result<T, Error>;
