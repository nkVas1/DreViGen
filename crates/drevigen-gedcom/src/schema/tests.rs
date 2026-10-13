#![allow(clippy::unwrap_used)]

use super::grammar::accepts;
use super::{DataType, ViolationKind, validate};
use crate::document::Document;

fn violations(text: &str) -> Vec<(usize, ViolationKind)> {
    validate(&Document::parse(text).unwrap())
        .into_iter()
        .map(|violation| (violation.line, violation.kind))
        .collect()
}

const HEAD: &str = "0 HEAD\n1 GEDC\n2 VERS 7.0\n";

#[test]
fn a_minimal_and_a_typical_document_are_valid() {
    assert_eq!(violations(&format!("{HEAD}0 TRLR\n")), []);
    let typical = format!(
        "{HEAD}1 LANG ru\n0 @I1@ INDI\n1 NAME Иван Петрович /Смирнов/\n2 GIVN Иван Петрович\n1 SEX M\n1 BIRT\n2 DATE JULIAN 12 MAR 1871\n2 PLAC Великое\n3 MAP\n4 LATI N57.0167\n4 LONG E39.3833\n2 AGE < 1d\n1 FAMS @F1@\n1 _MILT Служил\n2 _ANY Anything goes beneath an extension\n0 @F1@ FAM\n1 HUSB @I1@\n1 MARR Y\n0 TRLR\n"
    );
    assert_eq!(violations(&typical), []);
}

#[test]
fn a_structure_out_of_place_is_reported_with_where_it_is() {
    let found = violations(&format!(
        "{HEAD}0 @I1@ INDI\n1 BIRT\n2 DATE 1871\n3 NOTE A note has no place under a date\n0 @X1@ MISE\n0 TRLR\n"
    ));
    assert_eq!(
        found,
        [
            (
                7,
                ViolationKind::NotAllowed {
                    tag: "NOTE".to_owned(),
                    within: Some("DATE".to_owned())
                }
            ),
            (
                8,
                ViolationKind::NotAllowed {
                    tag: "MISE".to_owned(),
                    within: None
                }
            ),
        ]
    );
}

#[test]
fn cardinalities_are_counted() {
    let found = violations("0 HEAD\n0 @I1@ INDI\n1 SEX M\n1 SEX F\n0 TRLR\n");
    assert_eq!(
        found,
        [
            (
                1,
                ViolationKind::Missing {
                    tag: "GEDC".to_owned(),
                    within: "HEAD".to_owned()
                }
            ),
            (
                4,
                ViolationKind::Repeated {
                    tag: "SEX".to_owned(),
                    within: "INDI".to_owned()
                }
            ),
        ]
    );
}

#[test]
fn payloads_pointers_and_enumerations_are_checked() {
    let found = violations(&format!(
        "{HEAD}0 @I1@ INDI\n1 SEX Q\n1 BIRT пропал без вести\n1 FAMC @I1@\n1 SEX _FLUID\n0 TRLR\n"
    ));
    let kinds: Vec<_> = found.into_iter().map(|(_, kind)| kind).collect();
    assert_eq!(
        kinds,
        [
            ViolationKind::NotInSet {
                tag: "SEX".to_owned(),
                value: "Q".to_owned()
            },
            ViolationKind::Payload {
                tag: "BIRT".to_owned(),
                expected: DataType::Y
            },
            ViolationKind::PointsAt {
                tag: "FAMC".to_owned(),
                expected: "FAM",
                found: "INDI".to_owned()
            },
            // The second SEX is an extension value, and allowed; that there are two is not.
            ViolationKind::Repeated {
                tag: "SEX".to_owned(),
                within: "INDI".to_owned()
            },
        ]
    );
}

#[test]
fn dates_and_times_follow_the_date_grammar() {
    for valid in [
        "",
        "1871",
        "JULIAN 12 MAR 1871",
        "BET 1870 AND 1872",
        "_CAL 5 _MON 300",
    ] {
        assert!(accepts(DataType::Date, valid), "{valid:?}");
    }
    for invalid in ["12 MAR", "31 FEB 1871", "about 1871"] {
        assert!(!accepts(DataType::Date, invalid), "{invalid:?}");
    }
    assert!(accepts(DataType::DateExact, "9 OCT 2025"));
    assert!(!accepts(DataType::DateExact, "OCT 2025"));
    assert!(accepts(DataType::DatePeriod, "FROM 1904 TO 1907"));
    assert!(!accepts(DataType::DatePeriod, "ABT 1904"));

    for valid in ["02:50", "2:50", "23:59:59.25", "14:00Z"] {
        assert!(accepts(DataType::Time, valid), "{valid:?}");
    }
    for invalid in ["24:00", "12:60", "12", "2:5"] {
        assert!(!accepts(DataType::Time, invalid), "{invalid:?}");
    }
}

