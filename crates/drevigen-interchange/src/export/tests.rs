#![allow(clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeSet;

use drevigen_core::{Archive, ContributorId, Entity, EventKind, Resolution, Timestamp};
use drevigen_gedcom::{Document, Structure};

use super::{Options, Unwritten, export};
use crate::{Import, import};

const FAMILY: &str = "0 HEAD
1 SOUR GENERY
2 NAME Древо Жизни
1 SUBM @U1@
1 GEDC
2 VERS 5.5.1
1 CHAR UTF-8
0 @U1@ SUBM
1 NAME Николай Смирнов
0 @S1@ SOUR
1 TITL Метрическая книга, 1871
1 REPO @R1@
2 CALN ф. 230 оп. 1 д. 1437
0 @R1@ REPO
1 NAME Государственный архив Ярославской области
0 @I1@ INDI
1 NAME Иван Петрович /Смирнов/
2 FONE Ivan /Smirnov/
1 SEX M
1 BIRT
2 DATE 12 MAR 1871
2 PLAC с. Великое, Ярославский уезд, Ярославская губерния
3 MAP
4 LATI N57.0167
4 LONG E39.3833
2 SOUR @S1@
3 PAGE л. 17 об.
3 QUAY 3
1 OCCU кузнец
1 _MILT Служил в армии
2 DATE 1904
1 ASSO @I2@
2 RELA Godmother
1 FAMS @F1@
1 NOTE Старший сын.
0 @I2@ INDI
1 NAME Анна /Смирнова/
1 SEX F
1 FAMS @F1@
0 @I3@ INDI
1 NAME Ольга Ивановна /Смирнова/
1 FAMC @F1@
2 PEDI birth
2 _STATUS confirmed
0 @F1@ FAM
1 HUSB @I1@
1 WIFE @I2@
1 CHIL @I3@
1 MARR
2 DATE 3 FEB 1895
0 TRLR
";

fn imported(text: &str) -> Import {
    let options = crate::Options {
        file_name: "Смирновы.ged".to_owned(),
        by: ContributorId::from_raw(7),
        at: Timestamp(1_760_000_000_000),
    };
    let mut counter = 0;
    import(text.as_bytes(), &options, move || {
        counter += 1;
        counter
    })
    .unwrap()
}

fn written(
    archive: &Archive,
    provenance: BTreeSet<drevigen_core::SourceId>,
) -> (String, Vec<Unwritten>) {
    let options = Options {
        version: "0.1.0".to_owned(),
        at: Timestamp(1_760_000_000_000),
        provenance,
    };
    let export = export(archive, &options);
    (export.document.write(), export.report)
}

/// Exported as a cousin would receive it: without the file's own citations.
fn shared(file: &Import) -> String {
    written(&file.archive, BTreeSet::from([file.source])).0
}

fn record<'a>(document: &'a Document, tag: &str, nth: usize) -> &'a Structure {
    document
        .records
        .iter()
        .filter(|r| r.tag == tag)
        .nth(nth)
        .unwrap()
}

#[test]
fn an_export_is_valid_gedcom_7() {
    let text = shared(&imported(FAMILY));
    let document = Document::parse(&text).unwrap_or_else(|error| panic!("{error}\n{text}"));
    let head = &document.records[0];
    assert_eq!(
        head.child("GEDC")
            .unwrap()
            .child("VERS")
            .unwrap()
            .payload
            .text(),
        Some("7.0")
    );
    assert_eq!(
        head.child("DATE").unwrap().payload.text(),
        Some("9 OCT 2025")
    );
}

#[test]
fn names_dates_places_and_citations_are_written_as_gedcom_writes_them() {
    let document = Document::parse(&shared(&imported(FAMILY))).unwrap();
    let ivan = record(&document, "INDI", 0);
    let name = ivan.child("NAME").unwrap();
    assert_eq!(name.payload.text(), Some("Иван Петрович /Смирнов/"));
    assert_eq!(
        name.child("GIVN").unwrap().payload.text(),
        Some("Иван Петрович")
    );

    let birth = ivan.child("BIRT").unwrap();
    assert_eq!(
        birth.child("DATE").unwrap().payload.text(),
        Some("12 MAR 1871")
    );
    let place = birth.child("PLAC").unwrap();
    assert_eq!(
        place.payload.text(),
        Some("с. Великое, Ярославский уезд, Ярославская губерния")
    );
    let map = place.child("MAP").unwrap();
    assert_eq!(map.child("LATI").unwrap().payload.text(), Some("N57.0167"));
    let cited = birth.child("SOUR").unwrap();
    assert_eq!(
        cited.child("PAGE").unwrap().payload.text(),
        Some("л. 17 об.")
    );
    assert_eq!(cited.child("QUAY").unwrap().payload.text(), Some("3"));

    let source = record(&document, "SOUR", 0);
    let repo = source.child("REPO").unwrap();
    assert_eq!(
        repo.child("CALN").unwrap().payload.text(),
        Some("ф. 230 оп. 1 д. 1437")
    );
    assert_eq!(
        repo.payload.pointer(),
        record(&document, "REPO", 0).xref.as_deref()
    );
}

