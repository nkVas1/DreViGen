//! Generates the design tokens, and refuses to when the palette cannot be read.
//!
//! ```text
//! cargo run -p drevigen-tokens -- check         # audit only; exits non-zero on failure
//! cargo run -p drevigen-tokens -- generate      # audit, then write the files
//! cargo run -p drevigen-tokens -- check-assets  # no artwork invents a colour
//! ```
//!
//! `check` is what CI runs. The generated files are committed, so `generate` is a developer
//! action whose diff is reviewable — a colour change should be visible in a pull request, not
//! materialise at build time.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::fs;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::process::ExitCode;

use drevigen_tokens::{assets, audit, css, palette, report};

const OUT_DIR: &str = "packages/tokens/dist";

fn main() -> ExitCode {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "check".to_owned());

    // The artwork gate does not need the contrast table, and printing forty-five rows before
    // an unrelated answer is noise.
    if mode == "check-assets" {
        return check_assets();
    }

    let findings = audit();
    print!("{}", report::table(&findings));

    let failures = findings.iter().filter(|f| !f.passes()).count();
    if failures > 0 {
        eprintln!(
            "\nRefusing to continue: {failures} pairing(s) cannot be read.\n\
             Adjust the token in crates/drevigen-tokens/src/palette.rs and run again."
        );
        return ExitCode::FAILURE;
    }

    match mode.as_str() {
        "check" => {
            println!("\nPalette is readable. Run `generate` to write the files.");
            ExitCode::SUCCESS
        }
        "generate" => match generate() {
            Ok(written) => {
                println!("\nWrote:");
                for path in written {
                    println!("  {}", path.display());
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("\nGeneration failed: {error}");
                ExitCode::FAILURE
            }
        },
        other => {
            eprintln!("\nUnknown mode {other:?}. Use `check`, `generate` or `check-assets`.");
            ExitCode::FAILURE
        }
    }
}

fn generate() -> std::io::Result<Vec<PathBuf>> {
    let root = workspace_root();
    let dir = root.join(OUT_DIR);
    fs::create_dir_all(&dir)?;

    let themes = palette::themes();
    let files = [
        ("tokens.css", css::stylesheet(&themes)),
        ("tokens.ts", css::typescript(&themes)),
        ("contrast-report.txt", report::table(&audit())),
    ];

    let mut written = Vec::new();
    for (name, body) in files {
        let path = dir.join(name);
        fs::write(&path, body)?;
        written.push(path);
    }
    Ok(written)
}

/// Finds the workspace root by walking up from the executable's manifest directory.
///
/// `CARGO_MANIFEST_DIR` points at this crate, and the output belongs at the workspace root, so
/// the two must be related explicitly rather than by assuming the current directory — which is
/// whatever the developer happened to be standing in.
fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .find(|dir| dir.join("pnpm-workspace.yaml").exists())
        .map_or_else(|| manifest.to_path_buf(), Path::to_path_buf)
}

/// Directories whose vector artwork the interface renders, and which therefore may not carry a
/// colour the palette does not define.
///
/// Raster plates are deliberately absent: an engraving is a photograph of ink on paper and its
/// millions of shades are the point. It is the vectors that sit beside gated chrome.
const ARTWORK: [&str; 3] = ["assets/source", "packages/ui/icons", "packages/app/src"];

/// Fails if any vector carries a colour the palette does not define.
fn check_assets() -> ExitCode {
    let root = workspace_root();
    let mut scanned = 0_usize;
    let mut strays = Vec::new();

    for directory in ARTWORK {
        let base = root.join(directory);
        if !base.exists() {
            continue;
        }
        for path in svg_files(&base) {
            let Ok(source) = fs::read_to_string(&path) else {
                eprintln!("cannot read {}", path.display());
                return ExitCode::FAILURE;
            };
            scanned += 1;
            let label = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace(MAIN_SEPARATOR, "/");
            strays.extend(assets::stray_colours(&label, &source));
        }
    }

    if strays.is_empty() {
        println!("{scanned} vectors scanned. Every colour is one the palette defines.");
        return ExitCode::SUCCESS;
    }

    eprintln!(
        "Artwork carries {} colour(s) the palette does not define:",
        strays.len()
    );
    for stray in &strays {
        eprintln!("  {}", stray.file);
        eprintln!(
            "    #{} x{} - nearest is `{}`, {:.3} away in OKLab",
            stray.colour, stray.count, stray.nearest, stray.distance
        );
    }
    eprintln!();
    eprintln!("Fix the generator in tools/assets/, not the file: the files are regenerated.");
    eprintln!("Artwork that should follow the theme uses `currentColor`, with `color` on");
    eprintln!("the root element, so that a rasteriser still resolves it.");
    ExitCode::FAILURE
}

/// Every `.svg` under a directory, depth first.
fn svg_files(base: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![base.to_path_buf()];

    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}
