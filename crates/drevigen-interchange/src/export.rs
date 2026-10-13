//! Writing an archive as GEDCOM 7: the other half of ADR 0011.
//!
//! Everything the archive holds is written in the form GEDCOM 7 has for it, and what an import
//! kept as foreign data goes back where it was found — beneath the structure written for its
//! subject, or, if it was a record of its own, as a record again. A kept structure whose line
//! matches one written from the archive — the remainder of something the import took apart, such
//! as a source's repository link with more beneath it than a call number — is merged into it
//! rather than written twice.
//!
//! What a file cannot hold is listed in the export's report: a disagreement between sources, of
//! which a file holds one side; a third partner; citations of a child's place in a family.
//!
//! The files an archive imported are sources too, and every value a file gave without a citation
//! cites it. Those citations are the archive's memory of where something came from, not evidence a
//! reader of the exported file could look up. The caller names the sources that are such files,
//! and they are left out with their citations; what an import kept as a record of its own — a
//! submitter, an unknown record — is still written, because it was in the file.

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use drevigen_core::{
    Archive, AttributeKind, CallNumber, ChildRelation, Citation, CitationId, Claimable, Confidence,
    Entity, Event, EventId, EventKind, Fact, Family, FamilyId, Foreign, ForeignId, NoteId,
    Partnership, Person, PersonId, PersonName, PlaceId, Resolution, Role, Sex, SourceId, Timestamp,
};
use drevigen_date::gregorian;
use drevigen_gedcom::{Document, Payload, Structure};

/// What an export needs from its caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The application's version, for the header.
    pub version: String,
    /// When the file is written, for the header's date.
    pub at: Timestamp,
    /// The sources that are files this archive imported (ADR 0011): left out, with every citation
    /// of them.
    pub provenance: BTreeSet<SourceId>,
}

/// An archive, written as a GEDCOM 7 document.
#[derive(Debug, Clone)]
pub struct Export {
    /// The document. [`Document::write`] gives the file.
    pub document: Document,
    /// What it could not hold as the archive holds it.
    pub report: Vec<Unwritten>,
}

/// What an export could not write as the archive holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unwritten {
    /// A fact its sources disagree about. A file holds one value, so the provisional one is
    /// written, with the citations of every side.
    Contested {
        /// The fact.
        fact: drevigen_core::FactId,
    },
    /// A partner beyond the two a GEDCOM family has room for.
    ThirdPartner {
        /// The family.
        family: FamilyId,
        /// The partner left out of it.
        person: PersonId,
    },
    /// Citations of a child's place in a family, which GEDCOM has nowhere to write.
    ChildCitation {
        /// The family.
        family: FamilyId,
        /// The child.
        person: PersonId,
    },
    /// Someone's part in an event in a role GEDCOM writes elsewhere than in the event — a
    /// godparent, a witness. Not yet written.
    Role {
        /// The event.
        event: EventId,
        /// Who.
        person: PersonId,
    },
    /// An event with no principal and no family: nothing to write it under.
    Unplaced {
        /// The event.
        event: EventId,
    },
    /// Foreign data that does not read as GEDCOM 7 lines, and so is not written.
    Unreadable {
        /// The item.
        foreign: ForeignId,
    },
    /// A pointer in foreign data to something not written, written as `@VOID@`.
    Voided {
        /// The item.
        foreign: ForeignId,
    },
}

/// Writes an archive as a GEDCOM 7 document.
#[must_use]
pub fn export(archive: &Archive, options: &Options) -> Export {
    let mut exporter = Exporter::new(archive, options);
    let document = exporter.run();
    Export {
        document,
        report: exporter.report,
    }
}

struct Exporter<'a> {
    archive: &'a Archive,
    options: &'a Options,
    /// Every identity that becomes a record, and its cross-reference identifier.
    xrefs: BTreeMap<u128, String>,
    /// The notes written beneath the one subject they annotate, rather than as records.
    inline: BTreeSet<NoteId>,
    /// The events a family writes, which its partners therefore do not.
    family_events: BTreeSet<EventId>,
    report: Vec<Unwritten>,
}

impl<'a> Exporter<'a> {
    fn new(archive: &'a Archive, options: &'a Options) -> Self {
        Self {
            archive,
            options,
            xrefs: BTreeMap::new(),
            inline: BTreeSet::new(),
            family_events: archive
                .families()
                .flat_map(|family| family.events.iter().copied())
                .collect(),
            report: Vec::new(),
        }
    }

