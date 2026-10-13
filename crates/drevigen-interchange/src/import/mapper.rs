//! The structures of a document into the archive's entities and claims.
//!
//! Each kind of structure has a function that takes the substructures it understands and passes
//! every other one to [`Mapper::keep`], which stores it verbatim as foreign data under the nearest
//! subject. That is the whole of the losslessness guarantee: there is no third path.
//!
//! Some structures are taken *into* the subject above them rather than becoming one: a name's
//! `GIVN`, a person's `FAMC` and its `PEDI`, a source's `REPO` and its `CALN`. One of those is
//! taken only when everything beneath it is understood. When it carries something more, what is
//! kept is the structure's own line with only the substructures that were not understood beneath
//! it — so that an export, writing the structure from the archive, can find the rest by its line
//! and put it back where it was.
//!
//! The file's identifiers live only as long as the import. Every record gets a fresh identity,
//! and a pointer inside kept text is rewritten to the identity of what it pointed at, written as
//! 32 hexadecimal digits — itself a valid GEDCOM 7 identifier, so an export can write it as it
//! stands.

use std::collections::{BTreeMap, BTreeSet};

use drevigen_core::{
    Archive, Assertion, Attribute, AttributeKind, CallNumber, ChildRelation, Citation, CitationId,
    Claimable, Coordinates, Entity, Event, EventId, EventKind, Fact, Family, FamilyId, Foreign,
    ForeignId, Id, Kind, Locator, Membership, Note, NoteId, Participation, Partnership, Person,
    PersonId, Place, PlaceId, PlaceName, PlaceParent, Provenance, Refusal, Repository,
    RepositoryId, Resolution, Role, Sex, Source, SourceId, fold,
};
use drevigen_date::RecordedDate;
use drevigen_date::gedcom::Severity as DateSeverity;
use drevigen_gedcom::{Document, Payload, Structure};

use super::Options;
use super::names;
use super::report::{Mapping, Report};

/// Maps a document into a new archive. Returns the archive and the source the file became.
pub(super) fn map(
    document: &Document,
    options: &Options,
    entropy: impl FnMut() -> u128,
    report: &mut Report,
) -> (Archive, SourceId) {
    let mut mapper = Mapper::new(options, entropy, report);
    mapper.run(document);
    (mapper.archive, mapper.file)
}

/// What a record became, by the identifier the file gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Person(PersonId),
    Family(FamilyId),
    Source(SourceId),
    Repository(RepositoryId),
    Note(NoteId),
    /// A record the archive cannot hold, kept whole.
    Foreign(ForeignId),
}

impl Target {
    const fn raw(self) -> u128 {
        match self {
            Self::Person(id) => id.raw(),
            Self::Family(id) => id.raw(),
            Self::Source(id) => id.raw(),
            Self::Repository(id) => id.raw(),
            Self::Note(id) => id.raw(),
            Self::Foreign(id) => id.raw(),
        }
    }
}

/// Work that has to wait until its subject is in the archive.
enum Deferred {
    Annotate {
        subject: Entity,
        note: NoteId,
        line: usize,
    },
    Keep {
        item: Foreign,
        line: usize,
    },
}

struct Mapper<'d, 'r, E> {
    archive: Archive,
    report: &'r mut Report,
    entropy: E,
    millis: u64,
    made: Provenance,
    file_name: String,
    /// `GEDCOM 5.5.1`: what foreign data is labelled as.
    format: String,
    /// The source the file became.
    file: SourceId,
    targets: BTreeMap<&'d str, Target>,
    records: BTreeMap<&'d str, &'d Structure>,
    /// Identifiers whose record has been mapped, so that a second record with one gets an
    /// identity of its own instead of colliding with the first.
    taken: BTreeSet<&'d str>,
    /// The `FAMC` and `FAMS` structures of person records, by person and family: the person's
    /// side of a link the family records from its side.
    as_child: BTreeMap<(PersonId, FamilyId), &'d Structure>,
    as_partner: BTreeMap<(PersonId, FamilyId), &'d Structure>,
    /// The file's citation of each record, by the record's line, made when first needed.
    citations: BTreeMap<usize, CitationId>,
    /// Every place made so far, by its jurisdictions from the largest down, folded.
    places: BTreeMap<String, PlaceId>,
    /// The coordinates the file gives for places, by the same key.
    positions: BTreeMap<String, Coordinates>,
    deferred: Vec<Deferred>,
}

impl<'d, 'r, E: FnMut() -> u128> Mapper<'d, 'r, E> {
    fn new(options: &Options, mut entropy: E, report: &'r mut Report) -> Self {
        let millis = u64::try_from(options.at.0).unwrap_or(0);
        let file = Id::mint(millis, entropy());
        Self {
            archive: Archive::new(),
            report,
            entropy,
            millis,
            made: Provenance {
                by: options.by,
                at: options.at,
            },
            file_name: options.file_name.clone(),
            format: "GEDCOM".to_owned(),
            file,
            targets: BTreeMap::new(),
            records: BTreeMap::new(),
            taken: BTreeSet::new(),
            as_child: BTreeMap::new(),
            as_partner: BTreeMap::new(),
            citations: BTreeMap::new(),
            places: BTreeMap::new(),
            positions: BTreeMap::new(),
            deferred: Vec::new(),
        }
    }

