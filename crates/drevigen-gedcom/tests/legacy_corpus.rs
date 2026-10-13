//! The gedcom7code corpus: GEDCOM 5.5.1 files, each with the 7.0 file its author — the GEDCOM 7
//! editor — says it converts to. See `tests/corpus/README.md`.

#![allow(clippy::unwrap_used, clippy::panic, missing_docs)]

use std::path::PathBuf;

use drevigen_gedcom::lenient::{self, Reading, RepairKind};
use drevigen_gedcom::{Document, encoding};

fn corpus(version: &str) -> Vec<(String, String)> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus/gedcom7code")
        .join(version);
    let mut files: Vec<_> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let decoded = encoding::decode(&std::fs::read(&path).unwrap());
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            assert!(decoded.flaws.is_empty(), "{name}: {:?}", decoded.flaws);
            (name, decoded.text)
        })
        .collect()
}

fn lenient(name: &str, text: &str) -> Reading {
    lenient::read(text).unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn on_valid_70_the_lenient_reader_reads_what_the_strict_one_reads() {
    for (name, text) in corpus("7") {
        let strict = Document::parse(&text).unwrap_or_else(|error| panic!("{name}: {error}"));
        let reading = lenient(&name, &text);
        assert_eq!(reading.document, strict, "{name}");
        assert!(reading.repairs.is_empty(), "{name}: {:?}", reading.repairs);
    }
}

#[test]
fn the_551_files_need_only_the_repairs_their_content_calls_for() {
    for (name, text) in corpus("5") {
        let reading = lenient(&name, &text);
        let kinds: Vec<RepairKind> = reading.repairs.into_iter().map(|r| r.kind).collect();
        let expected: Vec<RepairKind> = match name.as_str() {
            // The header points at @test@, the record is @TEST@: identifiers are case-sensitive,
            // and the 7.0 file has @VOID@ there for the same reason.
            "xref-case.ged" => vec![RepairKind::DanglingPointer {
                xref: "test".to_owned(),
            }],
            // The two escapes that are not a calendar's, which 7.0 drops as well.
            "atsign.ged" => ["@#OLD FORM@"; 4]
                .into_iter()
                .chain(["@#OBSOLETE@"])
                .map(|escape| RepairKind::Escape {
                    escape: escape.to_owned(),
                })
                .collect(),
            _ => Vec::new(),
        };
        assert_eq!(kinds, expected, "{name}");
    }
}

#[test]
fn at_signs_and_escapes_read_as_the_70_conversion_has_them() {
    let old = corpus("5");
    let new = corpus("7");
    let find = |files: &[(String, String)]| {
        files
            .iter()
            .find(|(name, _)| name == "atsign.ged")
            .map(|(_, text)| text.clone())
            .unwrap()
    };
    let old = lenient("5/atsign.ged", &find(&old)).document;
    let new = Document::parse(&find(&new)).unwrap();

    let mut compared = 0;
    for converted in new.records.iter().filter(|r| r.tag == "SNOTE") {
        let xref = converted.xref.as_deref().unwrap();
        let original = old
            .records
            .iter()
            .find(|r| r.xref.as_deref() == Some(xref))
            .unwrap_or_else(|| panic!("@{xref}@ is not in the 5.5.1 file"));
        assert_eq!(original.payload, converted.payload, "@{xref}@");
        compared += 1;
    }
    assert_eq!(compared, 19);
}