    fn run(&mut self) -> Document {
        self.assign();
        let archive = self.archive;
        let mut records = vec![self.header()];
        records.extend(archive.people().map(|person| self.person(person)));
        records.extend(archive.families().map(|family| self.family(family)));
        for source in archive.sources() {
            if !self.options.provenance.contains(&source.id) {
                records.push(self.source(source.id));
            }
        }
        records.extend(
            archive
                .repositories()
                .map(|repository| self.repository(repository.id)),
        );
        for note in archive.notes() {
            if !self.inline.contains(&note.id) {
                let mut shared = Structure::new("SNOTE").with_text(&note.text);
                shared.xref = self.xref(note.id.raw());
                self.finish(&mut shared, Entity::Note(note.id));
                records.push(shared);
            }
        }
        for item in self.kept_records() {
            if let Some(record) = self.read(item) {
                records.push(record);
            }
        }
        Document { records }
    }

    // ── identifiers ─────────────────────────────────────────────────────────────────────────

    /// Gives every record its identifier, and decides which notes are written in place.
    fn assign(&mut self) {
        let archive = self.archive;
        let mut assign = |raw: u128, prefix: &str, count: &mut usize| {
            *count += 1;
            self.xrefs.insert(raw, format!("{prefix}{count}"));
        };
        let (mut people, mut families, mut sources, mut repositories) = (0, 0, 0, 0);
        for person in archive.people() {
            assign(person.id.raw(), "I", &mut people);
        }
        for family in archive.families() {
            assign(family.id.raw(), "F", &mut families);
        }
        for source in archive.sources() {
            if !self.options.provenance.contains(&source.id) {
                assign(source.id.raw(), "S", &mut sources);
            }
        }
        for repository in archive.repositories() {
            assign(repository.id.raw(), "R", &mut repositories);
        }

        // A note annotating exactly one subject is written beneath it. Any other is a record, so
        // that nothing is lost: one that is shared or annotates nothing, one carrying kept data an
        // inline note has no room for, and one that kept data points at.
        let mut uses: BTreeMap<NoteId, usize> = BTreeMap::new();
        for subject in self.subjects() {
            for note in archive.notes_on(subject) {
                *uses.entry(note.id).or_default() += 1;
            }
        }
        let pointed_at = self.pointed_at();
        let mut notes = 0;
        for note in archive.notes() {
            let bare = archive.foreign_on(Entity::Note(note.id)).next().is_none();
            if uses.get(&note.id) == Some(&1) && bare && !pointed_at.contains(&note.id.raw()) {
                self.inline.insert(note.id);
            } else {
                notes += 1;
                self.xrefs.insert(note.id.raw(), format!("N{notes}"));
            }
        }
        let records: Vec<ForeignId> = self.kept_records().map(|item| item.id).collect();
        for (index, id) in records.into_iter().enumerate() {
            self.xrefs.insert(id.raw(), format!("X{}", index + 1));
        }
    }

    /// Every identity some kept data points at. What it points at has to be a record.
    fn pointed_at(&self) -> BTreeSet<u128> {
        fn collect(structure: &Structure, found: &mut BTreeSet<u128>) {
            if let Payload::Pointer(target) = &structure.payload
                && let Ok(raw) = u128::from_str_radix(target, 16)
            {
                found.insert(raw);
            }
            for child in &structure.children {
                collect(child, found);
            }
        }
        let archive = self.archive;
        let mut subjects = self.subjects();
        subjects.extend(archive.notes().map(|note| Entity::Note(note.id)));
        subjects.extend(archive.sources().map(|source| Entity::Source(source.id)));
        let mut found = BTreeSet::new();
        for subject in subjects {
            for item in archive.foreign_on(subject) {
                if let Ok(structure) = Structure::read(&item.text) {
                    collect(&structure, &mut found);
                }
            }
        }
        found
    }

    fn xref(&self, raw: u128) -> Option<String> {
        self.xrefs.get(&raw).cloned()
    }

    fn pointer(&self, raw: u128) -> Payload {
        self.xrefs
            .get(&raw)
            .map_or(Payload::Void, |xref| Payload::Pointer(xref.clone()))
    }