    fn mint<K: Kind>(&mut self) -> Id<K> {
        Id::mint(self.millis, (self.entropy)())
    }

    fn run(&mut self, document: &'d Document) {
        for record in &document.records {
            if let Some(xref) = record.xref.as_deref() {
                let target = self.new_target(&record.tag);
                self.targets.entry(xref).or_insert(target);
                self.records.entry(xref).or_insert(record);
            }
        }

        for record in &document.records {
            self.collect_positions(record);
        }
        let head = document
            .records
            .first()
            .filter(|record| record.tag == "HEAD");
        self.describe_file(head);
        let mut rest: Vec<&'d Structure> = document
            .records
            .iter()
            .skip(usize::from(head.is_some()))
            .collect();

        // Each pass needs what the one before it made: sources their repositories, people their
        // sources and notes, families their people.
        self.pass(&mut rest, &["REPO"], Self::repository);
        self.pass(&mut rest, &["SOUR"], Self::source);
        self.pass(&mut rest, &["NOTE", "SNOTE"], Self::shared_note);
        self.pass(&mut rest, &["INDI"], Self::person);
        self.pass(&mut rest, &["FAM"], Self::family);
        for record in rest {
            self.keep_record(record);
        }
        self.flush();
    }

    /// Maps the records with these tags, and leaves the rest for the passes after.
    fn pass(
        &mut self,
        records: &mut Vec<&'d Structure>,
        tags: &[&str],
        map: fn(&mut Self, &'d Structure),
    ) {
        records.retain(|record| {
            let matches = tags.contains(&record.tag.as_str());
            if matches {
                map(self, record);
            }
            !matches
        });
    }

    fn new_target(&mut self, tag: &str) -> Target {
        match tag {
            "INDI" => Target::Person(self.mint()),
            "FAM" => Target::Family(self.mint()),
            "SOUR" => Target::Source(self.mint()),
            "REPO" => Target::Repository(self.mint()),
            "NOTE" | "SNOTE" => Target::Note(self.mint()),
            _ => Target::Foreign(self.mint()),
        }
    }

    /// The identity a record maps to: the one its identifier was given, the first time; a fresh
    /// one for a record without an identifier, or with one an earlier record already has.
    fn identity(&mut self, record: &'d Structure) -> Target {
        if let Some(xref) = record.xref.as_deref() {
            if self.taken.insert(xref)
                && let Some(target) = self.targets.get(xref)
            {
                return *target;
            }
            self.report.note(
                record.line,
                Mapping::SecondRecord {
                    xref: xref.to_owned(),
                },
            );
        }
        self.new_target(&record.tag)
    }

    fn pointer(&self, structure: &Structure) -> Option<Target> {
        structure
            .payload
            .pointer()
            .and_then(|xref| self.targets.get(xref))
            .copied()
    }

    // ── records ─────────────────────────────────────────────────────────────────────────────

    /// The file's own source, described from the header: the program and the person that wrote
    /// it, and when. The header itself is kept whole beside it.
    fn describe_file(&mut self, head: Option<&'d Structure>) {
        let (mut author, mut publication) = (None, None);
        if let Some(head) = head {
            let version = head
                .child("GEDC")
                .and_then(|gedc| gedc.child("VERS"))
                .and_then(text_of);
            if let Some(version) = &version {
                self.format = format!("GEDCOM {version}");
            }
            let system = head.child("SOUR");
            let product = system
                .and_then(|s| s.child("NAME"))
                .and_then(text_of)
                .or_else(|| system.and_then(text_of));
            let program = match (
                product,
                system.and_then(|s| s.child("VERS")).and_then(text_of),
            ) {
                (Some(name), Some(release)) => Some(format!("{name} {release}")),
                (name, _) => name,
            };
            author = head
                .child("SUBM")
                .and_then(|subm| subm.payload.pointer())
                .and_then(|xref| self.records.get(xref))
                .and_then(|record| record.child("NAME"))
                .and_then(text_of);
            let written = [program.clone(), head.child("DATE").and_then(text_of)];
            let written: Vec<String> = written.into_iter().flatten().collect();
            publication = (!written.is_empty()).then(|| written.join(", "));
            self.report.version = version;
            self.report.program = program;
        }
        let source = Source {
            id: self.file,
            title: self.file_name.clone(),
            author,
            publication,
            repository: None,
            call_number: None,
        };
        if self.add(1, |archive| archive.add_source(source)) {
            self.report.counts.sources += 1;
        }
        if let Some(head) = head {
            self.keep(Entity::Source(self.file), head, "");
        }
    }

    fn repository(&mut self, record: &'d Structure) {
        let Target::Repository(id) = self.identity(record) else {
            return self.keep_record(record);
        };
        let (mut name, mut address) = (None, None);
        let mut rest = Vec::new();
        for child in &record.children {
            match child.tag.as_str() {
                "NAME" if name.is_none() && child.children.is_empty() => name = text_of(child),
                // The address's parts repeat its lines; the lines are what is kept, the parts
                // only read when there are no lines.
                "ADDR" if address.is_none() && child.children.iter().all(is_address_part) => {
                    address = text_of(child).or_else(|| address_parts(child));
                }
                _ => rest.push(child),
            }
        }
        let repository = Repository {
            id,
            name: name.unwrap_or_default(),
            abbreviation: None,
            address,
        };
        if self.add(record.line, |archive| archive.add_repository(repository)) {
            self.report.counts.repositories += 1;
        }
        self.rest(Entity::Repository(id), rest, "REPO");
    }

    fn source(&mut self, record: &'d Structure) {
        let Target::Source(id) = self.identity(record) else {
            return self.keep_record(record);
        };
        let subject = Entity::Source(id);
        let (mut title, mut author, mut publication, mut abbreviation) = (None, None, None, None);
        let (mut repository, mut call_number) = (None, None);
        let mut rest = Vec::new();
        for child in &record.children {
            let bare = child.children.is_empty();
            match child.tag.as_str() {
                "TITL" if bare && title.is_none() => title = text_of(child),
                "AUTH" if bare && author.is_none() => author = text_of(child),
                "PUBL" if bare && publication.is_none() => publication = text_of(child),
                "ABBR" if bare && abbreviation.is_none() => abbreviation = Some(child),
                "REPO" if repository.is_none() && call_number.is_none() => {
                    let target = self.pointer(child);
                    if child.payload.pointer().is_some() && target.is_none() {
                        self.unresolved(subject, child, "SOUR");
                        continue;
                    }
                    if let Some(Target::Repository(found)) = target {
                        repository = Some(found);
                    }
                    let mut beyond = Vec::new();
                    for part in &child.children {
                        match part.tag.as_str() {
                            "CALN" if call_number.is_none() && part.children.is_empty() => {
                                call_number = text_of(part).map(CallNumber::Free);
                            }
                            _ => beyond.push(part),
                        }
                    }
                    if !beyond.is_empty() {
                        self.keep_partly(subject, child, beyond, "SOUR");
                    }
                }
                _ => rest.push(child),
            }
        }
        // A source needs a title to be shown. Without one its abbreviation serves; otherwise the
        // abbreviation is kept, because the model has nowhere else to hold it.
        let title = if let Some(title) = title {
            rest.extend(abbreviation);
            title
        } else {
            self.report.note(record.line, Mapping::UntitledSource);
            abbreviation.and_then(text_of).unwrap_or_default()
        };
        let source = Source {
            id,
            title,
            author,
            publication,
            repository,
            call_number,
        };
        if self.add(record.line, |archive| archive.add_source(source)) {
            self.report.counts.sources += 1;
        }
        self.rest(subject, rest, "SOUR");
    }

    fn shared_note(&mut self, record: &'d Structure) {
        let Target::Note(id) = self.identity(record) else {
            return self.keep_record(record);
        };
        let note = Note {
            id,
            text: record.payload.text().unwrap_or_default().to_owned(),
        };
        if self.add(record.line, |archive| archive.add_note(note)) {
            self.report.counts.notes += 1;
        }
        let rest = record.children.iter().collect();
        self.rest(Entity::Note(id), rest, &record.tag);
    }

    fn person(&mut self, record: &'d Structure) {
        let Target::Person(id) = self.identity(record) else {
            return self.keep_record(record);
        };
        let subject = Entity::Person(id);
        let mut person = Person::new(id);
        let mut events = Vec::new();
        for child in &record.children {
            let tag = child.tag.as_str();
            match tag {
                "NAME" => self.name(&mut person, record, child),
                "SEX" => self.sex(&mut person, record, child),
                "FAMC" => self.link(id, child, true),
                "FAMS" => self.link(id, child, false),
                "NOTE" | "SNOTE" => self.annotation(subject, child),
                _ => match (person_event(tag), attribute_kind(tag)) {
                    (Some(kind), _) if plain_event(child) => events.push((child, kind)),
                    (_, Some(kind))
                        if !matches!(child.payload, Payload::Pointer(_) | Payload::Void) =>
                    {
                        self.attribute(&mut person, record, child, kind);
                    }
                    _ => self.keep(subject, child, "INDI"),
                },
            }
        }
        if !self.add(record.line, |archive| archive.add_person(person)) {
            return;
        }
        self.report.counts.people += 1;
        // An event needs its participant in the archive, so it comes after the person.
        for (structure, kind) in events {
            let principal = vec![(id, Role::Principal)];
            if self
                .event(structure, kind, principal, record, "INDI")
                .is_none()
            {
                self.keep(subject, structure, "INDI");
            }
        }
    }

    fn family(&mut self, record: &'d Structure) {
        let Target::Family(id) = self.identity(record) else {
            return self.keep_record(record);
        };
        let subject = Entity::Family(id);
        let mut family = Family::new(id);
        let mut events = Vec::new();
        for child in &record.children {
            if let Some(kind) = family_event(&child.tag)
                && plain_event(child)
            {
                events.push((child, kind));
                continue;
            }
            let person = match self.pointer(child) {
                Some(Target::Person(person)) => Some(person),
                _ => None,
            };
            match (child.tag.as_str(), person) {
                ("HUSB" | "WIFE", Some(person)) => {
                    let partnership = if child.tag == "HUSB" {
                        Partnership::Husband
                    } else {
                        Partnership::Wife
                    };
                    let side = self.as_partner.remove(&(person, id));
                    self.partner(&mut family, person, partnership, record, Some(child), side);
                }
                ("CHIL", Some(person)) if family.children.iter().all(|m| m.person != person) => {
                    let side = self.as_child.remove(&(person, id));
                    self.child(&mut family, person, record, Some(child), side);
                }
                ("HUSB" | "WIFE" | "CHIL", None) => self.unresolved(subject, child, "FAM"),
                ("NOTE" | "SNOTE", _) => self.annotation(subject, child),
                _ => self.keep(subject, child, "FAM"),
            }
        }

        // Links that people's records make and this family's record does not: read as if the
        // family made them too, and said so.
        let one_sided = |links: &BTreeMap<(PersonId, FamilyId), &'d Structure>| -> Vec<_> {
            links
                .iter()
                .filter(|((_, family), _)| *family == id)
                .map(|(&key, &side)| (key, side))
                .collect()
        };
        for (key, side) in one_sided(&self.as_partner) {
            self.as_partner.remove(&key);
            self.one_sided(record, side);
            self.partner(
                &mut family,
                key.0,
                Partnership::Partner,
                record,
                None,
                Some(side),
            );
        }
        for (key, side) in one_sided(&self.as_child) {
            self.as_child.remove(&key);
            self.one_sided(record, side);
            self.child(&mut family, key.0, record, None, Some(side));
        }

        // A family's events are its partners' events, each in the role the family gives them.
        let partners: Vec<(PersonId, Role)> = family
            .partners
            .iter()
            .map(|membership| {
                let role = match membership.fact.resolve() {
                    Resolution::Agreed { assertion, .. } => match assertion.value {
                        Partnership::Husband => Role::Husband,
                        Partnership::Wife => Role::Wife,
                        Partnership::Partner => Role::Spouse,
                    },
                    _ => Role::Spouse,
                };
                (membership.person, role)
            })
            .collect();
        for (structure, kind) in events {
            match self.event(structure, kind, partners.clone(), record, "FAM") {
                Some(event) => family.events.push(event),
                None => self.keep(subject, structure, "FAM"),
            }
        }

        if self.add(record.line, |archive| archive.add_family(family)) {
            self.report.counts.families += 1;
        }
    }