#[test]
fn ages_names_and_coordinates_follow_their_grammars() {
    for valid in ["", "35y", "> 41y 3m", "< 8w 30d", "1y 400d"] {
        assert!(accepts(DataType::Age, valid), "{valid:?}");
    }
    for invalid in [">41y", "3m 1y", "41", "41 y"] {
        assert!(!accepts(DataType::Age, invalid), "{invalid:?}");
    }
    for valid in ["Иван /Смирнов/", "//", "Иван"] {
        assert!(accepts(DataType::PersonalName, valid), "{valid:?}");
    }
    for invalid in ["", "Иван /Смирнов", "a/b/c/d", "Иван\tСмирнов"] {
        assert!(!accepts(DataType::PersonalName, invalid), "{invalid:?}");
    }
    for valid in ["N18.150944", "S90", "N9", "N09.5"] {
        assert!(accepts(DataType::Latitude, valid), "{valid:?}");
    }
    for invalid in ["N91", "18.15", "N18.", "E18"] {
        assert!(!accepts(DataType::Latitude, invalid), "{invalid:?}");
    }
    for valid in ["E168.150944", "W180", "E099.5", "E39.3833"] {
        assert!(accepts(DataType::Longitude, valid), "{valid:?}");
    }
    assert!(!accepts(DataType::Longitude, "E181"));
}

#[test]
fn languages_media_types_paths_and_uris_follow_their_standards() {
    for valid in [
        "ru",
        "en-US",
        "zh-Hant-TW",
        "sr-Latn-RS",
        "de-1901",
        "x-whatever",
        "i-klingon",
    ] {
        assert!(accepts(DataType::Language, valid), "{valid:?}");
    }
    for invalid in ["", "r", "en_US", "en-", "toolongname"] {
        assert!(!accepts(DataType::Language, invalid), "{invalid:?}");
    }
    for valid in [
        "image/jpeg",
        "text/html; charset=utf-8",
        "application/x-gedcom",
        "x-foo/bar",
    ] {
        assert!(accepts(DataType::MediaType, valid), "{valid:?}");
    }
    for invalid in ["image", "image/", "text/html; charset"] {
        assert!(!accepts(DataType::MediaType, invalid), "{invalid:?}");
    }
    for valid in [
        "media/фото.jpg",
        "https://example.com/a%20b.jpg#face",
        "file:///home/me/x.jpg",
    ] {
        assert!(accepts(DataType::FilePath, valid), "{valid:?}");
    }
    for invalid in [
        "C:\\Photos\\x.jpg",
        "/home/me/x.jpg",
        "../x.jpg",
        "media/a b.jpg",
        "mailto:me@example.com",
        "media/x.jpg?v=1",
    ] {
        assert!(!accepts(DataType::FilePath, invalid), "{invalid:?}");
    }
    assert!(accepts(
        DataType::Uri,
        "https://gedcom.io/terms/v7/record-INDI"
    ));
    assert!(!accepts(DataType::Uri, "two words"));
    assert!(accepts(
        DataType::TagDefinition,
        "_MILT https://example.com/military"
    ));
    assert!(!accepts(
        DataType::TagDefinition,
        "MILT https://example.com"
    ));
}

#[test]
fn lists_and_enumerations_follow_their_grammars() {
    assert!(accepts(DataType::TextList, "Place, County, State, Country"));
    assert!(accepts(DataType::TextList, ", , one, more,"));
    assert!(!accepts(DataType::TextList, " leading"));
    assert!(accepts(DataType::EnumList, "BIRT, DEAT, MARR"));
    assert!(!accepts(DataType::EnumList, "BIRT,,DEAT"));
    assert!(accepts(DataType::Enum, "_CUSTOM"));
    assert!(accepts(DataType::Enum, "3"));
    assert!(!accepts(DataType::Enum, "birth"));
}
