//! Renders the mark to the files every platform's icon pipeline expects.
//!
//! ```text
//! cargo run -p drevigen-mark --features render -- assets/identity apps/web/public
//! ```
//!
//! The first directory is the identity source the platform icon pipelines read. The second is
//! optional and holds what a browser needs instead: a favicon and the sizes a web app
//! manifest must declare.
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
//! | `adaptive-monochrome.png` | Android 13 themed icons: one shape, transparent |
//! | `mark.svg`, `mark-dark.svg`, `mark-mono.svg` | The vector source, for the web and for print |
//!
//! Into the web directory:
//!
//! | File | For |
//! |---|---|
//! | `favicon.svg` | The reduced mark: what survives a browser tab |
//! | `favicon-16.png`, `favicon-32.png` | The same, for anything that will not read the SVG |
//! | `icon-192.png`, `icon-512.png` | The two sizes a web app manifest must declare |
//! | `icon-maskable-512.png` | The same, inside the safe zone a launcher may crop to |
//! | `apple-touch-icon.png` | 180 px and opaque, because iOS composites nothing behind it |
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
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "assets/identity".to_owned());
    let web = args.next();
    let dir = Path::new(&out);

    if let Err(error) = fs::create_dir_all(dir) {
        eprintln!("cannot create {out}: {error}");
        return ExitCode::FAILURE;
    }

    let jobs: [(&str, Mark); 6] = [
        ("mark", Mark::new(Tone::Light)),
        ("mark-dark", Mark::new(Tone::Dark)),
        ("mark-mono", Mark::new(Tone::Monochrome)),
        // Android composites these two and crops the result to a launcher-dependent shape.
        (
            "adaptive-foreground",
            Mark::new(Tone::Light).transparent().inset(0.58),
        ),
        ("adaptive-background", ground_only()),
        // Android 13 themed icons: the system discards the colour and tints the alpha, so
        // this layer has to be one shape on nothing.
        (
            "adaptive-monochrome",
            Mark::new(Tone::Monochrome).transparent().inset(0.58),
        ),
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

    if let Some(web) = web
        && let Err(error) = write_web_icons(Path::new(&web))
    {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }

    println!("\nNext: cargo tauri icon -o apps/shell/src-tauri/icons {out}/tauri-icon.json");
    ExitCode::SUCCESS
}

/// Writes what a browser needs, which is a different set from what an operating system needs.
///
/// A favicon is seen at 16 px, so it gets the reduced mark. The manifest icons are seen on a
/// home screen, so they get the full one. The maskable variant is inset because a launcher may
/// crop it to any shape, and the Apple icon is opaque because iOS puts nothing behind it.
fn write_web_icons(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|error| format!("cannot create {}: {error}", dir.display()))?;

    let favicon = Mark::new(Tone::Light).reduced().to_svg(64);
    let path = dir.join("favicon.svg");
    fs::write(&path, &favicon)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    println!("  {}", path.display());

    let jobs: [(&str, u32, Mark); 6] = [
        // The PNG fallback for the tab, for the browsers and the operating systems that still
        // rasterise their own and get it wrong.
        ("favicon-16.png", 16, Mark::new(Tone::Light).reduced()),
        ("favicon-32.png", 32, Mark::new(Tone::Light).reduced()),
        ("icon-192.png", 192, Mark::new(Tone::Light)),
        ("icon-512.png", 512, Mark::new(Tone::Light)),
        (
            "icon-maskable-512.png",
            512,
            Mark::new(Tone::Light).inset(0.58),
        ),
        ("apple-touch-icon.png", 180, Mark::new(Tone::Light)),
    ];

    for (name, size, mark) in jobs {
        let png = rasterise(&mark.to_svg(size), size)?;
        let path = dir.join(name);
        fs::write(&path, png)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        println!("  {}", path.display());
    }

    Ok(())
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
