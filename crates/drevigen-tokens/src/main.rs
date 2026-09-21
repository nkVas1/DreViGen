//! Generates the design tokens, and refuses to when the palette cannot be read.
//!
//! ```text
//! cargo run -p drevigen-tokens -- check       # audit only; exits non-zero on failure
//! cargo run -p drevigen-tokens -- generate    # audit, then write the files
//! ```
//!
//! `check` is what CI runs. The generated files are committed, so `generate` is a developer
//! action whose diff is reviewable — a colour change should be visible in a pull request, not
//! materialise at build time.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use drevigen_tokens::{audit, css, palette, report};

const OUT_DIR: &str = "packages/tokens/dist";

fn main() -> ExitCode {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "check".to_owned());

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
            eprintln!("\nUnknown mode {other:?}. Use `check` or `generate`.");
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
