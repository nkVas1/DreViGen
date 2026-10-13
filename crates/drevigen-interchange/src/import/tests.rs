#![allow(clippy::unwrap_used, clippy::panic)]

use drevigen_core::{
    Archive, Attribute, AttributeKind, ChildRelation, ContributorId, Entity, EventKind, Fact,
    NameKind, Person, PersonId, Resolution, Role, Timestamp,
};
use drevigen_date::Repair;
use drevigen_gedcom::lenient::{NotGedcom, Severity};

use super::{Import, Mapping, Options, What, import};

const EVENTS: &str = "0 HEAD
1 GEDC
2 VERS 5.5.1
0 @I1@ INDI
1 NAME Иван Петрович /Смирнов/
1 SEX M
1 BIRT
2 DATE 12 MAR 1871
2 PLAC с. Великое, Ярославский уезд, Ярославская губерния
3 MAP
4 LATI N57.0167
4 LONG E39.3833
1 DEAT Y
1 OCCU кузнец
2 DATE 1897
2 PLAC с. Великое, Ярославский уезд, Ярославская губерния
1 CAST крестьянин
1 FACT Георгиевский кавалер
2 TYPE Награда
1 EVEN Служил в армии
2 TYPE Военная служба
1 FAMS @F1@
0 @I2@ INDI
1 NAME Анна /Смирнова/
1 BIRT
2 DATE 1873/74
2 PLAC д. Горки, Ярославский уезд, Ярославская губерния
1 FAMS @F1@
0 @F1@ FAM
1 HUSB @I1@
1 WIFE @I2@
1 MARR
2 DATE 3 FEB 1895
2 PLAC с. Великое, Ярославский уезд, Ярославская губерния
0 TRLR
";

fn agreed<V: drevigen_core::Claimable + Clone + core::fmt::Debug>(fact: &Fact<V>) -> V {
    match fact.resolve() {
        Resolution::Agreed { assertion, .. } => assertion.value.clone(),
        other => panic!("{other:?}"),
    }
}

const FAMILY: &str = "0 HEAD
1 SOUR GENERY
2 NAME Древо Жизни
2 VERS 5.0
1 SUBM @U1@
1 DATE 2 MAY 2019
1 GEDC
2 VERS 5.5.1
2 FORM LINEAGE-LINKED
1 CHAR UTF-8
0 @U1@ SUBM
1 NAME Николай Смирнов
0 @I1@ INDI
1 NAME Иван Петрович /Смирнов/
1 SEX M
1 FAMS @F1@
1 _MILT Served in the Imperial Army
2 DATE 1904
1 ASSO @I2@
2 RELA Godmother
0 @I2@ INDI
1 NAME Анна /Смирнова/
2 GIVN Анна
2 SURN Смирнова
2 TYPE married
2 FONE Anna /Smirnova/
1 SEX F
1 FAMS @F1@
0 @I3@ INDI
1 NAME Ольга Ивановна /Смирнова/
1 SEX F
1 FAMC @F1@
2 PEDI birth
2 _STATUS confirmed
1 NOTE Родилась в селе Великом.
0 @F1@ FAM
1 HUSB @I1@
1 WIFE @I2@
1 CHIL @I3@
0 TRLR
";

fn options() -> Options {
    Options {
        file_name: "Смирновы.ged".to_owned(),
        by: ContributorId::from_raw(7),
        at: Timestamp(1_760_000_000_000),
    }
}

fn imported(text: &str) -> Import {
    let mut counter = 0;
    import(text.as_bytes(), &options(), move || {
        counter += 1;
        counter
    })
    .unwrap_or_else(|error| panic!("{error}"))
}

fn person<'a>(archive: &'a Archive, given: &str) -> &'a Person {
    archive
        .people()
        .find(|person| {
            person.names.iter().any(|name| match name.resolve() {
                Resolution::Agreed { assertion, .. } => {
                    assertion.value.given.as_deref() == Some(given)
                }
                _ => false,
            })
        })
        .unwrap_or_else(|| panic!("nobody called {given}"))
}

