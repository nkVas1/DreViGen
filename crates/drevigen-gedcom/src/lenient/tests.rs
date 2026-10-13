#![allow(clippy::unwrap_used, clippy::panic)]

use super::{NotGedcom, Reading, RepairKind, Severity, read, unescape};
use crate::document::{Document, Payload, Structure};

const HEAD: &str = "0 HEAD\n1 GEDC\n2 VERS 5.5.1\n";

fn reading(body: &str) -> Reading {
    read(&format!("{HEAD}{body}0 TRLR\n")).unwrap_or_else(|error| panic!("{error}"))
}

fn record(reading: &Reading, xref: &str) -> Structure {
    reading
        .document
        .records
        .iter()
        .find(|record| record.xref.as_deref() == Some(xref))
        .cloned()
        .unwrap_or_else(|| panic!("no @{xref}@"))
}

fn kinds(reading: &Reading) -> Vec<RepairKind> {
    reading
        .repairs
        .iter()
        .map(|repair| repair.kind.clone())
        .collect()
}

#[test]
fn continuations_join_as_551_means_them() {
    let file = reading("0 @N1@ NOTE First li\n1 CONC ne, joined\n1 CONT second line\n");
    assert_eq!(
        record(&file, "N1").payload.text(),
        Some("First line, joined\nsecond line")
    );
    assert!(file.repairs.is_empty(), "{:?}", file.repairs);
}

#[test]
fn escapes_become_what_70_writes() {
    let (text, removed) = unescape("@#DFRENCH R@ 9 THER 2");
    assert_eq!(text, "FRENCH_R 9 THER 2");
    assert!(removed.is_empty());

    let (text, removed) = unescape("a@@b @ c @#Xodd@ d");
    assert_eq!(text, "a@b @ c d");
    assert_eq!(removed, ["@#Xodd@"]);

    // An escape that is never closed is not an escape.
    assert_eq!(unescape("@#DJULIAN 1917").0, "@#DJULIAN 1917");
}

#[test]
fn an_escape_split_by_conc_is_read_whole() {
    let file = reading("0 @I1@ INDI\n1 BIRT\n2 DATE @#DJUL\n3 CONC IAN@ 25 OCT 1917\n");
    let date = record(&file, "I1").children[0].children[0].clone();
    assert_eq!(date.payload.text(), Some("JULIAN 25 OCT 1917"));
}

#[test]
fn removed_escapes_are_reported() {
    let file = reading("0 @N1@ NOTE obsolete @#OLD FORM@ escape\n");
    assert_eq!(record(&file, "N1").payload.text(), Some("obsolete escape"));
    assert_eq!(
        kinds(&file),
        [RepairKind::Escape {
            escape: "@#OLD FORM@".to_owned()
        }]
    );
    assert_eq!(file.repairs[0].line, 4);
}

#[test]
fn what_551_told_readers_to_ignore_is_ignored() {
    let spaced = "0 HEAD\n\n  1 GEDC\n\t2 VERS 5.5.1\n\n0 TRLR\n";
    let file = read(spaced).unwrap();
    assert!(file.repairs.is_empty(), "{:?}", file.repairs);
    assert_eq!(file.document.records[0].children[0].children[0].line, 4);

    // LF CR is one terminator to 5.5.1. Counted as a text editor shows it, the lines double.
    let text = "0 HEAD\n1 GEDC\n2 VERS 5.5.1\n0 @I1@ INDI\n1 NAME Ivan /Smirnov/\n0 TRLR\n";
    let plain = read(text).unwrap();
    let lf_cr = read(&text.replace('\n', "\n\r")).unwrap();
    assert_eq!(
        lf_cr.document,
        plain.document.renumbered(|line| 2 * line - 1)
    );
    assert!(lf_cr.repairs.is_empty());
}

#[test]
fn identifiers_keep_their_spaces_and_their_case() {
    let file = reading(
        "0 @NoTe ref@ NOTE mixed case and space\n0 @I1@ INDI\n1 NOTE @NoTe ref@\n1 NOTE @note REF@\n",
    );
    let person = record(&file, "I1");
    assert_eq!(person.children[0].payload.pointer(), Some("NoTe ref"));
    assert_eq!(
        kinds(&file),
        [RepairKind::DanglingPointer {
            xref: "note REF".to_owned()
        }]
    );
}

#[test]
fn a_line_too_deep_is_read_one_level_down() {
    let file = reading("0 @I1@ INDI\n1 BIRT\n3 DATE 1871\n");
    let birth = &record(&file, "I1").children[0];
    assert_eq!(birth.children[0].tag, "DATE");
    assert_eq!(
        kinds(&file),
        [RepairKind::LevelJump {
            written: 3,
            read: 2
        }]
    );
}

#[test]
fn small_letter_tags_are_capitalised_and_reported_once() {
    let file = reading("0 @I1@ indi\n1 name Ivan\n0 @I2@ indi\n1 name Pyotr\n");
    assert_eq!(record(&file, "I2").tag, "INDI");
    assert_eq!(record(&file, "I2").children[0].tag, "NAME");
    assert_eq!(
        kinds(&file),
        [
            RepairKind::TagCase {
                written: "indi".to_owned()
            },
            RepairKind::TagCase {
                written: "name".to_owned()
            },
        ]
    );
}

