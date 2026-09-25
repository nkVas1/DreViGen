//! Archives for tests: small ones written by hand, and large ones grown from the testkit.
//!
//! Both go through the archive's checked edits — `assert_child`, `assert_partner` — rather than
//! building families whole, so every test that uses them also exercises the refusals, and a
//! generated tree of thousands of people doubles as a test that the refusals never fire on a
//! tree that is sound.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use drevigen_date::RecordedDate;
use drevigen_testkit::SyntheticTree;

use crate::archive::Archive;
use crate::claim::{Assertion, Provenance, Timestamp};
use crate::id::{AssertionId, ContributorId, EventId, FactId, FamilyId, PersonId};
use crate::model::{ChildRelation, Event, EventKind, Family, Partnership, Person, Role};
use crate::name::PersonName;
use crate::value::Sex;

/// An archive being built, with deterministic identities.
pub(crate) struct Scene {
    pub(crate) archive: Archive,
    next: u128,
}

impl Scene {
    pub(crate) fn new() -> Self {
        Self {
            archive: Archive::new(),
            next: 0,
        }
    }

    fn mint(&mut self) -> u128 {
        self.next += 1;
        self.next
    }

    fn assertion<V>(&mut self, value: V) -> Assertion<V> {
        let id = AssertionId::from_raw(self.mint());
        Assertion::new(
            id,
            value,
            Provenance {
                by: ContributorId::from_raw(1),
                at: Timestamp(0),
            },
        )
    }

    /// A person with a name, a sex, and whichever vital dates are given, each a GEDCOM 7 date.
    pub(crate) fn person(
        &mut self,
        given: &str,
        sex: Sex,
        born: Option<&str>,
        died: Option<&str>,
    ) -> PersonId {
        let id = PersonId::from_raw(self.mint());
        self.archive.add_person(Person::new(id)).unwrap();

        let (fact, name) = (
            FactId::from_raw(self.mint()),
            self.assertion(PersonName::russian(given, "", "Смирнов")),
        );
        self.archive.assert_name(id, fact, name).unwrap();
        let (fact, sex) = (FactId::from_raw(self.mint()), self.assertion(sex));
        self.archive.assert_sex(id, fact, sex).unwrap();

        if let Some(date) = born {
            self.event(EventKind::Birth, Some(date), &[(id, Role::Principal)]);
        }
        if let Some(date) = died {
            self.event(EventKind::Death, Some(date), &[(id, Role::Principal)]);
        }
        id
    }

    /// An event with a date and participants.
    pub(crate) fn event(
        &mut self,
        kind: EventKind,
        date: Option<&str>,
        participants: &[(PersonId, Role)],
    ) -> EventId {
        let id = EventId::from_raw(self.mint());
        self.archive.add_event(Event::new(id, kind)).unwrap();
        if let Some(date) = date {
            let fact = FactId::from_raw(self.mint());
            let value = RecordedDate::parse_gedcom7(date, None)
                .unwrap_or_else(|error| panic!("{date:?}: {error}"));
            let assertion = self.assertion(value);
            self.archive.assert_event_date(id, fact, assertion).unwrap();
        }
        for (person, role) in participants {
            let fact = FactId::from_raw(self.mint());
            let assertion = self.assertion(*role);
            self.archive
                .assert_participation(id, *person, fact, assertion)
                .unwrap();
        }
        id
    }

    /// A union of a husband and a wife with children born to them.
    pub(crate) fn union(
        &mut self,
        husband: Option<PersonId>,
        wife: Option<PersonId>,
        children: &[PersonId],
    ) -> FamilyId {
        let id = FamilyId::from_raw(self.mint());
        self.archive.add_family(Family::new(id)).unwrap();
        for (partner, how) in [(husband, Partnership::Husband), (wife, Partnership::Wife)] {
            if let Some(partner) = partner {
                let fact = FactId::from_raw(self.mint());
                let assertion = self.assertion(how);
                self.archive
                    .assert_partner(id, partner, fact, assertion)
                    .unwrap();
            }
        }
        for child in children {
            let fact = FactId::from_raw(self.mint());
            let assertion = self.assertion(ChildRelation::Birth);
            self.archive
                .assert_child(id, *child, fact, assertion)
                .unwrap();
        }
        id
    }
}

/// Grows an archive from a synthetic tree: every person with a name, a sex and their vital
/// events; every union with its children and its marriage.
pub(crate) fn from_synthetic(tree: &SyntheticTree) -> Scene {
    let mut scene = Scene::new();
    let mut people = Vec::with_capacity(tree.people.len());

    for person in &tree.people {
        let sex = match person.sex {
            drevigen_testkit::Sex::Male => Sex::Male,
            drevigen_testkit::Sex::Female => Sex::Female,
        };
        let born = person.birth_year.to_string();
        let died = person.death_year.map(|year| year.to_string());
        let id = scene.person(&person.given, sex, Some(&born), died.as_deref());
        people.push(id);
    }

    for family in &tree.families {
        let husband = family.husband.map(|h| people[h.0 as usize]);
        let wife = family.wife.map(|w| people[w.0 as usize]);
        let children: Vec<PersonId> = family
            .children
            .iter()
            .map(|c| people[c.0 as usize])
            .collect();
        scene.union(husband, wife, &children);

        let mut couple = Vec::new();
        if let Some(husband) = husband {
            couple.push((husband, Role::Husband));
        }
        if let Some(wife) = wife {
            couple.push((wife, Role::Wife));
        }
        let year = family.marriage_year.to_string();
        scene.event(EventKind::Marriage, Some(&year), &couple);
    }
    scene
}
