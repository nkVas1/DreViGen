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
    Claimable, Confidence, Coordinates, Entity, Event, EventId, EventKind, Fact, FactId, Family,
    FamilyId, Foreign, ForeignId, Id, Kind, Locator, Membership, Note, NoteId, Participation,
    Partnership, Person, PersonId, Place, PlaceId, PlaceName, PlaceParent, Provenance, Refusal,
    Repository, RepositoryId, Resolution, Role, Sex, Source, SourceId, fold,
};
use drevigen_date::RecordedDate;
use drevigen_date::gedcom::Severity as DateSeverity;
use drevigen_gedcom::{Document, Payload, Structure};

use super::Options;
use super::names;
use super::report::{Mapping, Report};
use crate::export::{AN_EVENT, MILITARY_SERVICE};

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

/// A citation the file gives for a value, and how far the file trusts it.
#[derive(Debug, Clone, Copy)]
struct Cited {
    citation: CitationId,
    confidence: Option<Confidence>,
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
    /// The sources 5.5.1 citations describe in place, by their description.
    described: BTreeMap<String, SourceId>,
    /// The citations made so far that carry nothing beyond their source, place and text, by
    /// those three, so that a citation written out again is not made twice.
    repeated: BTreeMap<(SourceId, Option<String>, Option<String>), CitationId>,
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
            described: BTreeMap::new(),
            repeated: BTreeMap::new(),
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
        // A citation on the person as a whole supports their names: a source that mentions
        // someone records them by name. Without a name, there is nothing for it to support.
        let named = record
            .children
            .iter()
            .any(|c| c.tag == "NAME" && !names::person_name(c).is_empty());
        let general = if named {
            let sources = record.children_tagged("SOUR").collect();
            self.sources(subject, sources, "INDI")
        } else {
            Vec::new()
        };
        for child in &record.children {
            let tag = child.tag.as_str();
            match tag {
                "NAME" => self.name(&mut person, record, child, &general),
                "SEX" => self.sex(&mut person, record, child),
                "SOUR" if named => {}
                "FAMC" => self.link(id, child, true),
                "FAMS" => self.link(id, child, false),
                "NOTE" | "SNOTE" => self.annotation(subject, child, "INDI"),
                _ => match (person_event(tag), attribute_kind(tag)) {
                    (Some(kind), _) if mappable(child) => events.push((child, kind)),
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
        // A citation on the family as a whole supports the union: its partners, not its
        // children, whose own records say where they come from.
        let partnered = record.children.iter().any(|c| {
            matches!(c.tag.as_str(), "HUSB" | "WIFE")
                && matches!(self.pointer(c), Some(Target::Person(_)))
        }) || self.as_partner.keys().any(|&(_, family)| family == id);
        let general = if partnered {
            let sources = record.children_tagged("SOUR").collect();
            self.sources(subject, sources, "FAM")
        } else {
            Vec::new()
        };
        for child in &record.children {
            if let Some(kind) = family_event(&child.tag)
                && mappable(child)
            {
                events.push((child, kind));
                continue;
            }
            if child.tag == "SOUR" && partnered {
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
                    let written = Some(child);
                    self.partner(
                        &mut family,
                        person,
                        partnership,
                        record,
                        written,
                        side,
                        &general,
                    );
                }
                ("CHIL", Some(person)) if family.children.iter().all(|m| m.person != person) => {
                    let side = self.as_child.remove(&(person, id));
                    self.child(&mut family, person, record, Some(child), side);
                }
                ("HUSB" | "WIFE" | "CHIL", None) => self.unresolved(subject, child, "FAM"),
                ("NOTE" | "SNOTE", _) => self.annotation(subject, child, "FAM"),
                _ => self.keep(subject, child, "FAM"),
            }
        }

        self.one_sided_links(&mut family, record, &general);
        self.family_events(&mut family, events, record);

        if self.add(record.line, |archive| archive.add_family(family)) {
            self.report.counts.families += 1;
        }
    }

    /// Links that people's records make and the family's record does not: read as if the family
    /// made them too, and said so.
    fn one_sided_links(&mut self, family: &mut Family, record: &'d Structure, general: &[Cited]) {
        let id = family.id;
        let on_this = |links: &BTreeMap<(PersonId, FamilyId), &'d Structure>| -> Vec<_> {
            links
                .iter()
                .filter(|((_, found), _)| *found == id)
                .map(|(&key, &side)| (key, side))
                .collect()
        };
        for (key, side) in on_this(&self.as_partner) {
            self.as_partner.remove(&key);
            self.one_sided(record, side);
            let partnership = Partnership::Partner;
            self.partner(
                family,
                key.0,
                partnership,
                record,
                None,
                Some(side),
                general,
            );
        }
        for (key, side) in on_this(&self.as_child) {
            self.as_child.remove(&key);
            self.one_sided(record, side);
            self.child(family, key.0, record, None, Some(side));
        }
    }