    /// A record the archive cannot hold: kept whole under the file's source, under the identity
    /// its identifier was given, so that whatever points at it still does.
    fn keep_record(&mut self, record: &'d Structure) {
        let id = match self.identity(record) {
            Target::Foreign(id) => id,
            _ => self.mint(),
        };
        self.keep_as(id, Entity::Source(self.file), record, None, "");
    }

    // ── a person's facts ────────────────────────────────────────────────────────────────────

    fn name(&mut self, person: &mut Person, record: &'d Structure, name: &'d Structure) {
        let parsed = names::person_name(name);
        if parsed.is_empty() {
            self.report.note(name.line, Mapping::EmptyName);
            if !name.children.is_empty() {
                self.keep(Entity::Person(person.id), name, "INDI");
            }
            return;
        }
        let fact = self.fact(parsed, record);
        let subject = Entity::Fact(fact.id);
        person.names.push(fact);
        let rest = name
            .children
            .iter()
            .filter(|child| {
                let part = names::PARTS.contains(&child.tag.as_str()) && child.children.is_empty();
                let kind = child.tag == "TYPE" && names::kind(child).is_some();
                !(part || kind)
            })
            .collect();
        self.rest(subject, rest, "INDI.NAME");
    }

    fn sex(&mut self, person: &mut Person, record: &'d Structure, sex: &'d Structure) {
        let value = sex.payload.text().unwrap_or_default().trim();
        let Some(parsed) = Sex::from_tag(&value.to_ascii_uppercase()) else {
            self.report.note(
                sex.line,
                Mapping::UnknownSex {
                    value: value.to_owned(),
                },
            );
            return self.keep(Entity::Person(person.id), sex, "INDI");
        };
        let assertion = self.assertion(parsed, record);
        if person.sex.is_none() {
            person.sex = Some(Fact::new(self.mint()));
        }
        let Some(fact) = person.sex.as_mut() else {
            return;
        };
        // A second SEX line is a second claim about the same fact, with a fresh identity: it
        // cannot be refused.
        let _ = fact.claims.assert(assertion);
        let subject = Entity::Fact(fact.id);
        self.rest(subject, sex.children.iter().collect(), "INDI.SEX");
    }