    /// Every subject the export writes, for counting what annotates what.
    fn subjects(&self) -> Vec<Entity> {
        let archive = self.archive;
        let mut subjects = Vec::new();
        for person in archive.people() {
            subjects.push(Entity::Person(person.id));
            subjects.extend(person.names.iter().map(|fact| Entity::Fact(fact.id)));
            subjects.extend(person.sex.iter().map(|fact| Entity::Fact(fact.id)));
            subjects.extend(person.attributes.iter().map(|fact| Entity::Fact(fact.id)));
        }
        for event in archive.events() {
            subjects.push(Entity::Event(event.id));
            subjects.extend(event.date.iter().map(|fact| Entity::Fact(fact.id)));
            subjects.extend(event.place.iter().map(|fact| Entity::Fact(fact.id)));
            subjects.extend(event.participants.iter().map(|p| Entity::Fact(p.role.id)));
        }
        for family in archive.families() {
            subjects.push(Entity::Family(family.id));
            subjects.extend(family.partners.iter().map(|m| Entity::Fact(m.fact.id)));
            subjects.extend(family.children.iter().map(|m| Entity::Fact(m.fact.id)));
        }
        for source in archive.sources() {
            if !self.options.provenance.contains(&source.id) {
                subjects.push(Entity::Source(source.id));
            }
        }
        for citation in archive.citations() {
            if !self.options.provenance.contains(&citation.source) {
                subjects.push(Entity::Citation(citation.id));
            }
        }
        subjects.extend(archive.repositories().map(|r| Entity::Repository(r.id)));
        subjects
    }

    /// The records an import kept whole, wherever they were kept. The headers of imported files
    /// are not among them: a file has one header, and this one is written new.
    fn kept_records(&self) -> impl Iterator<Item = &'a Foreign> + use<'a> {
        let archive = self.archive;
        archive
            .sources()
            .flat_map(move |source| archive.foreign_on(Entity::Source(source.id)))
            .filter(|item| item.is_record() && item.path != "HEAD")
    }

    // ── records ─────────────────────────────────────────────────────────────────────────────

    fn header(&self) -> Structure {
        let mut head = Structure::new("HEAD")
            .with_child(Structure::new("GEDC").with_child(Structure::new("VERS").with_text("7.0")))
            .with_child(
                Structure::new("SOUR")
                    .with_text("DREVIGEN")
                    .with_child(Structure::new("VERS").with_text(&self.options.version))
                    .with_child(Structure::new("NAME").with_text("DreViGen")),
            );
        if let Some(date) = gedcom_day(self.options.at) {
            head = head.with_child(Structure::new("DATE").with_text(&date));
        }
        head
    }

    fn person(&mut self, person: &Person) -> Structure {
        let mut indi = Structure::new("INDI");
        indi.xref = self.xref(person.id.raw());
        // A citation supporting every name supports the person: an import reads a citation of the
        // person as a whole that way, and so it is written back that way, once.
        let shared = self.shared_citations(&person.names);
        for name in &person.names {
            if let Some(value) = self.value(name) {
                let mut written = name_structure(&value);
                for (citation, confidence) in self.citations(name) {
                    if !shared.iter().any(|(found, _)| *found == citation)
                        && let Some(cited) = self.citation(citation, confidence)
                    {
                        written.children.push(cited);
                    }
                }
                self.finish(&mut written, Entity::Fact(name.id));
                indi.children.push(written);
            }
        }
        for (citation, confidence) in shared {
            if let Some(cited) = self.citation(citation, confidence) {
                indi.children.push(cited);
            }
        }
        if let Some(sex) = &person.sex
            && let Some(value) = self.value(sex)
        {
            let mut written = Structure::new("SEX").with_text(value.tag());
            self.cite(&mut written, sex);
            self.finish(&mut written, Entity::Fact(sex.id));
            indi.children.push(written);
        }
        for attribute in &person.attributes {
            if let Some(value) = self.value(attribute) {
                let (tag, label) = match value.kind {
                    AttributeKind::Other => ("FACT", value.label.as_deref().or(Some("Fact"))),
                    kind => (attribute_tag(kind), None),
                };
                let mut written = Structure::new(tag);
                if !value.value.is_empty() {
                    written.payload = Payload::Text(value.value.clone());
                }
                if let Some(label) = label {
                    written
                        .children
                        .push(Structure::new("TYPE").with_text(label));
                }
                if let Some(date) = &value.date {
                    written.children.push(date_structure(date));
                }
                if let Some(place) = value.place {
                    written.children.push(self.place_structure(place));
                }
                self.cite(&mut written, attribute);
                self.finish(&mut written, Entity::Fact(attribute.id));
                indi.children.push(written);
            }
        }
        let archive = self.archive;
        for event in archive.events_of(person.id) {
            if self.family_events.contains(&event.id) {
                continue;
            }
            let principal = event
                .participants
                .first()
                .is_some_and(|p| p.person == person.id && peek(&p.role) == Some(Role::Principal));
            if principal {
                indi.children.push(self.event(event));
            }
        }
        for family in archive.families_as_child(person.id) {
            indi.children.push(self.as_child(person.id, family));
        }
        for family in archive.families_as_partner(person.id) {
            let mut link = Structure::new("FAMS");
            link.payload = self.pointer(family.id.raw());
            if let Some(membership) = family.partners.iter().find(|m| m.person == person.id) {
                self.finish_side(&mut link, Entity::Fact(membership.fact.id), "INDI.FAMS");
            }
            indi.children.push(link);
        }
        self.finish(&mut indi, Entity::Person(person.id));
        indi
    }

