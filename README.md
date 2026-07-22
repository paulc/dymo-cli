# dymo-cli

Rust CLI and web interface for the Dymo LetraTag LT-200B Bluetooth label printer.

Protocol documentation sourced from the
[dymo-bluetooth](https://github.com/ysfchn/dymo-bluetooth) project by ysfchn.

## Requirements

- Rust 1.70+
- Linux: BlueZ (libdbus-1-dev)
- macOS: CoreBluetooth (no extra dependencies)

## Build

    cargo build --release

The binary is at `target/release/dymo-cli`.

## Rendering pipeline

Text or images are rendered to a 30-pixel-tall grayscale bitmap (the usable
height of the 12 mm tape at the printer's native resolution). For printing,
the bitmap is converted to 1-bit per pixel using Floyd-Steinberg dithering
(or a hard 128-midpoint threshold with --no-dither), then encoded as 4-byte
column words and stretched 2x horizontally per the protocol specification.
The web preview skips dithering and shows a smooth 8x-scaled version.

## Commands

### scan

Scan for nearby printers over Bluetooth.

    dymo-cli scan [--timeout SECS]

Default timeout is 5 seconds.

### print

Print text (1-3 lines) or an image.

    dymo-cli print [OPTIONS] TEXT [TEXT2] [TEXT3]
    dymo-cli print [OPTIONS] --image PATH_OR_URL

| Option | Short | Description |
|--------|-------|-------------|
| --font | -f | roboto (default), roboto-mono, routed-gothic, or path/URL to a TTF file |
| --weight | -w | Font weight 100-900 (default: 400). Variable fonts only. |
| --size | -s | Cap-letter height in pixels per line. Default: auto-fit to tape. |
| --italic | -i | Use italic variant. |
| --invert | | White text on black background. |
| --no-dither | | Hard threshold instead of Floyd-Steinberg dithering. |
| --printer | -p | Printer BLE address. Auto-selected if only one is visible. |

### preview

Render a label to a PNG file without printing.

    dymo-cli preview [OPTIONS] TEXT [TEXT2] [TEXT3]
    dymo-cli preview [OPTIONS] --image PATH_OR_URL

Accepts the same options as print, plus:

| Option | Short | Description |
|--------|-------|-------------|
| --output | -o | Output path (default: label.png). Use `-` to write to stdout. |
| --scale | | Integer scale factor for the output PNG (default: 4). |

### web

Start a local web interface.

    dymo-cli web [--port PORT] [--open]

Default port is 8080. `--open` launches a browser automatically.

## Examples

    # Single line
    dymo-cli print "Hello World"

    # Two lines, bold weight
    dymo-cli print --weight 700 "ITEM 4892" "Warehouse B"

    # Monospace font
    dymo-cli print --font roboto-mono "192.168.1.1"

    # Print an image
    dymo-cli print --image ./logo.png

    # Preview to file
    dymo-cli preview "Hello" --output label.png

    # Preview to stdout (pipe to viewer)
    dymo-cli preview "Hello" --output - > label.png

    # Web interface
    dymo-cli web --open