    /// The person's side of a family link, held until the family is mapped.
    fn link(&mut self, person: PersonId, link: &'d Structure, as_child: bool) {
        let Some(Target::Family(family)) = self.pointer(link) else {
            return self.unresolved(Entity::Person(person), link, "INDI");
        };
        let links = if as_child {
            &mut self.as_child
        } else {
            &mut self.as_partner
        };
        if links.contains_key(&(person, family)) {
            // The same link twice: nothing more to read from it, and so it is kept.
            return self.keep(Entity::Person(person), link, "INDI");
        }
        links.insert((person, family), link);
    }

    // ── a family's members ──────────────────────────────────────────────────────────────────

    fn partner(
        &mut self,
        family: &mut Family,
        person: PersonId,
        partnership: Partnership,
        record: &'d Structure,
        written: Option<&'d Structure>,
        side: Option<&'d Structure>,
    ) {
        let fact = self.fact(partnership, record);
        let subject = Entity::Fact(fact.id);
        family.partners.push(Membership { person, fact });
        if let Some(written) = written {
            let path = format!("FAM.{}", written.tag);
            self.rest(subject, written.children.iter().collect(), &path);
        }
        if let Some(side) = side {
            self.rest(subject, side.children.iter().collect(), "INDI.FAMS");
        }
    }

