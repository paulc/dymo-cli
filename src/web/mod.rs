use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use image::{GrayImage, ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::error::Result;
use crate::render::{self, RenderOptions, PRINT_HEIGHT};

static INDEX_HTML: &str = include_str!("index.html");

// How much to enlarge the preview image for display in the browser.
// The label is ~30px tall; 8× gives ~240px which is comfortably readable.
const PREVIEW_SCALE: u32 = 8;

#[derive(Clone)]
struct AppState {
    printers: Arc<Mutex<Vec<(String, String)>>>,
}

#[derive(Deserialize)]
struct RenderRequest {
    lines: Vec<String>,
    image_url: Option<String>,
    font: Option<String>,
    weight: Option<u32>,
    size: Option<f32>,
    italic: Option<bool>,
    invert: Option<bool>,
    printer: Option<String>,
}

#[derive(Serialize)]
struct ScanResult {
    printers: Vec<PrinterEntry>,
}

#[derive(Serialize)]
struct PrinterEntry {
    name: String,
    address: String,
}

#[derive(Serialize)]
struct PrintResult {
    success: bool,
    message: String,
}

pub async fn serve(port: u16) -> Result<()> {
    let state = AppState {
        printers: Arc::new(Mutex::new(Vec::new())),
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/scan", get(scan_handler))
        .route("/api/preview", post(preview_handler))
        .route("/api/print", post(print_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| crate::error::Error::Web(e.to_string()))?;

    println!("Web interface running at http://localhost:{}", port);
    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::Error::Web(e.to_string()))
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn scan_handler(State(state): State<AppState>) -> Json<ScanResult> {
    let found = match crate::ble::scan(4).await {
        Ok(list) => list,
        Err(_) => vec![],
    };
    let entries: Vec<PrinterEntry> = found
        .iter()
        .map(|p| PrinterEntry { name: p.name.clone(), address: p.address.clone() })
        .collect();
    *state.printers.lock().await = found
        .iter()
        .map(|p| (p.name.clone(), p.address.clone()))
        .collect();
    Json(ScanResult { printers: entries })
}

async fn preview_handler(Json(req): Json<RenderRequest>) -> Response {
    // Use smooth grayscale (no dithering) for the in-browser preview so it
    // looks clean at display scale.
    let smooth = match make_smooth_bitmap(&req) {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    // Scale up uniformly — the natural W×30 proportions are preserved and the
    // label will appear as a wide thin strip for typical content.
    let s = PREVIEW_SCALE;
    let (w, h) = smooth.dimensions();
    let out: GrayImage = ImageBuffer::from_fn(w * s, h * s, |x, y| {
        *smooth.get_pixel(x / s, y / s)
    });

    let mut png: Vec<u8> = Vec::new();
    out.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    ([(header::CONTENT_TYPE, "image/png")], png).into_response()
}

async fn print_handler(Json(req): Json<RenderRequest>) -> Json<PrintResult> {
    let bitmap = match make_print_bitmap(&req) {
        Ok(b) => b,
        Err(e) => return Json(PrintResult { success: false, message: e.to_string() }),
    };

    let addr = req.printer.as_deref();
    match crate::ble::connect(addr).await {
        Ok(peripheral) => match crate::ble::print_image(&peripheral, &bitmap).await {
            Ok(_) => {
                crate::ble::disconnect(&peripheral).await;
                Json(PrintResult { success: true, message: "Printed successfully".into() })
            }
            Err(e) => Json(PrintResult { success: false, message: e.to_string() }),
        },
        Err(e) => Json(PrintResult { success: false, message: e.to_string() }),
    }
}

// ── Bitmap helpers ────────────────────────────────────────────────────────────

/// Smooth grayscale (no dithering) — for browser preview.
fn make_smooth_bitmap(req: &RenderRequest) -> crate::error::Result<GrayImage> {
    let opts = render_opts(req);
    let raw = if let Some(url) = &req.image_url {
        render::render_image_smooth(url)?
    } else {
        let refs: Vec<&str> = req.lines.iter().map(|s| s.as_str()).collect();
        if refs.is_empty() {
            return Ok(ImageBuffer::from_pixel(200, PRINT_HEIGHT, Luma([255u8])));
        }
        render::render_text_lines(&refs, &opts)?
    };
    Ok(render::pad_to_height(&raw))
}

/// Dithered 1-bit — for sending to the printer.
fn make_print_bitmap(req: &RenderRequest) -> crate::error::Result<GrayImage> {
    let opts = render_opts(req);
    let raw = if let Some(url) = &req.image_url {
        render::render_image(url)?
    } else {
        let refs: Vec<&str> = req.lines.iter().map(|s| s.as_str()).collect();
        if refs.is_empty() {
            return Ok(ImageBuffer::from_pixel(200, PRINT_HEIGHT, Luma([255u8])));
        }
        render::render_text_lines(&refs, &opts)?
    };
    Ok(render::to_print_bitmap(&raw))
}

fn render_opts(req: &RenderRequest) -> RenderOptions {
    RenderOptions {
        font: req.font.clone(),
        weight: req.weight.unwrap_or(400) as f32,
        size: req.size,
        italic: req.italic.unwrap_or(false),
        invert: req.invert.unwrap_or(false),
    }
}