#[test]
fn what_was_kept_goes_back_where_it_was_found() {
    let document = Document::parse(&shared(&imported(FAMILY))).unwrap();
    let ivan = record(&document, "INDI", 0);
    assert_eq!(
        ivan.child("_MILT")
            .unwrap()
            .child("DATE")
            .unwrap()
            .payload
            .text(),
        Some("1904")
    );
    assert_eq!(
        ivan.child("NAME")
            .unwrap()
            .child("FONE")
            .unwrap()
            .payload
            .text(),
        Some("Ivan /Smirnov/")
    );
    // The pointer kept in ASSO leads to Анна under her new identifier.
    let anna = record(&document, "INDI", 1).xref.clone();
    assert_eq!(
        ivan.child("ASSO").unwrap().payload.pointer(),
        anna.as_deref()
    );

    // Kept from the person's side of the link, it goes back to that side.
    let olga = record(&document, "INDI", 2);
    let famc = olga.child("FAMC").unwrap();
    assert_eq!(famc.child("PEDI").unwrap().payload.text(), Some("BIRTH"));
    assert_eq!(
        famc.child("_STATUS").unwrap().payload.text(),
        Some("confirmed")
    );
    assert!(
        record(&document, "FAM", 0)
            .child("CHIL")
            .unwrap()
            .children
            .is_empty()
    );

    // The submitter was a record of its own, and is one again.
    let submitter = record(&document, "SUBM", 0);
    assert!(submitter.xref.is_some());
    assert_eq!(
        submitter.child("NAME").unwrap().payload.text(),
        Some("Николай Смирнов")
    );
}

#[test]
fn the_files_it_came_from_are_left_out_unless_asked_for() {
    let file = imported(FAMILY);
    let shared = Document::parse(&shared(&file)).unwrap();
    let titles: Vec<_> = shared
        .records
        .iter()
        .filter(|r| r.tag == "SOUR")
        .map(|r| r.child("TITL").unwrap().payload.text().unwrap().to_owned())
        .collect();
    assert_eq!(titles, ["Метрическая книга, 1871"]);
    assert!(
        record(&shared, "INDI", 0)
            .child("SEX")
            .unwrap()
            .children
            .is_empty()
    );

    let (everything, _) = written(&file.archive, BTreeSet::new());
    let everything = Document::parse(&everything).unwrap();
    assert_eq!(
        everything
            .records
            .iter()
            .filter(|r| r.tag == "SOUR")
            .count(),
        2
    );
}

#[test]
fn a_round_trip_brings_the_same_tree_back() {
    let first = imported(FAMILY);
    let second = imported(&shared(&first));
    assert_eq!(second.report.counts.people, 3);
    assert_eq!(second.report.counts.families, 1);
    assert_eq!(second.report.counts.events, first.report.counts.events);
    assert_eq!(second.report.counts.places, first.report.counts.places);

    let names = |archive: &Archive| -> BTreeSet<String> {
        archive
            .people()
            .flat_map(|person| person.names.iter())
            .filter_map(|name| match name.resolve() {
                Resolution::Agreed { assertion, .. } => Some(format!("{:?}", assertion.value)),
                _ => None,
            })
            .collect()
    };
    assert_eq!(names(&first.archive), names(&second.archive));

    let marriages = |archive: &Archive| {
        archive
            .events()
            .filter(|event| event.kind == EventKind::Marriage)
            .count()
    };
    assert_eq!(marriages(&second.archive), 1);
    let olga = second
        .archive
        .people()
        .find(|p| format!("{:?}", p.names[0].resolve()).contains("Ольга"))
        .unwrap()
        .id;
    assert_eq!(second.archive.parents_of(olga).len(), 2);
    assert_eq!(
        second
            .archive
            .notes_on(Entity::Person(second.archive.people().next().unwrap().id))
            .count(),
        1
    );
}

#[test]
fn a_disagreement_is_written_one_sided_and_reported() {
    let file = imported("0 HEAD\n0 @I1@ INDI\n1 NAME Иван /Смирнов/\n1 SEX M\n1 SEX F\n0 TRLR\n");
    let (_, report) = written(&file.archive, BTreeSet::from([file.source]));
    assert!(
        report
            .iter()
            .any(|entry| matches!(entry, Unwritten::Contested { .. }))
    );
}