    fn child(
        &mut self,
        family: &mut Family,
        person: PersonId,
        record: &'d Structure,
        written: Option<&'d Structure>,
        side: Option<&'d Structure>,
    ) {
        let pedigree = side.and_then(|side| side.child("PEDI"));
        let how = pedigree
            .and_then(relation)
            .unwrap_or(ChildRelation::Unspecified);
        let fact = self.fact(how, record);
        let subject = Entity::Fact(fact.id);
        family.children.push(Membership { person, fact });
        if let Some(written) = written {
            self.rest(subject, written.children.iter().collect(), "FAM.CHIL");
        }
        if let Some(side) = side {
            let rest = side
                .children
                .iter()
                .filter(|c| !(c.tag == "PEDI" && relation(c).is_some()))
                .collect();
            self.rest(subject, rest, "INDI.FAMC");
        }
    }

    // ── events and attributes ───────────────────────────────────────────────────────────────

    /// An event, with its date and place as facts and its participants in their roles. Added to
    /// the archive before anything beneath it is kept or annotated, so that a refusal leaves
    /// nothing behind; the caller then keeps the structure whole.
    fn event(
        &mut self,
        structure: &'d Structure,
        kind: EventKind,
        participants: Vec<(PersonId, Role)>,
        record: &'d Structure,
        parent: &str,
    ) -> Option<EventId> {
        let id: EventId = self.mint();
        let mut event = Event::new(id, kind);
        let mut rest = Vec::new();
        let mut beneath = Vec::new();
        for child in &structure.children {
            match child.tag.as_str() {
                "DATE" if event.date.is_none() => match self.date(child) {
                    Some((date, more)) => {
                        let fact = self.fact(date, record);
                        beneath.push((Entity::Fact(fact.id), more, "DATE"));
                        event.date = Some(fact);
                    }
                    None => rest.push(child),
                },
                "PLAC" if event.place.is_none() => match self.place(child) {
                    Some((place, more)) => {
                        let fact = self.fact(place, record);
                        beneath.push((Entity::Fact(fact.id), more, "PLAC"));
                        event.place = Some(fact);
                    }
                    None => rest.push(child),
                },
                _ => rest.push(child),
            }
        }
        for (person, role) in participants {
            let role = self.fact(role, record);
            event.participants.push(Participation { person, role });
        }
        if !self.add(structure.line, |archive| archive.add_event(event)) {
            return None;
        }
        self.report.counts.events += 1;
        let path = format!("{parent}.{}", structure.tag);
        self.rest(Entity::Event(id), rest, &path);
        for (subject, more, tag) in beneath {
            self.rest(subject, more, &format!("{path}.{tag}"));
        }
        Some(id)
    }

    /// An attribute: its value, with the date and place the file gives it, as one fact. A `FACT`
    /// or `IDNO` says what it is in its `TYPE`, which becomes the attribute's label.
    fn attribute(
        &mut self,
        person: &mut Person,
        record: &'d Structure,
        structure: &'d Structure,
        kind: AttributeKind,
    ) {
        let labelled = matches!(structure.tag.as_str(), "FACT" | "IDNO");
        let mut value = Attribute {
            kind,
            label: None,
            value: structure.payload.text().unwrap_or_default().to_owned(),
            date: None,
            place: None,
        };
        let mut rest = Vec::new();
        let mut beyond = Vec::new();
        for child in &structure.children {
            match child.tag.as_str() {
                "TYPE" if labelled && value.label.is_none() && child.children.is_empty() => {
                    value.label = text_of(child);
                }
                "DATE" if value.date.is_none() => match self.date(child) {
                    Some((date, more)) => {
                        value.date = Some(date);
                        beyond.push((child, more));
                    }
                    None => rest.push(child),
                },
                "PLAC" if value.place.is_none() => match self.place(child) {
                    Some((place, more)) => {
                        value.place = Some(place);
                        beyond.push((child, more));
                    }
                    None => rest.push(child),
                },
                _ => rest.push(child),
            }
        }
        let fact = self.fact(value, record);
        let subject = Entity::Fact(fact.id);
        person.attributes.push(fact);
        let path = format!("INDI.{}", structure.tag);
        self.rest(subject, rest, &path);
        // A date or place is part of the attribute's value, not a subject of its own, so what
        // it carries beyond that is kept on its own line beneath the attribute.
        for (written, more) in beyond {
            if !more.is_empty() {
                self.keep_partly(subject, written, more, &path);
            }
        }
    }