    /// A family's events are its partners' events, each in the role the family gives them.
    fn family_events(
        &mut self,
        family: &mut Family,
        events: Vec<(&'d Structure, EventKind)>,
        record: &'d Structure,
    ) {
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
                None => self.keep(Entity::Family(family.id), structure, "FAM"),
            }
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

    fn name(
        &mut self,
        person: &mut Person,
        record: &'d Structure,
        name: &'d Structure,
        general: &[Cited],
    ) {
        let parsed = names::person_name(name);
        if parsed.is_empty() {
            self.report.note(name.line, Mapping::EmptyName);
            if !name.children.is_empty() {
                self.keep(Entity::Person(person.id), name, "INDI");
            }
            return;
        }
        let id: FactId = self.mint();
        let subject = Entity::Fact(id);
        let mut support =
            self.sources(subject, name.children_tagged("SOUR").collect(), "INDI.NAME");
        support.extend_from_slice(general);
        person.names.push(self.fact(id, parsed, &support, record));
        let rest = name
            .children
            .iter()
            .filter(|child| {
                let part = names::PARTS.contains(&child.tag.as_str()) && child.children.is_empty();
                let kind = child.tag == "TYPE" && names::kind(child).is_some();
                !(part || kind || child.tag == "SOUR")
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
        // A second SEX line is a second claim about the same fact.
        let id = match &person.sex {
            Some(fact) => fact.id,
            None => self.mint(),
        };
        let subject = Entity::Fact(id);
        let support = self.sources(subject, sex.children_tagged("SOUR").collect(), "INDI.SEX");
        let fact = person.sex.get_or_insert_with(|| Fact::new(id));
        self.claim(fact, parsed, &support, record);
        let rest = sex.children.iter().filter(|c| c.tag != "SOUR").collect();
        self.rest(subject, rest, "INDI.SEX");
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

    #[allow(clippy::too_many_arguments)]
    fn partner(
        &mut self,
        family: &mut Family,
        person: PersonId,
        partnership: Partnership,
        record: &'d Structure,
        written: Option<&'d Structure>,
        side: Option<&'d Structure>,
        general: &[Cited],
    ) {
        let id: FactId = self.mint();
        let fact = self.fact(id, partnership, general, record);
        let subject = Entity::Fact(id);
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
        let id: FactId = self.mint();
        let fact = self.fact(id, how, &[], record);
        let subject = Entity::Fact(id);
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

    /// An event, with its date, place and description as facts and its participants in their
    /// roles. Added to the archive before anything beneath it is kept or annotated, so that a
    /// refusal leaves nothing behind; the caller then keeps the structure whole.
    ///
    /// A generic `EVEN` is what its `TYPE` says it is, and its value describes it. A specific
    /// event's `TYPE` "further classifies" it in GEDCOM 7's words — *венчание*, *в больнице* — and
    /// is its description, unless the file gave it words as its value, as many programs do.
    fn event(
        &mut self,
        structure: &'d Structure,
        kind: EventKind,
        participants: Vec<(PersonId, Role)>,
        record: &'d Structure,
        parent: &str,
    ) -> Option<EventId> {
        let classified = structure.children.iter().find(|child| {
            child.tag == "TYPE" && child.children.is_empty() && text_of(child).is_some()
        });
        let words = described(structure);
        let (kind, label, description, taken) = if structure.tag == "EVEN" {
            let (kind, label) = generic_event(classified.and_then(text_of).as_deref())?;
            (kind, label, words, classified)
        } else if words.is_some() {
            (kind, None, words, None)
        } else {
            (kind, None, classified.and_then(text_of), classified)
        };

        let id: EventId = self.mint();
        let mut event = Event::new(id, kind);
        event.label = label;
        let path = format!("{parent}.{}", structure.tag);
        // What the event's citations say, they say of all of it: its date, its place, and who
        // took part in what role.
        let support = self.sources(
            Entity::Event(id),
            structure.children_tagged("SOUR").collect(),
            &path,
        );
        let mut rest = Vec::new();
        let mut beneath = Vec::new();
        for child in &structure.children {
            match child.tag.as_str() {
                "SOUR" => {}
                "TYPE" if taken.is_some_and(|taken| core::ptr::eq(taken, child)) => {}
                "DATE" if event.date.is_none() => match self.date(child) {
                    Some((date, more)) => {
                        let fact: FactId = self.mint();
                        beneath.push((Entity::Fact(fact), more, "DATE"));
                        event.date = Some(self.fact(fact, date, &support, record));
                    }
                    None => rest.push(child),
                },
                "PLAC" if event.place.is_none() => match self.place(child) {
                    Some((place, more)) => {
                        let fact: FactId = self.mint();
                        beneath.push((Entity::Fact(fact), more, "PLAC"));
                        event.place = Some(self.fact(fact, place, &support, record));
                    }
                    None => rest.push(child),
                },
                _ => rest.push(child),
            }
        }
        if let Some(description) = description {
            let fact: FactId = self.mint();
            event.description = Some(self.fact(fact, description, &support, record));
        }
        for (person, role) in participants {
            let fact: FactId = self.mint();
            let role = self.fact(fact, role, &support, record);
            event.participants.push(Participation { person, role });
        }
        if !self.add(structure.line, |archive| archive.add_event(event)) {
            return None;
        }
        self.report.counts.events += 1;
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
        let path = format!("INDI.{}", structure.tag);
        let id: FactId = self.mint();
        let subject = Entity::Fact(id);
        let support = self.sources(subject, structure.children_tagged("SOUR").collect(), &path);
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
                "SOUR" => {}
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
        person
            .attributes
            .push(self.fact(id, value, &support, record));
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

    /// A fact claiming a value, as [`Mapper::claim`] claims it.
    fn fact<V: Claimable + Clone>(
        &mut self,
        id: FactId,
        value: V,
        support: &[Cited],
        record: &'d Structure,
    ) -> Fact<V> {
        let mut fact = Fact::new(id);
        self.claim(&mut fact, value, support, record);
        fact
    }

    /// Asserts a value once for each citation the file gives for it, with the confidence the
    /// file gives that citation — or, where it gives none, once, cited by the file's own citation
    /// of the record (ADR 0011).
    fn claim<V: Claimable + Clone>(
        &mut self,
        fact: &mut Fact<V>,
        value: V,
        support: &[Cited],
        record: &'d Structure,
    ) {
        let support = if support.is_empty() {
            vec![Cited {
                citation: self.citation(record),
                confidence: None,
            }]
        } else {
            support.to_vec()
        };
        let values = core::iter::repeat_n(value, support.len());
        for (cited, value) in support.into_iter().zip(values) {
            let assertion = Assertion {
                citation: Some(cited.citation),
                confidence: cited.confidence,
                ..Assertion::new(self.mint(), value, self.made)
            };
            // A fresh assertion identity: nothing to refuse.
            let _ = fact.claims.assert(assertion);
        }
    }

    /// The citations a list of `SOUR` structures make; those that cite nothing the file has are
    /// reported and kept under the subject.
    fn sources(
        &mut self,
        subject: Entity,
        sources: Vec<&'d Structure>,
        parent: &str,
    ) -> Vec<Cited> {
        sources
            .into_iter()
            .filter_map(|source| self.cited(subject, source, parent))
            .collect()
    }

    /// One `SOUR` beneath a fact: a citation of a source record, or — 5.5.1's other form — of a
    /// source described in place, which becomes a source of its own. Its `PAGE` is the locator,
    /// its text the transcription, its `QUAY` the confidence of what it supports.
    fn cited(&mut self, subject: Entity, structure: &'d Structure, parent: &str) -> Option<Cited> {
        let source = match &structure.payload {
            Payload::Pointer(_) => {
                let Some(Target::Source(source)) = self.pointer(structure) else {
                    self.unresolved(subject, structure, parent);
                    return None;
                };
                source
            }
            Payload::Text(description) => self.described_source(description, structure.line),
            // A citation of nothing — 7.0's @VOID@ — supports nothing, and is kept as written.
            Payload::Void | Payload::None => {
                self.keep(subject, structure, parent);
                return None;
            }
        };
        let described = matches!(structure.payload, Payload::Text(_));
        let id: CitationId = self.mint();
        let mut citation = Citation {
            id,
            source,
            locator: None,
            transcription: None,
            media: Vec::new(),
        };
        let mut confidence = None;
        let mut rest = Vec::new();
        let mut data = None;
        for child in &structure.children {
            let bare = child.children.is_empty();
            match child.tag.as_str() {
                "PAGE" if bare && citation.locator.is_none() => {
                    citation.locator = text_of(child).map(|page| read_locator(&page));
                }
                "QUAY" if bare && confidence.is_none() && quay(child).is_some() => {
                    confidence = quay(child);
                }
                // The described form writes the transcription beneath it; the record form, beneath
                // its DATA.
                "TEXT" if bare && described && citation.transcription.is_none() => {
                    citation.transcription = text_of(child);
                }
                "DATA" if data.is_none() => {
                    let mut more = Vec::new();
                    for part in &child.children {
                        let text = part.tag == "TEXT" && part.children.is_empty();
                        if text && citation.transcription.is_none() {
                            citation.transcription = text_of(part);
                        } else {
                            more.push(part);
                        }
                    }
                    data = Some((child, more));
                }
                _ => rest.push(child),
            }
        }
        // A citation the file repeats word for word — the same source, place and text, with
        // nothing more beneath it — is one citation, however many facts it is written under.
        let plain = rest.is_empty() && data.as_ref().is_none_or(|(_, more)| more.is_empty());
        let key = (
            citation.source,
            citation
                .locator
                .as_ref()
                .map(drevigen_core::Locator::written),
            citation.transcription.clone(),
        );
        if plain && let Some(&existing) = self.repeated.get(&key) {
            return Some(Cited {
                citation: existing,
                confidence,
            });
        }
        if !self.add(structure.line, |archive| archive.add_citation(citation)) {
            self.keep(subject, structure, parent);
            return None;
        }
        if plain {
            self.repeated.insert(key, id);
        }
        self.report.counts.citations += 1;
        let subject = Entity::Citation(id);
        self.rest(subject, rest, "SOUR");
        if let Some((data, more)) = data
            && !more.is_empty()
        {
            self.keep_partly(subject, data, more, "SOUR");
        }
        Some(Cited {
            citation: id,
            confidence,
        })
    }

    /// The source a 5.5.1 citation describes in place instead of pointing at: one source for each
    /// description, however many times the file repeats it.
    fn described_source(&mut self, description: &str, line: usize) -> SourceId {
        let title = description.trim().to_owned();
        if let Some(source) = self.described.get(&title) {
            return *source;
        }
        let id: SourceId = self.mint();
        let source = Source {
            id,
            title: title.clone(),
            author: None,
            publication: None,
            repository: None,
            call_number: None,
        };
        if self.add(line, |archive| archive.add_source(source)) {
            self.report.counts.sources += 1;
        }
        self.described.insert(title, id);
        id
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
    fn annotation(&mut self, subject: Entity, note: &'d Structure, parent: &str) {
        let line = note.line;
        if note.payload.pointer().is_some() {
            let Some(Target::Note(id)) = self.pointer(note) else {
                return self.unresolved(subject, note, parent);
            };
            self.deferred.push(Deferred::Annotate {
                subject,
                note: id,
                line,
            });
            if !note.children.is_empty() {
                let beyond = note.children.iter().collect();
                self.keep_partly(subject, note, beyond, parent);
            }
            return;
        }
        let Some(text) = note.payload.text() else {
            if !note.children.is_empty() {
                self.keep(subject, note, parent);
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
        // 7.0's @VOID@ points at nothing on purpose — "a pointer is required here, and there is no
        // record" — and is kept without remark.
        if let Payload::Pointer(xref) = &structure.payload {
            let xref = xref.clone();
            self.report
                .note(structure.line, Mapping::Unresolved { xref });
        }
        self.keep(subject, structure, parent);
    }

    /// The substructures a mapping did not take: notes annotate, everything else is kept.
    fn rest(&mut self, subject: Entity, rest: Vec<&'d Structure>, parent: &str) {
        for structure in rest {
            match structure.tag.as_str() {
                "NOTE" | "SNOTE" => self.annotation(subject, structure, parent),
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
        let remainder = only.is_some();
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
        *self.report.kept.entry(path.clone()).or_default() += 1;
        self.deferred.push(Deferred::Keep {
            item: Foreign {
                id,
                subject,
                path,
                format: self.format.clone(),
                text: kept.write(),
                remainder,
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

/// The kind of a person's event GEDCOM writes with this tag, where the model has the same kind;
/// for the generic `EVEN`, `Other` until its `TYPE` is read. The kinds the model does not
/// distinguish — `BARM`, `FCOM` and the rest — are kept, because `Other` would lose which one it
/// was.
fn person_event(tag: &str) -> Option<EventKind> {
    Some(match tag {
        "EVEN" => EventKind::Other,
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
        "EVEN" => EventKind::Other,
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

/// Whether an event's value is one the model can hold: nothing; `Y`, "it happened, and nothing
/// more is known"; or words, which become its description. A pointer is none of these.
fn mappable(event: &Structure) -> bool {
    matches!(event.payload, Payload::None | Payload::Text(_))
}

/// The words an event's value gives it, where it gives more than `Y`: MobileFamilyTree writes
/// *пропал без вести* there, and GEDCOM 7 a generic event's description.
fn described(event: &Structure) -> Option<String> {
    text_of(event).filter(|text| !text.eq_ignore_ascii_case("Y"))
}

/// What a generic `EVEN` is, from the words of its `TYPE`: military service, which GEDCOM has no
/// tag for and an export writes this way; an event of no particular kind; or anything else, under
/// its own name. `None` for a program's code in place of words — MyHeritage classifies a couple
/// as `MYHERITAGE:REL_PARTNERS` this way — which says something about a union rather than naming
/// an event, and is kept.
fn generic_event(classified: Option<&str>) -> Option<(EventKind, Option<String>)> {
    let Some(words) = classified else {
        return Some((EventKind::Other, None));
    };
    let code = words.contains(':')
        && words
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, '_' | ':'));
    if code {
        return None;
    }
    let folded = fold(words);
    Some(if folded == fold(MILITARY_SERVICE) {
        (EventKind::Military, None)
    } else if folded == fold(AN_EVENT) {
        (EventKind::Other, None)
    } else {
        (EventKind::Other, Some(words.to_owned()))
    })
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

/// A `QUAY` read as a confidence: 0 to 3, nothing else.
fn quay(quay: &Structure) -> Option<Confidence> {
    text_of(quay)?
        .parse::<u8>()
        .ok()
        .and_then(Confidence::from_quay)
}

/// A `PAGE` read as a locator. Russian archival files are numbered by leaf, and a citation of one
/// is written *л. 17* or *л. 17 об.* — the back of the leaf — which is read as a leaf; anything
/// else is kept as written.
pub(super) fn read_locator(page: &str) -> Locator {
    leaf(page).unwrap_or_else(|| Locator::Free(page.to_owned()))
}

fn leaf(page: &str) -> Option<Locator> {
    let lower = page.trim().to_lowercase();
    let rest = lower
        .strip_prefix("лист")
        .or_else(|| lower.strip_prefix("л."))?
        .trim_start();
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    // A leaf number may carry one letter: 17а.
    let letter = rest[digits..]
        .chars()
        .next()
        .filter(|c| matches!(c, 'а'..='я'))
        .filter(|_| !rest[digits..].starts_with("об"));
    let number_end = digits + letter.map_or(0, char::len_utf8);
    let verso = match rest[number_end..].trim() {
        "" => false,
        "об" | "об." | "оборот" => true,
        _ => return None,
    };
    Some(Locator::Leaf {
        leaf: rest[..number_end].to_owned(),
        verso,
    })
}
