//! FamilySearch's own GEDCOM 7 test files, read and written back.
//!
//! The files are published for exactly this use and carry no licence permitting redistribution,
//! so they are not in this repository. `tools/fetch-gedcom-testfiles.sh` downloads them into the
//! git-ignored `target/gedcom-testfiles/`, and CI runs it before this test. Run locally with:
//!
//! ```text
//! tools/fetch-gedcom-testfiles.sh
//! cargo test -p drevigen-gedcom --test official -- --ignored
//! ```
//!
//! Ignored by default rather than silently passing when the files are absent: a test that passes
//! because it had nothing to check is worse than one that says it did not run.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::fs;
use std::path::PathBuf;

use drevigen_gedcom::{Document, Structure};

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/gedcom-testfiles")
}

fn strip_lines(mut document: Document) -> Document {
    fn strip(structure: &mut Structure) {
        structure.line = 0;
        structure.children.iter_mut().for_each(strip);
    }
    document.records.iter_mut().for_each(strip);
    document
}

#[test]
#[ignore = "needs the official files; run tools/fetch-gedcom-testfiles.sh"]
fn every_official_file_is_read_and_survives_a_round_trip() {
    let entries: Vec<PathBuf> = fs::read_dir(directory())
        .expect("run tools/fetch-gedcom-testfiles.sh first")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "ged"))
        .collect();
    assert!(
        entries.len() >= 16,
        "expected the full set, found {}",
        entries.len()
    );

    for path in entries {
        let text = fs::read_to_string(&path).unwrap();
        let document =
            Document::parse(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let again = Document::parse(&document.write())
            .unwrap_or_else(|error| panic!("{} rewritten: {error}", path.display()));
        assert_eq!(
            strip_lines(document),
            strip_lines(again),
            "{} changed on a round trip",
            path.display()
        );
    }
}
