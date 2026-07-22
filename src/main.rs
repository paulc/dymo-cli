mod cli;
mod error;
mod fetch;
mod fonts;
mod render;
mod web;
pub mod ble;

use std::io::{self, Write};

use image::{GrayImage, ImageBuffer};

use cli::{Args, Command};
use error::{Error, Result};
use render::RenderOptions;

#[tokio::main]
async fn main() {
    let args: Args = argh::from_env();
    if let Err(e) = run(args).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Scan(cmd)    => cmd_scan(cmd).await,
        Command::Print(cmd)   => cmd_print(cmd).await,
        Command::Preview(cmd) => cmd_preview(cmd),
        Command::Web(cmd)     => cmd_web(cmd).await,
    }
}

// ── scan ─────────────────────────────────────────────────────────────────────

async fn cmd_scan(cmd: cli::ScanCmd) -> Result<()> {
    println!("Scanning for Dymo printers ({} s)…", cmd.timeout);
    let printers = ble::scan(cmd.timeout).await?;

    if printers.is_empty() {
        println!("No printers found.");
        return Ok(());
    }

    println!("\nFound {} printer(s):", printers.len());
    for (i, p) in printers.iter().enumerate() {
        println!("  [{i}] {} — {}", p.name, p.address);
    }
    Ok(())
}

// ── print ─────────────────────────────────────────────────────────────────────

async fn cmd_print(cmd: cli::PrintCmd) -> Result<()> {
    let bitmap = build_bitmap_from_print_cmd(&cmd)?;

    let peripheral = match ble::connect(cmd.printer.as_deref()).await {
        Ok(p) => p,
        Err(Error::PrintFailed(ref msg)) if msg.contains("multiple printers") => {
            let printers = ble::scan(4).await?;
            let selected = prompt_select_printer(&printers)?;
            ble::connect(Some(&selected)).await?
        }
        Err(e) => return Err(e),
    };

    print!("Printing… ");
    io::stdout().flush().ok();
    ble::print_image(&peripheral, &bitmap).await?;
    ble::disconnect(&peripheral).await;
    println!("done.");
    Ok(())
}

fn build_bitmap_from_print_cmd(cmd: &cli::PrintCmd) -> Result<GrayImage> {
    let opts = make_render_opts(cmd.font.as_deref(), cmd.weight, cmd.size, cmd.italic, cmd.invert, cmd.no_dither);

    let raw = if let Some(src) = &cmd.image {
        render::render_image(src)?
    } else {
        validate_text(&cmd.text)?;
        let refs: Vec<&str> = cmd.text.iter().map(|s| s.as_str()).collect();
        render::render_text_lines(&refs, &opts)?
    };

    Ok(render::to_print_bitmap(&raw, !cmd.no_dither))
}

// ── preview ──────────────────────────────────────────────────────────────────

fn cmd_preview(cmd: cli::PreviewCmd) -> Result<()> {
    let opts = make_render_opts(cmd.font.as_deref(), cmd.weight, cmd.size, cmd.italic, cmd.invert, cmd.no_dither);

    let raw = if let Some(src) = &cmd.image {
        render::render_image(src)?
    } else {
        validate_text(&cmd.text)?;
        let refs: Vec<&str> = cmd.text.iter().map(|s| s.as_str()).collect();
        render::render_text_lines(&refs, &opts)?
    };

    let bitmap = render::to_print_bitmap(&raw, !cmd.no_dither);

    let s = cmd.scale.max(1);
    let (w, h) = bitmap.dimensions();
    let out: GrayImage = ImageBuffer::from_fn(w * s, h * s, |x, y| {
        *bitmap.get_pixel(x / s, y / s)
    });

    if cmd.output == "-" {
        let mut buf = std::io::Cursor::new(Vec::<u8>::new());
        out.write_to(&mut buf, image::ImageFormat::Png)?;
        use std::io::Write;
        io::stdout().write_all(buf.get_ref())?;
    } else {
        out.save_with_format(&cmd.output, image::ImageFormat::Png)?;
        eprintln!("Saved '{}' ({}×{} px, {}× scale)", cmd.output, w * s, h * s, s);
    }
    Ok(())
}

// ── web ───────────────────────────────────────────────────────────────────────

async fn cmd_web(cmd: cli::WebCmd) -> Result<()> {
    web::serve(cmd.port, cmd.open).await
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn make_render_opts(
    font: Option<&str>,
    weight: u32,
    size: Option<f32>,
    italic: bool,
    invert: bool,
    _no_dither: bool,
) -> RenderOptions {
    RenderOptions {
        font: font.map(str::to_owned),
        weight: weight as f32,
        size,
        italic,
        invert,
    }
}

fn validate_text(text: &[String]) -> Result<()> {
    if text.is_empty() {
        return Err(Error::Font("provide 1–3 text arguments or --image".into()));
    }
    if text.len() > 3 {
        return Err(Error::Font("maximum 3 text lines".into()));
    }
    Ok(())
}

fn prompt_select_printer(printers: &[ble::PrinterInfo]) -> Result<String> {
    println!("\nMultiple printers found:");
    for (i, p) in printers.iter().enumerate() {
        println!("  [{i}] {} — {}", p.name, p.address);
    }
    print!("Select printer [0–{}]: ", printers.len() - 1);
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(Error::Io)?;
    let idx: usize = input
        .trim()
        .parse()
        .map_err(|_| Error::PrintFailed("invalid selection".into()))?;
    printers
        .get(idx)
        .map(|p| p.address.clone())
        .ok_or_else(|| Error::PrintFailed("selection out of range".into()))
}