    /// A person's `FAMC`: the link from their side, with the relation the family gives them.
    fn as_child(&mut self, person: PersonId, family: &Family) -> Structure {
        let mut link = Structure::new("FAMC");
        link.payload = self.pointer(family.id.raw());
        if let Some(membership) = family.children.iter().find(|m| m.person == person) {
            if let Some(pedigree) = self.value(&membership.fact).and_then(pedigree_tag) {
                link.children
                    .push(Structure::new("PEDI").with_text(pedigree));
            }
            self.finish_side(&mut link, Entity::Fact(membership.fact.id), "INDI.FAMC");
        }
        link
    }

    fn family(&mut self, family: &Family) -> Structure {
        let mut fam = Structure::new("FAM");
        fam.xref = self.xref(family.id.raw());
        let mut slots: [Option<Structure>; 2] = [None, None];
        let mut cited = Vec::new();
        for membership in &family.partners {
            let partnership = self.value(&membership.fact);
            let sex = self
                .archive
                .person(membership.person)
                .and_then(|person| person.sex.as_ref())
                .and_then(peek);
            let wife = match partnership {
                Some(Partnership::Wife) => true,
                Some(Partnership::Husband) => false,
                _ => sex == Some(Sex::Female) || slots[0].is_some(),
            };
            let slot = usize::from(wife);
            if slots[slot].is_some() {
                self.report.push(Unwritten::ThirdPartner {
                    family: family.id,
                    person: membership.person,
                });
                continue;
            }
            let mut written = Structure::new(if wife { "WIFE" } else { "HUSB" });
            written.payload = self.pointer(membership.person.raw());
            let path = format!("FAM.{}", written.tag);
            self.finish_side(&mut written, Entity::Fact(membership.fact.id), &path);
            slots[slot] = Some(written);
            cited.extend(self.citations(&membership.fact));
        }
        fam.children.extend(slots.into_iter().flatten());
        for membership in &family.children {
            let mut written = Structure::new("CHIL");
            written.payload = self.pointer(membership.person.raw());
            self.finish_side(&mut written, Entity::Fact(membership.fact.id), "FAM.CHIL");
            fam.children.push(written);
            if !self.citations(&membership.fact).is_empty() {
                self.report.push(Unwritten::ChildCitation {
                    family: family.id,
                    person: membership.person,
                });
            }
        }
        // What supports the partners' belonging supports the union: written once for the family.
        let mut seen = BTreeSet::new();
        for (citation, confidence) in cited {
            if seen.insert(citation)
                && let Some(written) = self.citation(citation, confidence)
            {
                fam.children.push(written);
            }
        }
        let archive = self.archive;
        for event in &family.events {
            if let Some(event) = archive.event(*event) {
                fam.children.push(self.event(event));
            }
        }
        self.finish(&mut fam, Entity::Family(family.id));
        fam
    }

    fn source(&mut self, id: SourceId) -> Structure {
        let mut sour = Structure::new("SOUR");
        sour.xref = self.xref(id.raw());
        if let Some(source) = self.archive.source(id) {
            let fields = [
                ("TITL", Some(&source.title)),
                ("AUTH", source.author.as_ref()),
                ("PUBL", source.publication.as_ref()),
            ];
            for (tag, text) in fields {
                if let Some(text) = text.filter(|text| !text.is_empty()) {
                    sour.children.push(Structure::new(tag).with_text(text));
                }
            }
            if source.repository.is_some() || source.call_number.is_some() {
                let mut repo = Structure::new("REPO");
                repo.payload = source
                    .repository
                    .map_or(Payload::Void, |repository| self.pointer(repository.raw()));
                if let Some(number) = &source.call_number {
                    let written = match number {
                        CallNumber::Free(text) => text.clone(),
                        archival @ CallNumber::Archival { .. } => archival.written(),
                    };
                    repo.children
                        .push(Structure::new("CALN").with_text(&written));
                }
                sour.children.push(repo);
            }
        }
        self.finish(&mut sour, Entity::Source(id));
        sour
    }

