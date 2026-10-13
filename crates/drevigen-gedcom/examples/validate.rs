//! Checks GEDCOM 7 files against the specification's rules and prints every violation.
//!
//! ```text
//! cargo run -p drevigen-gedcom --example validate -- file.ged [more.ged …]
//! ```
//!
//! Exits with 1 if any file is not valid GEDCOM 7.

#![allow(clippy::print_stdout, missing_docs)]

use drevigen_gedcom::{Document, validate};

fn main() {
    let mut invalid = false;
    for path in std::env::args().skip(1) {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                println!("{path}: {error}");
                invalid = true;
                continue;
            }
        };
        match Document::parse(&text) {
            Err(error) => {
                println!("{path}: not a GEDCOM 7 document: {error}");
                invalid = true;
            }
            Ok(document) => {
                let violations = validate(&document);
                println!("{path}: {} violations", violations.len());
                for violation in &violations {
                    println!("  {violation}");
                }
                invalid |= !violations.is_empty();
            }
        }
    }
    std::process::exit(i32::from(invalid));
}
