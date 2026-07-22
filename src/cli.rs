use argh::FromArgs;

#[derive(FromArgs)]
/// Dymo LetraTag LT-200B BLE label printer CLI.
pub struct Args {
    #[argh(subcommand)]
    pub command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum Command {
    Scan(ScanCmd),
    Print(PrintCmd),
    Preview(PreviewCmd),
    Web(WebCmd),
}

/// Scan for nearby Dymo printers.
#[derive(FromArgs)]
#[argh(subcommand, name = "scan")]
pub struct ScanCmd {
    /// scan duration in seconds (default: 5)
    #[argh(option, short = 't', default = "5")]
    pub timeout: u64,
}

/// Print text or an image to a Dymo printer.
#[derive(FromArgs)]
#[argh(subcommand, name = "print")]
pub struct PrintCmd {
    /// text to print — 1, 2, or 3 positional arguments for 1/2/3 lines
    #[argh(positional)]
    pub text: Vec<String>,

    /// image file path or URL to print instead of text
    #[argh(option)]
    pub image: Option<String>,

    /// font: roboto (default), roboto-mono, routed-gothic,
    ///       routed-gothic-narrow, routed-gothic-wide, or a path/URL to a TTF
    #[argh(option, short = 'f')]
    pub font: Option<String>,

    /// font weight 100–900 (default: 400). Ignored for Routed Gothic.
    #[argh(option, short = 'w', default = "400")]
    pub weight: u32,

    /// font size in pixels (default: auto-fit to label height)
    #[argh(option, short = 's')]
    pub size: Option<f32>,

    /// use italic variant
    #[argh(switch, short = 'i')]
    pub italic: bool,

    /// invert colours (white text on black)
    #[argh(switch)]
    pub invert: bool,

    /// disable Floyd-Steinberg dithering (use hard threshold instead)
    #[argh(switch)]
    pub no_dither: bool,

    /// printer BLE address — auto-selected if only one printer is visible
    #[argh(option, short = 'p')]
    pub printer: Option<String>,
}

/// Render a label to a PNG file without printing.
#[derive(FromArgs)]
#[argh(subcommand, name = "preview")]
pub struct PreviewCmd {
    /// text to render — 1, 2, or 3 positional arguments
    #[argh(positional)]
    pub text: Vec<String>,

    /// image file path or URL to render instead of text
    #[argh(option)]
    pub image: Option<String>,

    /// font: roboto (default), roboto-mono, routed-gothic, or path/URL to TTF
    #[argh(option, short = 'f')]
    pub font: Option<String>,

    /// font weight 100–900 (default: 400)
    #[argh(option, short = 'w', default = "400")]
    pub weight: u32,

    /// font size in pixels
    #[argh(option, short = 's')]
    pub size: Option<f32>,

    /// use italic variant
    #[argh(switch, short = 'i')]
    pub italic: bool,

    /// invert colours
    #[argh(switch)]
    pub invert: bool,

    /// disable Floyd-Steinberg dithering (use hard threshold instead)
    #[argh(switch)]
    pub no_dither: bool,

    /// output PNG path (default: label.png); use - for stdout
    #[argh(option, short = 'o', default = "String::from(\"label.png\")")]
    pub output: String,

    /// scale the output PNG up for easier viewing (default: 4)
    #[argh(option, default = "4")]
    pub scale: u32,
}

/// Start the web interface.
#[derive(FromArgs)]
#[argh(subcommand, name = "web")]
pub struct WebCmd {
    /// port to listen on (default: 8080)
    #[argh(option, short = 'p', default = "8080")]
    pub port: u16,

    /// open a browser to the server after starting
    #[argh(switch)]
    pub open: bool,
}