    fn repository(&mut self, id: drevigen_core::RepositoryId) -> Structure {
        let mut repo = Structure::new("REPO");
        repo.xref = self.xref(id.raw());
        if let Some(repository) = self.archive.repository(id) {
            repo.children
                .push(Structure::new("NAME").with_text(&repository.name));
            if let Some(address) = &repository.address {
                repo.children
                    .push(Structure::new("ADDR").with_text(address));
            }
        }
        self.finish(&mut repo, Entity::Repository(id));
        repo
    }

    // ── events and places ───────────────────────────────────────────────────────────────────

    fn event(&mut self, event: &Event) -> Structure {
        let description = event
            .description
            .as_ref()
            .and_then(|fact| Some((fact, self.value(fact)?)));
        let mut written = match event_tag(event.kind) {
            Ok(tag) => {
                let mut written = Structure::new(tag);
                // GEDCOM 7 gives a specific event no words of its own. Its `TYPE` "further
                // classifies" it, and is to be shown wherever the event is.
                if let Some((_, text)) = &description {
                    written
                        .children
                        .push(Structure::new("TYPE").with_text(text));
                }
                written
            }
            // GEDCOM's generic event, which must say what it is, and is described by its value.
            Err(kind) => {
                let classified = match event.kind {
                    EventKind::Other => event.label.as_deref().unwrap_or(kind),
                    _ => kind,
                };
                let mut written =
                    Structure::new("EVEN").with_child(Structure::new("TYPE").with_text(classified));
                if let Some((_, text)) = &description {
                    written.payload = Payload::Text(text.clone());
                }
                written
            }
        };
        let mut citations: Vec<(CitationId, Option<Confidence>)> = Vec::new();
        if let Some((fact, _)) = description {
            citations.extend(self.citations(fact));
            self.finish(&mut written, Entity::Fact(fact.id));
        }
        if let Some(date) = &event.date
            && let Some(value) = self.value(date)
        {
            let mut written_date = date_structure(&value);
            self.finish(&mut written_date, Entity::Fact(date.id));
            written.children.push(written_date);
            citations.extend(self.citations(date));
        }
        if let Some(place) = &event.place
            && let Some(value) = self.value(place)
        {
            let mut written_place = self.place_structure(value);
            self.finish(&mut written_place, Entity::Fact(place.id));
            written.children.push(written_place);
            citations.extend(self.citations(place));
        }
        for participation in &event.participants {
            citations.extend(self.citations(&participation.role));
            let role = peek(&participation.role);
            let family_role = matches!(role, Some(Role::Husband | Role::Wife | Role::Spouse));
            if role != Some(Role::Principal) && !family_role {
                self.report.push(Unwritten::Role {
                    event: event.id,
                    person: participation.person,
                });
            }
        }
        // The event's citations, once each: GEDCOM cites an event as a whole.
        let mut seen = BTreeSet::new();
        for (citation, confidence) in citations {
            if seen.insert(citation)
                && let Some(cited) = self.citation(citation, confidence)
            {
                written.children.push(cited);
            }
        }
        self.finish(&mut written, Entity::Event(event.id));
        // GEDCOM 7 reads an event as having happened only if it has a date, a place or a value,
        // and the archive holds events that happened.
        let occurred = written
            .children
            .iter()
            .any(|child| matches!(child.tag.as_str(), "DATE" | "PLAC"));
        if !occurred && written.tag != "EVEN" {
            written.payload = Payload::Text("Y".to_owned());
        }
        written
    }

    /// A `PLAC`: the place and every place it lies within, smallest first, and its coordinates.
    fn place_structure(&self, id: PlaceId) -> Structure {
        let archive = self.archive;
        let mut names = Vec::new();
        let mut visited = BTreeSet::new();
        let mut current = Some(id);
        while let Some(place) = current.and_then(|id| archive.place(id)) {
            if !visited.insert(place.id) {
                break;
            }
            if let Some(name) = place.names.first() {
                names.push(name.name.clone());
            }
            current = place.within.first().map(|parent| parent.place);
        }
        let mut written = Structure::new("PLAC").with_text(&names.join(", "));
        if let Some(position) = archive.place(id).and_then(|place| place.position) {
            let (latitude, longitude) = position.degrees();
            written.children.push(
                Structure::new("MAP")
                    .with_child(Structure::new("LATI").with_text(&degrees(latitude, 'N', 'S')))
                    .with_child(Structure::new("LONG").with_text(&degrees(longitude, 'E', 'W'))),
            );
        }
        written
    }