    /// A `DATE` read as 7.0 reads it, however the file wrote it, with a 7.0 `PHRASE` beside it
    /// taken as the date's own words. Returns what else the `DATE` carries. `None` for one with
    /// nothing in it.
    fn date(&mut self, date: &'d Structure) -> Option<(RecordedDate, Vec<&'d Structure>)> {
        let text = date.payload.text().unwrap_or_default();
        let phrase = date
            .child("PHRASE")
            .filter(|phrase| phrase.children.is_empty())
            .and_then(text_of);
        if text.trim().is_empty() && phrase.is_none() {
            return None;
        }
        let read = RecordedDate::parse_lenient(text);
        for repair in read.repairs {
            if repair.severity() >= DateSeverity::Interpretation {
                self.report.note(
                    date.line,
                    Mapping::Date {
                        text: text.to_owned(),
                        repair,
                    },
                );
            }
        }
        let mut recorded = read.date;
        let mut more = Vec::new();
        for child in &date.children {
            if child.tag == "PHRASE" && recorded.phrase.is_none() && phrase.is_some() {
                recorded.phrase.clone_from(&phrase);
            } else {
                more.push(child);
            }
        }
        Some((recorded, more))
    }

    /// The place a `PLAC` names, as a hierarchy: `с. Великое, Ярославский уезд, Ярославская
    /// губерния` is a village within an uyezd within a governorate, and every other village in
    /// the same uyezd shares the uyezd and the governorate. A new place takes its coordinates from
    /// `MAP`. Returns what else the `PLAC` carries.
    fn place(&mut self, place: &'d Structure) -> Option<(PlaceId, Vec<&'d Structure>)> {
        let parts = jurisdictions(place)?;
        let mut key = String::new();
        let mut within: Option<PlaceId> = None;
        for part in parts.iter().rev() {
            key.push('|');
            key.push_str(&fold(part));
            if let Some(found) = self.places.get(&key) {
                within = Some(*found);
                continue;
            }
            let id: PlaceId = self.mint();
            let position = self.positions.get(&key).copied();
            let new = Place {
                id,
                kind: None,
                names: vec![PlaceName {
                    name: (*part).to_owned(),
                    language: None,
                    valid: None,
                }],
                within: within
                    .map(|place| PlaceParent { place, valid: None })
                    .into_iter()
                    .collect(),
                position,
            };
            if self.add(place.line, |archive| archive.add_place(new)) {
                self.report.counts.places += 1;
            }
            self.places.insert(key.clone(), id);
            within = Some(id);
        }
        // A MAP that agrees with the place's position is the position; one that does not is
        // another claim about where the place is, and is kept.
        let agrees =
            coordinates(place).is_some() && coordinates(place) == self.positions.get(&key).copied();
        let more = place
            .children
            .iter()
            .filter(|child| !(agrees && child.tag == "MAP"))
            .collect();
        Some((within?, more))
    }

    /// The first coordinates the file gives for each place, wherever it gives them, so that a
    /// place has them however it was first mentioned.
    fn collect_positions(&mut self, structure: &Structure) {
        if structure.tag == "PLAC"
            && let (Some(parts), Some(position)) =
                (jurisdictions(structure), coordinates(structure))
        {
            let mut key = String::new();
            for part in parts.iter().rev() {
                key.push('|');
                key.push_str(&fold(part));
            }
            self.positions.entry(key).or_insert(position);
        }
        for child in &structure.children {
            self.collect_positions(child);
        }
    }

    fn one_sided(&mut self, record: &Structure, side: &Structure) {
        self.report.note(
            side.line,
            Mapping::OneSidedLink {
                family: record.xref.clone().unwrap_or_default(),
            },
        );
    }

    // ── claims ──────────────────────────────────────────────────────────────────────────────

    /// A fact with one assertion of a value, cited by the file's citation of its record.
    fn fact<V: Claimable>(&mut self, value: V, record: &'d Structure) -> Fact<V> {
        let assertion = self.assertion(value, record);
        let mut fact = Fact::new(self.mint());
        // A fresh fact and a fresh assertion: nothing to refuse.
        let _ = fact.claims.assert(assertion);
        fact
    }

    fn assertion<V>(&mut self, value: V, record: &'d Structure) -> Assertion<V> {
        let citation = self.citation(record);
        Assertion {
            citation: Some(citation),
            ..Assertion::new(self.mint(), value, self.made)
        }
    }

    /// The file's citation of a record, made the first time a value from the record needs it:
    /// *the tree says so, in record @I12@*.
    fn citation(&mut self, record: &'d Structure) -> CitationId {
        if let Some(citation) = self.citations.get(&record.line) {
            return *citation;
        }
        let id = self.mint();
        let citation = Citation {
            id,
            source: self.file,
            locator: record
                .xref
                .as_ref()
                .map(|xref| Locator::Free(format!("@{xref}@"))),
            transcription: None,
            media: Vec::new(),
        };
        if self.add(record.line, |archive| archive.add_citation(citation)) {
            self.report.counts.citations += 1;
        }
        self.citations.insert(record.line, id);
        id
    }

    // ── notes, and what is kept ─────────────────────────────────────────────────────────────

