//! Imports a GEDCOM file and prints what the import made of it: the encoding and how it was
//! settled, what the archive now holds, what was kept as foreign data and where, every kind of
//! entry in the report, and whether an export and a second import come back to the same place.
//!
//! ```text
//! cargo run --release -p drevigen-interchange --example inspect -- path/to/file.ged [export.ged]
//! ```
//!
//! For trying real files — someone's own tree, an export from another program — without putting
//! them anywhere: nothing is written but what is printed, and the export where one is asked for;
//! what is printed is counts and tags, not names.

#![allow(clippy::print_stdout, clippy::unwrap_used, missing_docs)]

#[path = "../tests/support/fingerprint.rs"]
mod fingerprint;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use drevigen_core::{ContributorId, Timestamp};
use drevigen_interchange::{Import, Options, export, import};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: inspect <file.ged> [export.ged]");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        eprintln!("{path}: {error}");
        std::process::exit(1);
    });
    let options = Options {
        file_name: "file.ged".to_owned(),
        by: ContributorId::from_raw(1),
        at: Timestamp(1_760_000_000_000),
    };
    let mut counter = 0;
    let mut entropy = move || {
        counter += 1;
        counter
    };

    let started = Instant::now();
    let first = import(&bytes, &options, &mut entropy).unwrap_or_else(|error| {
        eprintln!("not GEDCOM: {error}");
        std::process::exit(1);
    });
    println!("imported  in {:.2?}", started.elapsed());
    describe(&first);
    round_trip(&first, &options, &mut entropy);
}

/// What the import made of the file.
fn describe(first: &Import) {
    let report = &first.report;
    println!(
        "encoding  {} ({:?}), declared {:?}",
        report.encoding, report.basis, report.declared_encoding
    );
    println!(
        "version   {:?}, written by {:?}",
        report.version, report.program
    );
    println!("holds     {:?}", report.counts);
    let positioned = first
        .archive
        .places()
        .filter(|place| place.position.is_some())
        .count();
    println!("          {positioned} of the places with a position");

    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &report.entries {
        let kind = format!("{:?}", entry.what);
        let kind = kind
            .split(['(', '{', ' '])
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        *kinds.entry(kind).or_default() += 1;
    }
    println!(
        "\nreport    {} entries, {} more unlisted",
        report.entries.len(),
        report.unlisted
    );
    for (kind, count) in &kinds {
        println!("  {count:>6}  {kind}");
    }

    let mut kept: Vec<_> = report.kept.iter().collect();
    kept.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!("\nkept as foreign data, by where it was found");
    for (path, count) in kept {
        println!("  {count:>6}  {path}");
    }
}

/// Exports, checks the export with the strict GEDCOM 7 reader, imports it again and compares.
fn round_trip(first: &Import, options: &Options, entropy: &mut impl FnMut() -> u128) {
    let started = Instant::now();
    let written = export(
        &first.archive,
        &export::Options {
            version: "0.0.0".to_owned(),
            at: Timestamp(1_760_000_000_000),
            provenance: BTreeSet::from([first.source]),
        },
    );
    let text = written.document.write();
    // Written only where asked: a real tree is someone's family.
    if let Some(out) = std::env::args().nth(2) {
        std::fs::write(&out, &text).unwrap();
    }
    let verdict = match drevigen_gedcom::Document::parse(&text) {
        Ok(_) => "accepted".to_owned(),
        Err(error) => format!("REFUSED — {error}"),
    };
    println!(
        "\nexported  in {:.2?}: {} lines, strict 7.0 reader: {verdict}",
        started.elapsed(),
        text.lines().count()
    );
    let mut unwritten: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &written.report {
        let kind = format!("{entry:?}");
        let kind = kind.split([' ', '{']).next().unwrap_or_default().to_owned();
        *unwritten.entry(kind).or_default() += 1;
    }
    for (kind, count) in &unwritten {
        println!("  {count:>6}  not written as held: {kind}");
    }

    let second = import(text.as_bytes(), options, entropy).unwrap();
    // A 5.5.1 note record is a 7.0 SNOTE: the one place that is renamed on the way.
    let renamed = |kept: &BTreeMap<String, usize>| -> BTreeMap<String, usize> {
        kept.iter()
            .map(|(path, count)| {
                let path = path
                    .strip_prefix("NOTE.")
                    .map_or_else(|| path.clone(), |rest| format!("SNOTE.{rest}"));
                (path, *count)
            })
            .collect()
    };
    let (before, after) = (renamed(&first.report.kept), renamed(&second.report.kept));
    let same_counts = second.report.counts == first.report.counts;
    println!(
        "round trip  counts {}, kept data {}",
        same(same_counts),
        same(before == after)
    );
    if !same_counts {
        println!(
            "  before {:?}\n  after  {:?}",
            first.report.counts, second.report.counts
        );
    }
    let paths: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    for path in paths {
        if before.get(path) != after.get(path) {
            println!("  {path}: {:?} -> {:?}", before.get(path), after.get(path));
        }
    }

    // What the archive says, compared thing by thing; told here by kind only, not by content.
    let said = fingerprint::fingerprint(&first.archive, &BTreeSet::from([first.source]));
    let said_again = fingerprint::fingerprint(&second.archive, &BTreeSet::from([second.source]));
    println!("            content {}", same(said == said_again));
    for (direction, this, that) in [("lost", &said, &said_again), ("gained", &said_again, &said)] {
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        for (line, count) in fingerprint::difference(this, that) {
            let kind = line.split(' ').next().unwrap_or_default().to_owned();
            *kinds.entry(kind).or_default() += count;
        }
        for (kind, count) in kinds {
            println!("  {count:>6}  {kind} {direction}");
        }
    }
}

const fn same(same: bool) -> &'static str {
    if same { "the same" } else { "DIFFERENT" }
}