    // ── claims and citations ────────────────────────────────────────────────────────────────

    /// What a fact comes to. A contested fact is written with its provisional value, and said so.
    fn value<V: Claimable + Clone>(&mut self, fact: &Fact<V>) -> Option<V> {
        match fact.resolve() {
            Resolution::Unknown => None,
            Resolution::Concluded { conclusion, .. } => Some(conclusion.value.clone()),
            Resolution::Agreed { assertion, .. } => Some(assertion.value.clone()),
            Resolution::Contested { provisional, .. } => {
                self.report.push(Unwritten::Contested { fact: fact.id });
                Some(provisional.value.clone())
            }
        }
    }

    /// The citations of a fact's live assertions, each once, without those of provenance.
    fn citations<V: Claimable>(&self, fact: &Fact<V>) -> Vec<(CitationId, Option<Confidence>)> {
        let mut seen = BTreeSet::new();
        fact.claims
            .assertions()
            .iter()
            .filter(|assertion| assertion.retracted.is_none())
            .filter_map(|assertion| Some((assertion.citation?, assertion.confidence)))
            .filter(|(citation, _)| {
                self.archive
                    .citation(*citation)
                    .is_some_and(|c| !self.options.provenance.contains(&c.source))
            })
            .filter(|(citation, _)| seen.insert(*citation))
            .collect()
    }

    /// The citations every one of these facts has, in the order the first has them.
    fn shared_citations<V: Claimable>(
        &self,
        facts: &[Fact<V>],
    ) -> Vec<(CitationId, Option<Confidence>)> {
        let Some((first, rest)) = facts.split_first() else {
            return Vec::new();
        };
        let others: Vec<BTreeSet<CitationId>> = rest
            .iter()
            .map(|fact| self.citations(fact).into_iter().map(|(id, _)| id).collect())
            .collect();
        self.citations(first)
            .into_iter()
            .filter(|(id, _)| others.iter().all(|cited| cited.contains(id)))
            .collect()
    }

    fn cite<V: Claimable>(&mut self, written: &mut Structure, fact: &Fact<V>) {
        for (citation, confidence) in self.citations(fact) {
            if let Some(cited) = self.citation(citation, confidence) {
                written.children.push(cited);
            }
        }
    }

    /// A `SOUR` citing a source: where in it, how far it is trusted, and what it says.
    fn citation(&mut self, id: CitationId, confidence: Option<Confidence>) -> Option<Structure> {
        let archive = self.archive;
        let citation: &Citation = archive.citation(id)?;
        let mut written = Structure::new("SOUR");
        written.payload = self.pointer(citation.source.raw());
        if let Some(locator) = &citation.locator {
            written
                .children
                .push(Structure::new("PAGE").with_text(&locator.written()));
        }
        if let Some(text) = &citation.transcription {
            written
                .children
                .push(Structure::new("DATA").with_child(Structure::new("TEXT").with_text(text)));
        }
        if let Some(confidence) = confidence {
            written
                .children
                .push(Structure::new("QUAY").with_text(&confidence.quay().to_string()));
        }
        self.finish(&mut written, Entity::Citation(id));
        Some(written)
    }

    // ── notes and what was kept ─────────────────────────────────────────────────────────────

    /// Puts beneath a written structure the notes annotating its subject and the foreign data
    /// kept under it.
    fn finish(&mut self, written: &mut Structure, subject: Entity) {
        self.finish_where(written, subject, |_| true);
    }

    /// The same, for one side of a family link: a membership's foreign data was kept either from
    /// the person's `FAMC` or `FAMS`, or from the family's `CHIL`, `HUSB` or `WIFE`, and goes back
    /// to the side it came from. Notes go with the person's side, which GEDCOM gives a place for.
    fn finish_side(&mut self, written: &mut Structure, subject: Entity, side: &str) {
        let person_side = side.starts_with("INDI");
        if person_side {
            self.notes(written, subject);
        }
        let side = side.to_owned();
        self.foreign_where(written, subject, move |item| {
            if person_side {
                !item.path.starts_with("FAM.")
            } else {
                item.path.starts_with(&side)
            }
        });
    }