#[test]
fn continuations_in_the_wrong_place_find_their_value() {
    // Written at the level of the line it continues — a common mistake.
    let file = reading("0 @N1@ NOTE first\n0 CONT second\n");
    assert_eq!(record(&file, "N1").payload.text(), Some("first\nsecond"));
    assert_eq!(kinds(&file), [RepairKind::MisplacedContinuation]);

    // After a substructure, its level still says which value it belongs to.
    let file = reading("0 @N1@ NOTE first\n1 SOUR @S1@\n1 CONT second\n0 @S1@ SOUR\n");
    let note = record(&file, "N1");
    assert_eq!(note.payload.text(), Some("first\nsecond"));
    assert_eq!(note.children[0].payload.pointer(), Some("S1"));
    assert_eq!(kinds(&file), [RepairKind::MisplacedContinuation]);

    // After a pointer there is nothing to continue; the line is kept as it is.
    let file = reading("0 @I1@ INDI\n1 FAMC @F1@\n2 CONT stray\n0 @F1@ FAM\n");
    let famc = &record(&file, "I1").children[0];
    assert_eq!(famc.children[0].tag, "CONT");
    assert_eq!(famc.children[0].payload.text(), Some("stray"));
    assert_eq!(kinds(&file), [RepairKind::OrphanContinuation]);
}

#[test]
fn lines_that_are_not_gedcom_join_the_value_before_them() {
    let file = reading(
        "0 @N1@ NOTE The family left in spring.\nThey reached Kazan in May.\n1890 was a hard year.\n",
    );
    assert_eq!(
        record(&file, "N1").payload.text(),
        Some("The family left in spring.\nThey reached Kazan in May.\n1890 was a hard year.")
    );
    assert!(
        kinds(&file)
            .iter()
            .all(|kind| matches!(kind, RepairKind::StrayLine { .. }))
    );
    assert_eq!(file.repairs.len(), 2);

    // With nothing before it that holds text, the line is lost — and reported with its words.
    let file = reading("0 @F1@ FAM\n1 HUSB @I1@\nnot a line\n0 @I1@ INDI\n");
    assert_eq!(
        kinds(&file),
        [RepairKind::LostLine {
            text: "not a line".to_owned()
        }]
    );
    assert_eq!(file.repairs[0].kind.severity(), Severity::Loss);
}

#[test]
fn control_characters_go_and_the_dos_end_of_file_goes_quietly() {
    let file = read(&format!(
        "{HEAD}0 @N1@ NOTE bell\u{7} and form feed\u{C}\n0 TRLR\n\u{1A}"
    ))
    .unwrap();
    assert_eq!(
        record(&file, "N1").payload.text(),
        Some("bell and form feed")
    );
    assert_eq!(kinds(&file), [RepairKind::ControlCharacters { count: 2 }]);
}

#[test]
fn the_ends_of_a_file() {
    let file = read("0 HEAD\n0 @I1@ INDI\n").unwrap();
    assert_eq!(kinds(&file), [RepairKind::NoTrailer]);
    assert_eq!(file.repairs[0].line, 2);

    let file = read("0 @I1@ INDI\n0 TRLR\n").unwrap();
    assert_eq!(kinds(&file), [RepairKind::NoHeader]);

    let file = read("0 HEAD\n0 TRLR\n0 @I1@ INDI\n0 TRLR\n").unwrap();
    assert_eq!(kinds(&file), [RepairKind::AfterTrailer]);
    assert_eq!(file.document.records.len(), 2, "kept, not dropped");
}

#[test]
fn text_that_is_not_gedcom_is_refused_at_once() {
    assert_eq!(read(""), Err(NotGedcom { line: 1 }));
    assert_eq!(
        read("\n\n{\"name\": \"Ivan\"}\n"),
        Err(NotGedcom { line: 3 })
    );
    assert_eq!(read("Dear cousin,\n0 HEAD\n"), Err(NotGedcom { line: 1 }));
    assert_eq!(
        read("1 NAME Ivan\n"),
        Err(NotGedcom { line: 1 }),
        "level 0 first"
    );
}

#[test]
fn a_file_that_declares_70_keeps_70s_notation() {
    let file = read(
        "0 HEAD\n1 GEDC\n2 VERS 7.0\n0 @N1@ SNOTE @@handle and a@@b\n1 CONT @@line two\n0 TRLR\n",
    )
    .unwrap();
    assert_eq!(
        record(&file, "N1").payload.text(),
        Some("@handle and a@@b\n@line two")
    );
}

#[test]
fn identifiers_used_twice_are_reported_and_both_kept() {
    let file = reading("0 @I1@ INDI\n0 @I1@ INDI\n");
    assert_eq!(
        kinds(&file),
        [RepairKind::DuplicateXref {
            xref: "I1".to_owned()
        }]
    );
    assert_eq!(file.document.records.len(), 3);
}

impl Document {
    /// The same document with every line number mapped.
    fn renumbered(mut self, map: impl Fn(usize) -> usize + Copy) -> Self {
        fn walk(structure: &mut Structure, map: impl Fn(usize) -> usize + Copy) {
            structure.line = map(structure.line);
            structure
                .children
                .iter_mut()
                .for_each(|child| walk(child, map));
        }
        self.records.iter_mut().for_each(|record| walk(record, map));
        self
    }
}

#[test]
fn whitespace_only_values_are_no_value() {
    let file = reading("0 @I1@ INDI\n1 BIRT  \n");
    assert_eq!(record(&file, "I1").children[0].payload, Payload::None);
}