    /// A `NOTE` or `SNOTE` under a subject: the shared note it points at, or an inline one, which
    /// becomes a note of its own.
    fn annotation(&mut self, subject: Entity, note: &'d Structure) {
        let line = note.line;
        if note.payload.pointer().is_some() {
            let Some(Target::Note(id)) = self.pointer(note) else {
                return self.unresolved(subject, note, "");
            };
            self.deferred.push(Deferred::Annotate {
                subject,
                note: id,
                line,
            });
            if !note.children.is_empty() {
                let beyond = note.children.iter().collect();
                self.keep_partly(subject, note, beyond, "");
            }
            return;
        }
        let Some(text) = note.payload.text() else {
            if !note.children.is_empty() {
                self.keep(subject, note, "");
            }
            return;
        };
        let id: NoteId = self.mint();
        let added = self.add(line, |archive| {
            archive.add_note(Note {
                id,
                text: text.to_owned(),
            })
        });
        if added {
            self.report.counts.notes += 1;
            self.deferred.push(Deferred::Annotate {
                subject,
                note: id,
                line,
            });
            self.rest(Entity::Note(id), note.children.iter().collect(), "NOTE");
        }
    }

    /// A structure whose pointer leads nowhere it can: reported, and kept.
    fn unresolved(&mut self, subject: Entity, structure: &'d Structure, parent: &str) {
        let xref = match &structure.payload {
            Payload::Pointer(xref) => xref.clone(),
            _ => String::new(),
        };
        self.report
            .note(structure.line, Mapping::Unresolved { xref });
        self.keep(subject, structure, parent);
    }

    /// The substructures a mapping did not take: notes annotate, everything else is kept.
    fn rest(&mut self, subject: Entity, rest: Vec<&'d Structure>, parent: &str) {
        for structure in rest {
            match structure.tag.as_str() {
                "NOTE" | "SNOTE" => self.annotation(subject, structure),
                _ => self.keep(subject, structure, parent),
            }
        }
    }

    /// Keeps a structure verbatim under a subject (ADR 0011).
    fn keep(&mut self, subject: Entity, structure: &'d Structure, parent: &str) {
        let id = self.mint();
        self.keep_as(id, subject, structure, None, parent);
    }

    /// Keeps a structure that was mapped with only the substructures the mapping did not take.
    fn keep_partly(
        &mut self,
        subject: Entity,
        structure: &'d Structure,
        beyond: Vec<&'d Structure>,
        parent: &str,
    ) {
        let id = self.mint();
        self.keep_as(id, subject, structure, Some(beyond), parent);
    }

    fn keep_as(
        &mut self,
        id: ForeignId,
        subject: Entity,
        structure: &Structure,
        only: Option<Vec<&Structure>>,
        parent: &str,
    ) {
        let mut kept = self.rewritten(structure);
        if let Some(only) = only {
            kept.children = only
                .into_iter()
                .map(|child| self.rewritten(child))
                .collect();
        }
        if structure.xref.is_some() {
            kept.xref = Some(hex(id.raw()));
        }
        let path = if parent.is_empty() {
            structure.tag.clone()
        } else {
            format!("{parent}.{}", structure.tag)
        };
        *self.report.kept.entry(path).or_default() += 1;
        self.deferred.push(Deferred::Keep {
            item: Foreign {
                id,
                subject,
                format: self.format.clone(),
                text: kept.write(),
            },
            line: structure.line,
        });
    }

    /// The structure with every pointer rewritten to the identity of what it points at.
    fn rewritten(&self, structure: &Structure) -> Structure {
        let payload = match &structure.payload {
            Payload::Pointer(xref) => match self.targets.get(xref.as_str()) {
                Some(target) => Payload::Pointer(hex(target.raw())),
                None => structure.payload.clone(),
            },
            other => other.clone(),
        };
        Structure {
            tag: structure.tag.clone(),
            xref: structure.xref.clone(),
            payload,
            children: structure
                .children
                .iter()
                .map(|child| self.rewritten(child))
                .collect(),
            line: structure.line,
        }
    }

    fn flush(&mut self) {
        for deferred in core::mem::take(&mut self.deferred) {
            match deferred {
                Deferred::Annotate {
                    subject,
                    note,
                    line,
                } => {
                    self.add(line, |archive| archive.annotate(subject, note));
                }
                Deferred::Keep { item, line } => {
                    if self.add(line, |archive| archive.add_foreign(item)) {
                        self.report.counts.foreign += 1;
                    }
                }
            }
        }
    }

    /// Makes an edit, reporting a refusal instead of failing: an import never fails on what it
    /// was given, and a refusal here is a defect in the importer that must be visible.
    fn add(&mut self, line: usize, edit: impl FnOnce(&mut Archive) -> Result<(), Refusal>) -> bool {
        match edit(&mut self.archive) {
            Ok(()) => true,
            Err(refusal) => {
                self.report.note(
                    line,
                    Mapping::Refused {
                        refusal: refusal.to_string(),
                    },
                );
                false
            }
        }
    }
}

/// An identity as a GEDCOM 7 cross-reference identifier: 32 hexadecimal digits.
fn hex(raw: u128) -> String {
    format!("{raw:032X}")
}

