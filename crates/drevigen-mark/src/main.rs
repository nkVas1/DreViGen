//! Renders the mark to the files every platform's icon pipeline expects.
//!
//! ```text
//! cargo run -p drevigen-mark --features render -- assets/identity
//! ```
//!
//! Output:
//!
//! | File | For |
//! |---|---|
//! | `mark-1024.png` | The source `cargo tauri icon` expands into every desktop and mobile size |
//! | `mark-dark-1024.png` | The same in the lamplit tone |
//! | `mark-mono-1024.png` | One colour, for a Windows taskbar and macOS template icons |
//! | `adaptive-foreground.png` | Android's foreground layer, inside the 66 % safe zone |
//! | `adaptive-background.png` | Android's background layer: the vellum ground |
//! | `mark.svg`, `mark-dark.svg`, `mark-mono.svg` | The vector source, for the web and for print |
//!
//! This is the mark the application ships with. The asset brief assigns a higher-fidelity
//! engraved version (plate A1) to a generative model; when that arrives it replaces these
//! files, and until then the product has a real mark rather than a placeholder.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use drevigen_mark::{Mark, Tone};
use resvg::usvg;

/// The size `cargo tauri icon` wants as its source, and what the stores ask for.
const SOURCE: u32 = 1024;

fn main() -> ExitCode {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/identity".to_owned());
    let dir = Path::new(&out);

    if let Err(error) = fs::create_dir_all(dir) {
        eprintln!("cannot create {out}: {error}");
        return ExitCode::FAILURE;
    }

    let jobs: [(&str, Mark); 5] = [
        ("mark", Mark::new(Tone::Light)),
        ("mark-dark", Mark::new(Tone::Dark)),
        ("mark-mono", Mark::new(Tone::Monochrome)),
        // Android composites these two and crops the result to a launcher-dependent shape.
        (
            "adaptive-foreground",
            Mark::new(Tone::Light).transparent().inset(0.58),
        ),
        ("adaptive-background", ground_only()),
    ];

    for (name, mark) in jobs {
        let svg = mark.to_svg(SOURCE);
        let svg_path = dir.join(format!("{name}.svg"));
        if let Err(error) = fs::write(&svg_path, &svg) {
            eprintln!("cannot write {}: {error}", svg_path.display());
            return ExitCode::FAILURE;
        }

        match rasterise(&svg, SOURCE) {
            Ok(png) => {
                let png_path = dir.join(format!("{name}-{SOURCE}.png"));
                if let Err(error) = fs::write(&png_path, png) {
                    eprintln!("cannot write {}: {error}", png_path.display());
                    return ExitCode::FAILURE;
                }
                println!("  {}", png_path.display());
            }
            Err(error) => {
                eprintln!("cannot rasterise {name}: {error}");
                return ExitCode::FAILURE;
            }
        }
    }

    println!("\nNext: cargo tauri icon {out}/mark-{SOURCE}.png");
    ExitCode::SUCCESS
}

/// The Android background layer is the ground with nothing on it.
///
/// A launcher may crop this to any shape, so it must carry no detail that could be lost — which
/// is why it is a flat surface rather than the grain the interface uses.
fn ground_only() -> Mark {
    Mark::new(Tone::Light).inset(0.001)
}

/// Rasterises SVG to a PNG byte vector.
fn rasterise(svg: &str, size: u32) -> Result<Vec<u8>, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .map_err(|error| format!("the SVG did not parse: {error}"))?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| format!("cannot allocate a {size}×{size} pixmap"))?;

    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    pixmap
        .encode_png()
        .map_err(|error| format!("PNG encoding failed: {error}"))
}
