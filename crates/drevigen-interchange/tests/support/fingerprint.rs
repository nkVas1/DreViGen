//! What an archive says, written out without the identities it says it under: one line for each
//! place, person, event, family, source and note, counted. Two archives that say the same things —
//! a file imported, and the same file exported and imported again — have the same fingerprint,
//! however differently their entities are numbered.
//!
//! A fact is written as it resolves, with where each of its assertions is from: a source by its
//! title and the place in it, or "the file" for the citation an import makes of the file itself.
//! Notes and kept foreign data are written with what they are on.
//!
//! Shared by the corpus test and the `inspect` example, which include it by path.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use drevigen_core::{
    Archive, Attribute, CitationId, Claimable, Entity, Fact, PersonId, PlaceId, SourceId,
};

/// The lines, counted. `provenance` names the sources that are the imported files themselves.
pub(crate) fn fingerprint(
    archive: &Archive,
    provenance: &BTreeSet<SourceId>,
) -> BTreeMap<String, usize> {
    let print = Printer {
        archive,
        provenance,
    };
    let mut lines = BTreeMap::new();
    let mut add = |line: String| *lines.entry(line).or_insert(0) += 1;

    for place in archive.places() {
        add(format!(
            "place {} {:?}",
            print.place(place.id),
            place.position
        ));
    }
    for person in archive.people() {
        let names = sorted(person.names.iter().map(|name| print.fact(name)));
        let sex = person.sex.as_ref().map(|sex| print.fact(sex));
        let attributes = sorted(person.attributes.iter().map(|a| print.attribute(a)));
        add(format!(
            "person {names:?} sex {sex:?} attributes {attributes:?}{}",
            print.annotations(Entity::Person(person.id))
        ));
    }
    for event in archive.events() {
        let roles = sorted(event.participants.iter().map(|participation| {
            let person = print.person(participation.person);
            format!("{person} {}", print.fact(&participation.role))
        }));
        let place = event
            .place
            .as_ref()
            .map(|place| print.fact_with(place, |id| print.place(*id)));
        let texts: Vec<String> = event
            .texts
            .iter()
            .map(|(which, words)| format!("{which:?} {}", print.fact(words)))
            .collect();
        add(format!(
            "event {:?} {:?} date {:?} place {place:?} texts {texts:?} roles {roles:?}{}",
            event.kind,
            event.label,
            event.date.as_ref().map(|date| print.fact(date)),
            print.annotations(Entity::Event(event.id))
        ));
    }
    for family in archive.families() {
        let partners = sorted(family.partners.iter().map(|membership| {
            let person = print.person(membership.person);
            format!("{person} {}", print.fact(&membership.fact))
        }));
        let children = sorted(family.children.iter().map(|membership| {
            let person = print.person(membership.person);
            format!("{person} {}", print.fact(&membership.fact))
        }));
        add(format!(
            "family partners {partners:?} children {children:?} events {}{}",
            family.events.len(),
            print.annotations(Entity::Family(family.id))
        ));
    }
    for source in archive.sources() {
        if !provenance.contains(&source.id) {
            add(format!(
                "source {:?} {:?} {:?}{}",
                source.title,
                source.author,
                source.publication,
                print.annotations(Entity::Source(source.id))
            ));
        }
    }
    for note in archive.notes() {
        add(format!("note {:?}", note.text));
    }
    lines
}

/// The lines one fingerprint has more of than the other, each with how many more.
pub(crate) fn difference(
    this: &BTreeMap<String, usize>,
    that: &BTreeMap<String, usize>,
) -> Vec<(String, usize)> {
    this.iter()
        .filter_map(|(line, &count)| {
            let other = that.get(line).copied().unwrap_or(0);
            (count > other).then(|| (line.clone(), count - other))
        })
        .collect()
}

fn sorted(lines: impl Iterator<Item = String>) -> Vec<String> {
    let mut lines: Vec<String> = lines.collect();
    lines.sort();
    lines
}

struct Printer<'a> {
    archive: &'a Archive,
    provenance: &'a BTreeSet<SourceId>,
}

impl Printer<'_> {
    /// A place by its name and the names of what it lies within, smallest first.
    fn place(&self, id: PlaceId) -> String {
        let mut names = Vec::new();
        let mut seen = BTreeSet::new();
        let mut current = self.archive.place(id);
        while let Some(place) = current {
            if !seen.insert(place.id) {
                break;
            }
            names.push(place.names.first().map_or("?", |name| name.name.as_str()));
            current = place
                .within
                .first()
                .and_then(|parent| self.archive.place(parent.place));
        }
        names.join(", ")
    }

    /// A person by their names, as they resolve.
    fn person(&self, id: PersonId) -> String {
        let Some(person) = self.archive.person(id) else {
            return "?".to_owned();
        };
        let names = sorted(
            person
                .names
                .iter()
                .filter_map(|name| name.resolve().value().map(|value| format!("{value:?}"))),
        );
        names.join(" / ")
    }

    fn fact<V: Claimable + Debug>(&self, fact: &Fact<V>) -> String {
        self.fact_with(fact, |value| format!("{value:?}"))
    }

    fn fact_with<V: Claimable>(&self, fact: &Fact<V>, value: impl Fn(&V) -> String) -> String {
        let resolved = fact
            .resolve()
            .value()
            .map_or_else(|| "unknown".to_owned(), value);
        let cited = sorted(
            fact.claims
                .assertions()
                .iter()
                .filter(|assertion| assertion.retracted.is_none())
                .map(|assertion| {
                    format!(
                        "{} {:?}",
                        self.cited(assertion.citation),
                        assertion.confidence
                    )
                }),
        );
        format!(
            "{resolved} {cited:?}{}",
            self.annotations(Entity::Fact(fact.id))
        )
    }

    fn attribute(&self, fact: &Fact<Attribute>) -> String {
        self.fact_with(fact, |attribute| {
            let place = attribute.place.map(|place| self.place(place));
            format!(
                "{:?} {:?} {:?} {:?} {place:?} {:?}",
                attribute.kind, attribute.label, attribute.value, attribute.date, attribute.address
            )
        })
    }

    fn cited(&self, citation: Option<CitationId>) -> String {
        let Some(citation) = citation.and_then(|id| self.archive.citation(id)) else {
            return "uncited".to_owned();
        };
        if self.provenance.contains(&citation.source) {
            return "the file".to_owned();
        }
        let title = self
            .archive
            .source(citation.source)
            .map_or("?", |source| source.title.as_str());
        format!(
            "{title} {:?} {:?}{}",
            citation.locator,
            citation.transcription,
            self.annotations(Entity::Citation(citation.id))
        )
    }

    /// The notes on a subject, and how many things were kept beneath it.
    fn annotations(&self, subject: Entity) -> String {
        let notes = sorted(
            self.archive
                .notes_on(subject)
                .map(|note| format!("{:?}", note.text)),
        );
        let kept = self.archive.foreign_on(subject).count();
        if notes.is_empty() && kept == 0 {
            String::new()
        } else {
            format!(" notes {notes:?} kept {kept}")
        }
    }
}