fn text_of(structure: &Structure) -> Option<String> {
    structure
        .payload
        .text()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

const ADDRESS_PARTS: [&str; 7] = ["ADR1", "ADR2", "ADR3", "CITY", "STAE", "POST", "CTRY"];

fn is_address_part(structure: &Structure) -> bool {
    ADDRESS_PARTS.contains(&structure.tag.as_str()) && structure.children.is_empty()
}

/// An address from its parts, for a file that gives the parts and no lines.
fn address_parts(address: &Structure) -> Option<String> {
    let parts: Vec<String> = ADDRESS_PARTS
        .iter()
        .filter_map(|tag| address.child(tag).and_then(text_of))
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// The relation a `PEDI` names, when it names one of GEDCOM's and carries nothing more. 7.0's
/// `OTHER`, which comes with a `PHRASE`, says more than the model holds, and is kept.
fn relation(pedigree: &Structure) -> Option<ChildRelation> {
    if !pedigree.children.is_empty() {
        return None;
    }
    match pedigree
        .payload
        .text()?
        .trim()
        .to_ascii_uppercase()
        .as_str()
    {
        "BIRTH" => Some(ChildRelation::Birth),
        "ADOPTED" => Some(ChildRelation::Adopted),
        "FOSTER" => Some(ChildRelation::Foster),
        "SEALING" => Some(ChildRelation::Sealing),
        _ => None,
    }
}

/// The kind of a person's event GEDCOM writes with this tag, where the model has the same kind.
/// `EVEN` and the kinds the model does not distinguish — `BARM`, `FCOM` and the rest — are kept,
/// because `Other` would lose which one it was.
fn person_event(tag: &str) -> Option<EventKind> {
    Some(match tag {
        "BIRT" => EventKind::Birth,
        "CHR" => EventKind::Christening,
        "BAPM" => EventKind::Baptism,
        "DEAT" => EventKind::Death,
        "BURI" => EventKind::Burial,
        "CREM" => EventKind::Cremation,
        "ADOP" => EventKind::Adoption,
        "CONF" => EventKind::Confirmation,
        "ORDN" => EventKind::Ordination,
        "GRAD" => EventKind::Graduation,
        "EMIG" => EventKind::Emigration,
        "IMMI" => EventKind::Immigration,
        "NATU" => EventKind::Naturalisation,
        "CENS" => EventKind::Census,
        "PROB" => EventKind::Probate,
        "WILL" => EventKind::Will,
        "RETI" => EventKind::Retirement,
        _ => return None,
    })
}

/// The kind of a family's event, on the same terms.
fn family_event(tag: &str) -> Option<EventKind> {
    Some(match tag {
        "MARR" => EventKind::Marriage,
        "ENGA" => EventKind::Engagement,
        "MARB" => EventKind::Banns,
        "DIV" => EventKind::Divorce,
        "ANUL" => EventKind::Annulment,
        "CENS" => EventKind::Census,
        _ => return None,
    })
}

/// The kind of attribute GEDCOM writes with this tag. `CAST`, *caste*, is the closest GEDCOM
/// comes to a Russian *сословие*, and is read as one.
fn attribute_kind(tag: &str) -> Option<AttributeKind> {
    Some(match tag {
        "OCCU" => AttributeKind::Occupation,
        "CAST" => AttributeKind::Estate,
        "RELI" => AttributeKind::Religion,
        "RESI" => AttributeKind::Residence,
        "NATI" => AttributeKind::Nationality,
        "EDUC" => AttributeKind::Education,
        "TITL" => AttributeKind::Title,
        "PROP" => AttributeKind::Property,
        "DSCR" => AttributeKind::Description,
        "FACT" | "IDNO" => AttributeKind::Other,
        _ => return None,
    })
}

/// A `PLAC`'s jurisdictions as written, smallest first; empty ones, which only hold a position
/// in a 5.5.1 `FORM`, left out. `None` for a place with no name.
fn jurisdictions(place: &Structure) -> Option<Vec<&str>> {
    let parts: Vec<&str> = place
        .payload
        .text()?
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    (!parts.is_empty()).then_some(parts)
}

/// Whether an event's value is one the model can hold: nothing, or `Y` — "it happened, and
/// nothing more is known". Anything else is words the event structure has no place for.
fn plain_event(event: &Structure) -> bool {
    match &event.payload {
        Payload::None => true,
        Payload::Text(text) => text.trim().eq_ignore_ascii_case("Y"),
        Payload::Pointer(_) | Payload::Void => false,
    }
}

/// Coordinates from a `PLAC.MAP`: `N55.7558`, `E37.6173`, or plain signed degrees. `None` unless
/// both are there and readable, and the `MAP` carries nothing else.
fn coordinates(place: &Structure) -> Option<Coordinates> {
    let map = place.child("MAP")?;
    let bare = map
        .children
        .iter()
        .all(|c| matches!(c.tag.as_str(), "LATI" | "LONG") && c.children.is_empty());
    if !bare || map.payload != Payload::None {
        return None;
    }
    let degrees = |tag: &str, positive: char, negative: char| -> Option<f64> {
        let text = map.child(tag)?.payload.text()?.trim().to_ascii_uppercase();
        let (sign, digits) = match text.chars().next()? {
            c if c == positive => (1.0, &text[1..]),
            c if c == negative => (-1.0, &text[1..]),
            _ => (1.0, text.as_str()),
        };
        digits.parse::<f64>().ok().map(|value| sign * value)
    };
    let latitude = degrees("LATI", 'N', 'S')?;
    let longitude = degrees("LONG", 'E', 'W')?;
    ((-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude))
        .then(|| Coordinates::from_degrees(latitude, longitude))
}