    fn finish_where(
        &mut self,
        written: &mut Structure,
        subject: Entity,
        keep: impl Fn(&Foreign) -> bool,
    ) {
        self.notes(written, subject);
        self.foreign_where(written, subject, keep);
    }

    fn notes(&self, written: &mut Structure, subject: Entity) {
        for note in self.archive.notes_on(subject) {
            if self.inline.contains(&note.id) {
                written
                    .children
                    .push(Structure::new("NOTE").with_text(&note.text));
            } else {
                let mut shared = Structure::new("SNOTE");
                shared.payload = self.pointer(note.id.raw());
                written.children.push(shared);
            }
        }
    }

    fn foreign_where(
        &mut self,
        written: &mut Structure,
        subject: Entity,
        keep: impl Fn(&Foreign) -> bool,
    ) {
        let archive = self.archive;
        let generated = written.children.len();
        for item in archive.foreign_on(subject) {
            if item.is_record() || !keep(item) {
                continue;
            }
            if let Some(kept) = self.read(item) {
                place_into(written, kept, item.remainder, generated);
            }
        }
    }

    /// Reads a foreign item back into a structure, its pointers rewritten to what this export
    /// calls the records they lead to — or to `@VOID@`, where the record is not written.
    fn read(&mut self, item: &Foreign) -> Option<Structure> {
        let Ok(mut structure) = Structure::read(&item.text) else {
            self.report.push(Unwritten::Unreadable { foreign: item.id });
            return None;
        };
        let mut voided = false;
        self.repoint(&mut structure, &mut voided);
        if item.is_record() {
            structure.xref = self.xref(item.id.raw());
        }
        if voided {
            self.report.push(Unwritten::Voided { foreign: item.id });
        }
        Some(structure)
    }

    fn repoint(&self, structure: &mut Structure, voided: &mut bool) {
        if let Payload::Pointer(target) = &structure.payload {
            let raw = u128::from_str_radix(target, 16).ok();
            let xref = raw.and_then(|raw| self.xrefs.get(&raw)).cloned();
            *voided |= xref.is_none();
            structure.payload = xref.map_or(Payload::Void, Payload::Pointer);
        }
        for child in &mut structure.children {
            self.repoint(child, voided);
        }
    }
}

/// What a fact comes to, for deciding how to write something else: no disagreement is reported
/// from here, only from the values actually written.
fn peek<V: Claimable + Clone>(fact: &Fact<V>) -> Option<V> {
    match fact.resolve() {
        Resolution::Unknown => None,
        Resolution::Concluded { conclusion, .. } => Some(conclusion.value.clone()),
        Resolution::Agreed { assertion, .. } => Some(assertion.value.clone()),
        Resolution::Contested { provisional, .. } => Some(provisional.value.clone()),
    }
}

/// Puts a kept structure beneath a written one. A remainder — what was left of a structure the
/// import took apart — goes back into the structure written from the archive in its place, the
/// one among the first `generated` with the same tag and value. A whole kept structure goes in
/// beside it, however alike they look: a second `FAMC` to the same family is a second link.
fn place_into(written: &mut Structure, kept: Structure, remainder: bool, generated: usize) {
    if remainder {
        let generated = generated.min(written.children.len());
        let twin = written.children[..generated]
            .iter_mut()
            .find(|child| child.tag == kept.tag && child.payload == kept.payload);
        if let Some(twin) = twin {
            twin.children.extend(kept.children);
            return;
        }
    }
    written.children.push(kept);
}

