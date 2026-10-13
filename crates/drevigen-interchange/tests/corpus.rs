//! Every file of the vendored gedcom7code corpus, and FamilySearch's official files when fetched,
//! through the whole import. The corpus lives with the format crate that first needed it:
//! `crates/drevigen-gedcom/tests/corpus/README.md` says where it is from.

#![allow(clippy::unwrap_used, clippy::panic, missing_docs)]

use std::path::{Path, PathBuf};

use drevigen_core::{ContributorId, Timestamp};
use drevigen_interchange::import::{Mapping, What};
use drevigen_interchange::{Import, Options, import};

fn files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "ged"))
        .collect();
    files.sort();
    files
}

fn imported(path: &Path) -> Import {
    let options = Options {
        file_name: path.file_name().unwrap().to_string_lossy().into_owned(),
        by: ContributorId::from_raw(1),
        at: Timestamp(1_760_000_000_000),
    };
    let mut counter = 0;
    import(&std::fs::read(path).unwrap(), &options, move || {
        counter += 1;
        counter
    })
    .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// What the importer must never do, whatever the file: have an edit refused, or leave a person or
/// family record out.
fn check(path: &Path) {
    let file = imported(path);
    let refused: Vec<_> = file
        .report
        .entries
        .iter()
        .filter(|entry| matches!(entry.what, What::Mapping(Mapping::Refused { .. })))
        .collect();
    assert!(refused.is_empty(), "{}: {refused:?}", path.display());

    let text = String::from_utf8_lossy(&std::fs::read(path).unwrap()).into_owned();
    // Records with an identifier and without one: 7.0 lets a record nothing points at go
    // without, and the official `xref.ged` has one.
    let records = |tag: &str| {
        text.lines()
            .filter_map(|line| line.strip_prefix("0 "))
            .map(|rest| match rest.strip_prefix('@') {
                Some(after) => after.split_once("@ ").map_or("", |(_, tag)| tag),
                None => rest,
            })
            .filter(|found| found.trim_end() == tag)
            .count()
    };
    if !text.contains('\0') {
        assert_eq!(
            file.report.counts.people,
            records("INDI"),
            "{}",
            path.display()
        );
        assert_eq!(
            file.report.counts.families,
            records("FAM"),
            "{}",
            path.display()
        );
    }
}

#[test]
fn every_corpus_file_imports_whole() {
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../drevigen-gedcom/tests/corpus/gedcom7code");
    for version in ["5", "7"] {
        for path in files(&corpus.join(version)) {
            check(&path);
        }
    }
}

#[test]
#[ignore = "needs the official files; run tools/fetch-gedcom-testfiles.sh"]
fn every_official_file_imports_whole() {
    let official = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/gedcom-testfiles");
    let all = files(&official);
    assert!(
        all.len() >= 16,
        "expected the full set, found {}",
        all.len()
    );
    for path in all {
        check(&path);
    }
}