fn name(person: &Person) -> drevigen_core::PersonName {
    match person.names[0].resolve() {
        Resolution::Agreed { assertion, .. } => assertion.value.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_family_comes_in_with_its_names_read_as_russian_names() {
    let file = imported(FAMILY);
    let archive = &file.archive;
    assert_eq!(archive.population(), 3);
    assert_eq!(file.report.counts.people, 3);
    assert_eq!(file.report.counts.families, 1);

    let ivan = name(person(archive, "Иван"));
    assert_eq!(ivan.patronymic.as_deref(), Some("Петрович"));
    assert_eq!(ivan.surname.as_deref(), Some("Смирнов"));

    let anna = name(person(archive, "Анна"));
    assert_eq!(anna.kind, Some(NameKind::Married));
    assert_eq!(anna.patronymic, None);

    let olga = person(archive, "Ольга").id;
    let parents = archive.parents_of(olga);
    assert!(parents.contains(&person(archive, "Иван").id));
    assert!(parents.contains(&person(archive, "Анна").id));
    let family = archive.families_as_child(olga).next().unwrap();
    let relation = &family.children[0].fact;
    assert!(matches!(
        relation.resolve(),
        Resolution::Agreed { assertion, .. } if assertion.value == ChildRelation::Birth
    ));

    assert!(
        file.report
            .at_least(Severity::Interpretation)
            .next()
            .is_none(),
        "{:?}",
        file.report.entries
    );
}

#[test]
fn the_file_is_a_source_and_cites_everything_it_says() {
    let file = imported(FAMILY);
    let source = file.archive.source(file.source).unwrap();
    assert_eq!(source.title, "Смирновы.ged");
    assert_eq!(source.author.as_deref(), Some("Николай Смирнов"));
    assert_eq!(
        source.publication.as_deref(),
        Some("Древо Жизни 5.0, 2 MAY 2019")
    );
    assert_eq!(file.report.version.as_deref(), Some("5.5.1"));

    let ivan = person(&file.archive, "Иван");
    let Resolution::Agreed { assertion, .. } = ivan.names[0].resolve() else {
        panic!()
    };
    let citation = file.archive.citation(assertion.citation.unwrap()).unwrap();
    assert_eq!(citation.source, file.source);
    assert_eq!(assertion.made.by, ContributorId::from_raw(7));
}

#[test]
fn what_the_archive_cannot_hold_is_kept_where_it_was() {
    let file = imported(FAMILY);
    let archive = &file.archive;
    let ivan = person(archive, "Иван");

    let kept: Vec<_> = archive.foreign_on(Entity::Person(ivan.id)).collect();
    assert!(kept.iter().any(|item| {
        item.format == "GEDCOM 5.5.1"
            && item.text == "0 _MILT Served in the Imperial Army\n1 DATE 1904\n"
    }));

    // A pointer in kept text names what it pointed at by its new identity.
    let anna = person(archive, "Анна").id;
    let association = kept
        .iter()
        .find(|item| item.text.starts_with("0 ASSO"))
        .unwrap();
    assert_eq!(
        association.text,
        format!("0 ASSO @{:032X}@\n1 RELA Godmother\n", anna.raw())
    );

    // A name's phonetic form is kept under the name, not the person.
    let anna_name = Entity::Fact(person(archive, "Анна").names[0].id);
    assert_eq!(
        archive.foreign_on(anna_name).next().unwrap().text,
        "0 FONE Anna /Smirnova/\n"
    );

    // What the person's side of a link carries is kept under the link.
    let olga = person(archive, "Ольга").id;
    let membership = archive.families_as_child(olga).next().unwrap().children[0]
        .fact
        .id;
    assert_eq!(
        archive
            .foreign_on(Entity::Fact(membership))
            .next()
            .unwrap()
            .text,
        "0 _STATUS confirmed\n"
    );

    // Records the archive cannot hold are kept under the file's source, the header among them.
    let records: Vec<_> = archive
        .foreign_on(Entity::Source(file.source))
        .map(|item| item.text.lines().next().unwrap().to_owned())
        .collect();
    assert!(records.iter().any(|line| line == "0 HEAD"));
    assert!(records.iter().any(|line| line.ends_with(" SUBM")));
    assert_eq!(file.report.kept.get("INDI._MILT"), Some(&1));
}

#[test]
fn notes_annotate_what_they_were_written_under() {
    let file = imported(FAMILY);
    let olga = person(&file.archive, "Ольга").id;
    let notes: Vec<_> = file.archive.notes_on(Entity::Person(olga)).collect();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "Родилась в селе Великом.");
}

#[test]
fn events_come_in_dated_and_placed() {
    let file = imported(EVENTS);
    let archive = &file.archive;
    let ivan = person(archive, "Иван").id;

    let birth = archive
        .events_of(ivan)
        .find(|event| event.kind == EventKind::Birth)
        .unwrap();
    let date = agreed(birth.date.as_ref().unwrap());
    assert_eq!(date.value.unwrap().to_gedcom7(), "12 MAR 1871");
    let village = archive
        .place(agreed(birth.place.as_ref().unwrap()))
        .unwrap();
    assert_eq!(village.names[0].name, "с. Великое");
    assert!(
        village.position.is_some(),
        "MAP gave the village its position"
    );

    let death = archive
        .events_of(ivan)
        .find(|event| event.kind == EventKind::Death)
        .unwrap();
    assert!(
        death.date.is_none(),
        "DEAT Y: it happened, nothing more is known"
    );

    let marriage = archive
        .events_of(ivan)
        .find(|event| event.kind == EventKind::Marriage)
        .unwrap();
    let roles: Vec<Role> = marriage
        .participants
        .iter()
        .map(|participation| agreed(&participation.role))
        .collect();
    assert_eq!(roles, [Role::Husband, Role::Wife]);
    let family = archive.families_as_partner(ivan).next().unwrap();
    assert_eq!(family.events, [marriage.id]);
}

#[test]
fn places_share_the_jurisdictions_they_have_in_common() {
    let file = imported(EVENTS);
    // A village, a hamlet, their uyezd and their governorate: four places, not nine.
    assert_eq!(file.report.counts.places, 4);
    let anna = person(&file.archive, "Анна").id;
    let birth = file.archive.events_of(anna).next().unwrap();
    let hamlet = file
        .archive
        .place(agreed(birth.place.as_ref().unwrap()))
        .unwrap();
    let uyezd = file.archive.place(hamlet.within[0].place).unwrap();
    assert_eq!(uyezd.names[0].name, "Ярославский уезд");
}

#[test]
fn attributes_carry_their_date_place_and_label() {
    let file = imported(EVENTS);
    let ivan = person(&file.archive, "Иван");
    let attributes: Vec<Attribute> = ivan.attributes.iter().map(agreed).collect();

    let occupation = &attributes[0];
    assert_eq!(occupation.kind, AttributeKind::Occupation);
    assert_eq!(occupation.value, "кузнец");
    assert_eq!(
        occupation
            .date
            .as_ref()
            .unwrap()
            .value
            .as_ref()
            .unwrap()
            .to_gedcom7(),
        "1897"
    );
    assert!(occupation.place.is_some());

    assert_eq!(
        attributes[1].kind,
        AttributeKind::Estate,
        "CAST is сословие"
    );
    assert_eq!(attributes[1].value, "крестьянин");
    assert_eq!(attributes[2].kind, AttributeKind::Other);
    assert_eq!(attributes[2].label.as_deref(), Some("Награда"));
}

#[test]
fn an_event_the_model_has_no_kind_for_is_kept_and_a_dual_year_is_reported() {
    let file = imported(EVENTS);
    let ivan = person(&file.archive, "Иван").id;
    assert!(file.archive.foreign_on(Entity::Person(ivan)).any(|item| {
        item.text
            .starts_with("0 EVEN Служил в армии\n1 TYPE Военная служба")
    }));
    assert_eq!(file.report.kept.get("INDI.EVEN"), Some(&1));
    assert!(file.report.entries.iter().any(|entry| matches!(
        &entry.what,
        What::Mapping(Mapping::Date { text, repair: Repair::DualYear { .. } }) if text == "1873/74"
    )));
}

#[test]
fn a_link_recorded_on_one_side_is_read_and_reported() {
    let file =
        imported("0 HEAD\n0 @I1@ INDI\n1 NAME Пётр /Смирнов/\n1 FAMC @F1@\n0 @F1@ FAM\n0 TRLR\n");
    let peter = person(&file.archive, "Пётр").id;
    assert_eq!(file.archive.families_as_child(peter).count(), 1);
    assert!(file.report.entries.iter().any(|entry| matches!(
        &entry.what,
        What::Mapping(Mapping::OneSidedLink { family }) if family == "F1"
    )));
}

#[test]
fn a_second_record_with_one_identifier_is_imported_and_reported() {
    let file = imported(
        "0 HEAD\n0 @I1@ INDI\n1 NAME Иван /Смирнов/\n0 @I1@ INDI\n1 NAME Пётр /Смирнов/\n0 TRLR\n",
    );
    assert_eq!(file.archive.population(), 2);
    assert!(file.report.entries.iter().any(|entry| matches!(
        &entry.what,
        What::Mapping(Mapping::SecondRecord { xref }) if xref == "I1"
    )));
}

#[test]
fn a_sex_gedcom_does_not_have_is_kept_not_guessed() {
    let file = imported("0 HEAD\n0 @I1@ INDI\n1 NAME Иван /Смирнов/\n1 SEX male\n0 TRLR\n");
    let ivan: PersonId = person(&file.archive, "Иван").id;
    assert!(file.archive.person(ivan).unwrap().sex.is_none());
    assert!(
        file.archive
            .foreign_on(Entity::Person(ivan))
            .any(|item| item.text == "0 SEX male\n")
    );
}

#[test]
fn text_that_is_not_gedcom_is_refused() {
    let mut counter = 0;
    let result = import(b"Dear cousin,\n", &options(), move || {
        counter += 1;
        counter
    });
    assert_eq!(result.map(|_| ()), Err(NotGedcom { line: 1 }));
}

mod names {
    use drevigen_gedcom::lenient;

    use super::super::names::person_name;

    /// The given name, patronymic and surname a `NAME` line, and whatever follows it, records.
    #[track_caller]
    fn reads(lines: &str, expected: [Option<&str>; 3]) {
        let reading = lenient::read(&format!("0 HEAD\n0 @I1@ INDI\n{lines}0 TRLR\n")).unwrap();
        let name = person_name(&reading.document.records[1].children[0]);
        let found = [name.given, name.patronymic, name.surname];
        assert_eq!(found.each_ref().map(Option::as_deref), expected, "{lines}");
    }

    #[test]
    fn a_patronymic_is_split_when_it_can_be_nothing_else() {
        let names = [Some("Иван"), Some("Петрович"), Some("Смирнов")];
        reads("1 NAME Иван Петрович /Смирнов/\n", names);
        // The full form needs no surname beside it.
        reads(
            "1 NAME Ольга Ильинична\n",
            [Some("Ольга"), Some("Ильинична"), None],
        );
        // Pre-reform spelling is recognised, and kept as written.
        reads(
            "1 NAME Петръ Ивановичъ /Смирновъ/\n",
            [Some("Петръ"), Some("Ивановичъ"), Some("Смирновъ")],
        );
    }

    #[test]
    fn a_short_patronymic_needs_a_surname_beside_it() {
        reads(
            "1 NAME Иван Петров /Смирнов/\n",
            [Some("Иван"), Some("Петров"), Some("Смирнов")],
        );
        // Without a surname, Петров may be the surname.
        reads("1 NAME Иван Петров\n", [Some("Иван Петров"), None, None]);
    }

    #[test]
    fn other_given_names_stay_given_names() {
        reads(
            "1 NAME Мария Ангелина /Смирнова/\n",
            [Some("Мария Ангелина"), None, Some("Смирнова")],
        );
        // Only Cyrillic is read for a patronymic.
        reads(
            "1 NAME Iwan Petrowitsch /Smirnow/\n",
            [Some("Iwan Petrowitsch"), None, Some("Smirnow")],
        );
    }

    #[test]
    fn parts_the_file_gives_win_over_the_value() {
        reads(
            "1 NAME Dr. John /Smith/\n2 NPFX Dr.\n",
            [Some("John"), None, Some("Smith")],
        );
        reads(
            "1 NAME Иван /Смирнов/\n2 SURN Смирнов-Петров\n",
            [Some("Иван"), None, Some("Смирнов-Петров")],
        );
    }
}