fn name_structure(name: &PersonName) -> Structure {
    let words = |parts: &[&Option<String>]| -> String {
        parts
            .iter()
            .filter_map(|part| part.as_deref())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let before = words(&[&name.prefix, &name.given, &name.patronymic]);
    let surname = words(&[&name.surname_prefix, &name.surname]);
    let after = words(&[&name.suffix]);
    let mut value = before;
    if !surname.is_empty() {
        if !value.is_empty() {
            value.push(' ');
        }
        value.push('/');
        value.push_str(&surname);
        value.push('/');
    }
    if !after.is_empty() {
        if !value.is_empty() {
            value.push(' ');
        }
        value.push_str(&after);
    }
    let mut written = Structure::new("NAME").with_text(&value);
    // GEDCOM has no patronymic: it goes with the given name, where Russian programs put it and
    // where an import finds it again.
    let given = words(&[&name.given, &name.patronymic]);
    let parts: [(&str, Option<&str>); 6] = [
        ("NPFX", name.prefix.as_deref()),
        ("GIVN", (!given.is_empty()).then_some(given.as_str())),
        ("NICK", name.nickname.as_deref()),
        ("SPFX", name.surname_prefix.as_deref()),
        ("SURN", name.surname.as_deref()),
        ("NSFX", name.suffix.as_deref()),
    ];
    for (tag, part) in parts {
        if let Some(part) = part {
            written.children.push(Structure::new(tag).with_text(part));
        }
    }
    if let Some(kind) = name.kind {
        written
            .children
            .push(Structure::new("TYPE").with_text(kind.tag()));
    }
    written
}

fn date_structure(date: &drevigen_date::RecordedDate) -> Structure {
    let (value, phrase) = date.to_gedcom7();
    let mut written = Structure::new("DATE");
    if !value.is_empty() {
        written.payload = Payload::Text(value);
    }
    if let Some(phrase) = phrase {
        written
            .children
            .push(Structure::new("PHRASE").with_text(&phrase));
    }
    written
}

/// What a generic `EVEN` is classified as for military service, which GEDCOM has no tag for. An
/// import reads it back as that kind.
pub(crate) const MILITARY_SERVICE: &str = "Military service";

/// What a generic `EVEN` is classified as when nothing says more. An import reads it back as an
/// event with no label.
pub(crate) const AN_EVENT: &str = "Event";

/// The tag GEDCOM writes an event of this kind with, or — for the kinds it has none for — the
/// words for the generic `EVEN`'s `TYPE`.
const fn event_tag(kind: EventKind) -> Result<&'static str, &'static str> {
    Ok(match kind {
        EventKind::Birth => "BIRT",
        EventKind::Christening => "CHR",
        EventKind::Baptism => "BAPM",
        EventKind::Death => "DEAT",
        EventKind::Burial => "BURI",
        EventKind::Cremation => "CREM",
        EventKind::Adoption => "ADOP",
        EventKind::Confirmation => "CONF",
        EventKind::Ordination => "ORDN",
        EventKind::Graduation => "GRAD",
        EventKind::Emigration => "EMIG",
        EventKind::Immigration => "IMMI",
        EventKind::Naturalisation => "NATU",
        EventKind::Census => "CENS",
        EventKind::Probate => "PROB",
        EventKind::Will => "WILL",
        EventKind::Retirement => "RETI",
        EventKind::Marriage => "MARR",
        EventKind::Engagement => "ENGA",
        EventKind::Banns => "MARB",
        EventKind::Divorce => "DIV",
        EventKind::Annulment => "ANUL",
        EventKind::Military => return Err(MILITARY_SERVICE),
        EventKind::Other => return Err(AN_EVENT),
    })
}

const fn attribute_tag(kind: AttributeKind) -> &'static str {
    match kind {
        AttributeKind::Occupation => "OCCU",
        AttributeKind::Estate => "CAST",
        AttributeKind::Religion => "RELI",
        AttributeKind::Residence => "RESI",
        AttributeKind::Nationality => "NATI",
        AttributeKind::Education => "EDUC",
        AttributeKind::Title => "TITL",
        AttributeKind::Property => "PROP",
        AttributeKind::Description => "DSCR",
        AttributeKind::Other => "FACT",
    }
}

const fn pedigree_tag(relation: ChildRelation) -> Option<&'static str> {
    match relation {
        ChildRelation::Birth => Some("BIRTH"),
        ChildRelation::Adopted => Some("ADOPTED"),
        ChildRelation::Foster => Some("FOSTER"),
        ChildRelation::Sealing => Some("SEALING"),
        ChildRelation::Other => Some("OTHER"),
        ChildRelation::Unspecified => None,
    }
}

/// A coordinate as GEDCOM writes it: a hemisphere letter and degrees, to the 10⁻⁷ the archive
/// keeps, without trailing zeros.
fn degrees(value: f64, positive: char, negative: char) -> String {
    let hemisphere = if value < 0.0 { negative } else { positive };
    let written = format!("{:.7}", value.abs());
    let written = written.trim_end_matches('0').trim_end_matches('.');
    format!("{hemisphere}{written}")
}

/// A timestamp's day as GEDCOM writes a date: `13 OCT 2026`.
fn gedcom_day(at: Timestamp) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ];
    let days = i32::try_from(at.0.div_euclid(86_400_000)).ok()?;
    let (year, month, day) = gregorian::from_day(gregorian::to_day(1970, 1, 1).offset(days));
    let month = MONTHS.get(usize::from(month).checked_sub(1)?)?;
    Some(format!("{day} {month} {year}"))
}
