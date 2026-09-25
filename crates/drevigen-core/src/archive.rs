//! The archive: every entity, the indexes that relate them, and the edits that keep them sound.
//!
//! # Two ways in, on purpose
//!
//! **Adding** — [`Archive::add_person`] and its siblings — checks what must always hold: an
//! identity is not reused, and nothing points at something that does not exist. It does *not*
//! refuse a family structure that makes someone their own ancestor, because an import must never
//! fail on the data it was given. Real files contain cycles, and a tool that refuses the file
//! teaches its user nothing about the error; a tool that loads it and shows the cycle does. The
//! audit rules find them.
//!
//! **Relating** — [`Archive::assert_child`], [`Archive::assert_partner`] — is what a person does
//! by hand, and it refuses what would make the tree impossible: a child who is also a partner in
//! the same family, a third partner, and anyone becoming their own ancestor. A person should be
//! stopped at the moment of the mistake, with a sentence saying who and why, not told about it in
//! a report later.
//!
//! # What queries see
//!
//! Relationships are read through their claims. A membership whose every assertion has been
//! withdrawn does not hold and is not returned; a removed person is not anyone's parent. The
//! history is all still there — [`Archive::person`] returns a removed person — but the family as
//! it currently stands is what the relationship queries describe.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::claim::{Assertion, ClaimError, Claims, Conclusion, Provenance};
use crate::id::{
    CitationId, EventId, FactId, FamilyId, MediaId, NoteId, PersonId, PlaceId, RepositoryId,
    SourceId,
};
use crate::model::{
    Attribute, ChildRelation, Citation, Event, Fact, Family, Media, Membership, Note,
    Participation, Partnership, Person, Place, Repository, Role, Source,
};
use crate::name::PersonName;
use crate::value::Sex;
use drevigen_date::RecordedDate;

/// Anything in the archive, by identity. Used to say what an edit was refused about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entity {
    /// A person.
    Person(PersonId),
    /// A family.
    Family(FamilyId),
    /// An event.
    Event(EventId),
    /// A place.
    Place(PlaceId),
    /// A source.
    Source(SourceId),
    /// A citation.
    Citation(CitationId),
    /// A repository.
    Repository(RepositoryId),
    /// A note.
    Note(NoteId),
    /// A media item.
    Media(MediaId),
    /// A fact.
    Fact(FactId),
}

/// Why an edit was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The identity is already in use.
    Duplicate(Entity),
    /// The edit refers to something that is not in the archive.
    Missing(Entity),
    /// The edit refers to something that has been removed.
    Removed(Entity),
    /// The fact exists, but belongs to a different owner or holds a different kind of value.
    WrongFact(FactId),
    /// The claim layer refused.
    Claim(ClaimError),
    /// A person cannot be both a partner and a child in one family.
    PartnerAndChild {
        /// Who.
        person: PersonId,
        /// Which family.
        family: FamilyId,
    },
    /// A family is a union of at most two partners.
    ThirdPartner {
        /// Which family.
        family: FamilyId,
    },
    /// The edit would make someone their own ancestor.
    Cycle {
        /// The person who would become their own ancestor.
        person: PersonId,
        /// The family through which it would happen.
        family: FamilyId,
    },
}

impl core::fmt::Display for Refusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Duplicate(entity) => write!(f, "{entity:?} is already in the archive"),
            Self::Missing(entity) => write!(f, "{entity:?} is not in the archive"),
            Self::Removed(entity) => write!(f, "{entity:?} has been removed"),
            Self::WrongFact(fact) => write!(f, "{fact:?} is not that kind of fact"),
            Self::Claim(error) => write!(f, "{error}"),
            Self::PartnerAndChild { person, family } => {
                write!(
                    f,
                    "{person:?} cannot be a partner and a child in {family:?}"
                )
            }
            Self::ThirdPartner { family } => write!(f, "{family:?} already has two partners"),
            Self::Cycle { person, family } => {
                write!(
                    f,
                    "through {family:?}, {person:?} would become their own ancestor"
                )
            }
        }
    }
}

impl core::error::Error for Refusal {}

impl From<ClaimError> for Refusal {
    fn from(error: ClaimError) -> Self {
        Self::Claim(error)
    }
}

/// Where a fact lives, so that an edit addressed to a fact alone can find it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Location {
    Name(PersonId),
    Sex(PersonId),
    Attribute(PersonId),
    EventDate(EventId),
    EventPlace(EventId),
    Participation(EventId),
    Partner(FamilyId),
    Child(FamilyId),
}

/// Every entity, and the indexes relating them.
#[derive(Debug, Clone, Default)]
pub struct Archive {
    people: BTreeMap<PersonId, Person>,
    families: BTreeMap<FamilyId, Family>,
    events: BTreeMap<EventId, Event>,
    places: BTreeMap<PlaceId, Place>,
    sources: BTreeMap<SourceId, Source>,
    citations: BTreeMap<CitationId, Citation>,
    repositories: BTreeMap<RepositoryId, Repository>,
    notes: BTreeMap<NoteId, Note>,
    media: BTreeMap<MediaId, Media>,

    facts: BTreeMap<FactId, Location>,
    as_child: BTreeMap<PersonId, BTreeSet<FamilyId>>,
    as_partner: BTreeMap<PersonId, BTreeSet<FamilyId>>,
    in_events: BTreeMap<PersonId, BTreeSet<EventId>>,
}

impl Archive {
    /// An empty archive.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // ── reading ─────────────────────────────────────────────────────────────────────────────

    /// A person, removed or not.
    #[must_use]
    pub fn person(&self, id: PersonId) -> Option<&Person> {
        self.people.get(&id)
    }

    /// A family, removed or not.
    #[must_use]
    pub fn family(&self, id: FamilyId) -> Option<&Family> {
        self.families.get(&id)
    }

    /// An event, removed or not.
    #[must_use]
    pub fn event(&self, id: EventId) -> Option<&Event> {
        self.events.get(&id)
    }

    /// A place.
    #[must_use]
    pub fn place(&self, id: PlaceId) -> Option<&Place> {
        self.places.get(&id)
    }

    /// A source.
    #[must_use]
    pub fn source(&self, id: SourceId) -> Option<&Source> {
        self.sources.get(&id)
    }

    /// A citation.
    #[must_use]
    pub fn citation(&self, id: CitationId) -> Option<&Citation> {
        self.citations.get(&id)
    }

    /// A repository.
    #[must_use]
    pub fn repository(&self, id: RepositoryId) -> Option<&Repository> {
        self.repositories.get(&id)
    }

    /// A media item.
    #[must_use]
    pub fn media(&self, id: MediaId) -> Option<&Media> {
        self.media.get(&id)
    }

    /// Every person who has not been removed, in identity order — which, for minted identities,
    /// is the order they were added.
    pub fn people(&self) -> impl Iterator<Item = &Person> {
        self.people.values().filter(|person| !person.is_removed())
    }

    /// Every family that has not been removed.
    pub fn families(&self) -> impl Iterator<Item = &Family> {
        self.families
            .values()
            .filter(|family| family.removed.is_none())
    }

    /// Every event that has not been removed.
    pub fn events(&self) -> impl Iterator<Item = &Event> {
        self.events.values().filter(|event| event.removed.is_none())
    }

    /// How many people have not been removed.
    #[must_use]
    pub fn population(&self) -> usize {
        self.people().count()
    }

    // ── relationships ───────────────────────────────────────────────────────────────────────

    /// The families a person is currently a child in.
    pub fn families_as_child(&self, person: PersonId) -> impl Iterator<Item = &Family> {
        self.indexed_families(&self.as_child, person)
            .filter(move |family| family.current_children().any(|child| child == person))
    }

    /// The families a person is currently a partner in.
    pub fn families_as_partner(&self, person: PersonId) -> impl Iterator<Item = &Family> {
        self.indexed_families(&self.as_partner, person)
            .filter(move |family| family.current_partners().any(|p| p == person))
    }

    /// A person's parents: the partners of every family they are currently a child in.
    #[must_use]
    pub fn parents_of(&self, person: PersonId) -> Vec<PersonId> {
        let mut parents: Vec<PersonId> = self
            .families_as_child(person)
            .flat_map(Family::current_partners)
            .filter(|parent| self.is_present(*parent))
            .collect();
        parents.sort_unstable();
        parents.dedup();
        parents
    }

    /// A person's children, across every union.
    #[must_use]
    pub fn children_of(&self, person: PersonId) -> Vec<PersonId> {
        let mut children: Vec<PersonId> = self
            .families_as_partner(person)
            .flat_map(Family::current_children)
            .filter(|child| self.is_present(*child))
            .collect();
        children.sort_unstable();
        children.dedup();
        children
    }

    /// A person's partners, across every union.
    #[must_use]
    pub fn partners_of(&self, person: PersonId) -> Vec<PersonId> {
        let mut partners: Vec<PersonId> = self
            .families_as_partner(person)
            .flat_map(Family::current_partners)
            .filter(|partner| *partner != person && self.is_present(*partner))
            .collect();
        partners.sort_unstable();
        partners.dedup();
        partners
    }

    /// A person's siblings: everyone else who is a child in any family they are a child in. Half
    /// siblings included, because they are included in every family that has them.
    #[must_use]
    pub fn siblings_of(&self, person: PersonId) -> Vec<PersonId> {
        let mut siblings: Vec<PersonId> = self
            .families_as_child(person)
            .flat_map(Family::current_children)
            .filter(|sibling| *sibling != person && self.is_present(*sibling))
            .collect();
        siblings.sort_unstable();
        siblings.dedup();
        siblings
    }

    /// The events a person currently takes part in, in any role.
    pub fn events_of(&self, person: PersonId) -> impl Iterator<Item = &Event> {
        self.in_events
            .get(&person)
            .into_iter()
            .flatten()
            .filter_map(|id| self.events.get(id))
            .filter(|event| event.removed.is_none())
            .filter(move |event| {
                event
                    .participants
                    .iter()
                    .any(|p| p.person == person && p.role.holds())
            })
    }

    /// Whether `ancestor` is an ancestor of `descendant`, at any distance.
    ///
    /// A breadth-first walk upwards with a visited set, so pedigree collapse — a couple who are
    /// second cousins, a common ancestor reached twice — costs nothing extra and a cycle in
    /// imported data does not loop.
    #[must_use]
    pub fn is_ancestor(&self, ancestor: PersonId, descendant: PersonId) -> bool {
        self.ancestors_of(descendant).contains(&ancestor)
    }

    /// Every ancestor of a person, each once.
    #[must_use]
    pub fn ancestors_of(&self, person: PersonId) -> BTreeSet<PersonId> {
        let mut seen = BTreeSet::new();
        let mut queue: VecDeque<PersonId> = self.parents_of(person).into();
        while let Some(next) = queue.pop_front() {
            if seen.insert(next) {
                queue.extend(self.parents_of(next));
            }
        }
        seen
    }

    fn indexed_families<'a>(
        &'a self,
        index: &'a BTreeMap<PersonId, BTreeSet<FamilyId>>,
        person: PersonId,
    ) -> impl Iterator<Item = &'a Family> + 'a {
        index
            .get(&person)
            .into_iter()
            .flatten()
            .filter_map(|id| self.families.get(id))
            .filter(|family| family.removed.is_none())
    }

    /// Whether a person exists and has not been removed.
    fn is_present(&self, person: PersonId) -> bool {
        self.people.get(&person).is_some_and(|p| !p.is_removed())
    }

    // ── adding ──────────────────────────────────────────────────────────────────────────────

    /// Adds a person, with whatever facts they already carry.
    ///
    /// # Errors
    ///
    /// [`Refusal::Duplicate`] if the person or any of their facts is already in the archive.
    pub fn add_person(&mut self, person: Person) -> Result<(), Refusal> {
        if self.people.contains_key(&person.id) {
            return Err(Refusal::Duplicate(Entity::Person(person.id)));
        }
        let mut facts: Vec<(FactId, Location)> = person
            .names
            .iter()
            .map(|fact| (fact.id, Location::Name(person.id)))
            .chain(
                person
                    .sex
                    .iter()
                    .map(|fact| (fact.id, Location::Sex(person.id))),
            )
            .chain(
                person
                    .attributes
                    .iter()
                    .map(|fact| (fact.id, Location::Attribute(person.id))),
            )
            .collect();
        self.register_facts(&mut facts)?;
        self.people.insert(person.id, person);
        Ok(())
    }

    /// Adds an event, with its participants and facts.
    ///
    /// # Errors
    ///
    /// [`Refusal::Duplicate`] for a reused identity; [`Refusal::Missing`] if a participant or the
    /// recorded place is not in the archive.
    pub fn add_event(&mut self, event: Event) -> Result<(), Refusal> {
        if self.events.contains_key(&event.id) {
            return Err(Refusal::Duplicate(Entity::Event(event.id)));
        }
        for participation in &event.participants {
            self.require_person(participation.person)?;
        }
        if let Some(place) = &event.place {
            for assertion in place.claims.assertions() {
                self.require_place(assertion.value)?;
            }
        }

        let mut facts: Vec<(FactId, Location)> = event
            .date
            .iter()
            .map(|fact| (fact.id, Location::EventDate(event.id)))
            .chain(
                event
                    .place
                    .iter()
                    .map(|fact| (fact.id, Location::EventPlace(event.id))),
            )
            .chain(
                event
                    .participants
                    .iter()
                    .map(|p| (p.role.id, Location::Participation(event.id))),
            )
            .collect();
        self.register_facts(&mut facts)?;

        for participation in &event.participants {
            self.in_events
                .entry(participation.person)
                .or_default()
                .insert(event.id);
        }
        self.events.insert(event.id, event);
        Ok(())
    }

    /// Adds a family, with its members and the events of the union.
    ///
    /// Checks that everyone it names exists. Does **not** check the shape of the tree — see the
    /// module documentation for why an import must not be refused for a cycle.
    ///
    /// # Errors
    ///
    /// [`Refusal::Duplicate`] or [`Refusal::Missing`].
    pub fn add_family(&mut self, family: Family) -> Result<(), Refusal> {
        if self.families.contains_key(&family.id) {
            return Err(Refusal::Duplicate(Entity::Family(family.id)));
        }
        for membership in &family.partners {
            self.require_person(membership.person)?;
        }
        for membership in &family.children {
            self.require_person(membership.person)?;
        }
        for event in &family.events {
            if !self.events.contains_key(event) {
                return Err(Refusal::Missing(Entity::Event(*event)));
            }
        }

        let mut facts: Vec<(FactId, Location)> = family
            .partners
            .iter()
            .map(|m| (m.fact.id, Location::Partner(family.id)))
            .chain(
                family
                    .children
                    .iter()
                    .map(|m| (m.fact.id, Location::Child(family.id))),
            )
            .collect();
        self.register_facts(&mut facts)?;

        for membership in &family.partners {
            self.as_partner
                .entry(membership.person)
                .or_default()
                .insert(family.id);
        }
        for membership in &family.children {
            self.as_child
                .entry(membership.person)
                .or_default()
                .insert(family.id);
        }
        self.families.insert(family.id, family);
        Ok(())
    }

    /// Adds a place.
    ///
    /// # Errors
    ///
    /// [`Refusal::Duplicate`], or [`Refusal::Missing`] for a containing place not yet added.
    pub fn add_place(&mut self, place: Place) -> Result<(), Refusal> {
        if self.places.contains_key(&place.id) {
            return Err(Refusal::Duplicate(Entity::Place(place.id)));
        }
        for parent in &place.within {
            self.require_place(parent.place)?;
        }
        self.places.insert(place.id, place);
        Ok(())
    }

    /// Adds a repository.
    ///
    /// # Errors
    ///
    /// [`Refusal::Duplicate`].
    pub fn add_repository(&mut self, repository: Repository) -> Result<(), Refusal> {
        insert_unique(&mut self.repositories, repository.id, repository, |id| {
            Entity::Repository(id)
        })
    }

    /// Adds a source.
    ///
    /// # Errors
    ///
    /// [`Refusal::Duplicate`], or [`Refusal::Missing`] for its repository.
    pub fn add_source(&mut self, source: Source) -> Result<(), Refusal> {
        if let Some(repository) = source.repository
            && !self.repositories.contains_key(&repository)
        {
            return Err(Refusal::Missing(Entity::Repository(repository)));
        }
        insert_unique(&mut self.sources, source.id, source, Entity::Source)
    }

    /// Adds a citation.
    ///
    /// # Errors
    ///
    /// [`Refusal::Duplicate`], or [`Refusal::Missing`] for its source or media.
    pub fn add_citation(&mut self, citation: Citation) -> Result<(), Refusal> {
        if !self.sources.contains_key(&citation.source) {
            return Err(Refusal::Missing(Entity::Source(citation.source)));
        }
        for media in &citation.media {
            if !self.media.contains_key(media) {
                return Err(Refusal::Missing(Entity::Media(*media)));
            }
        }
        insert_unique(&mut self.citations, citation.id, citation, Entity::Citation)
    }

    /// Adds a note.
    ///
    /// # Errors
    ///
    /// [`Refusal::Duplicate`].
    pub fn add_note(&mut self, note: Note) -> Result<(), Refusal> {
        insert_unique(&mut self.notes, note.id, note, Entity::Note)
    }

    /// Adds a media item.
    ///
    /// # Errors
    ///
    /// [`Refusal::Duplicate`].
    pub fn add_media(&mut self, media: Media) -> Result<(), Refusal> {
        insert_unique(&mut self.media, media.id, media, Entity::Media)
    }

    // ── asserting ───────────────────────────────────────────────────────────────────────────

    /// Asserts something about one of a person's names, creating the name fact if it is new.
    ///
    /// # Errors
    ///
    /// [`Refusal::Missing`] for the person or the citation; [`Refusal::WrongFact`] if the fact
    /// belongs elsewhere; [`Refusal::Claim`] for a duplicate assertion.
    pub fn assert_name(
        &mut self,
        person: PersonId,
        fact: FactId,
        assertion: Assertion<PersonName>,
    ) -> Result<(), Refusal> {
        self.check_citation(assertion.citation)?;
        let owner = self.live_person_mut(person)?;
        let claims = fact_in(&mut owner.names, fact);
        claims.assert(assertion)?;
        self.facts.entry(fact).or_insert(Location::Name(person));
        Ok(())
    }

    /// Asserts a person's sex.
    ///
    /// # Errors
    ///
    /// As for [`Archive::assert_name`].
    pub fn assert_sex(
        &mut self,
        person: PersonId,
        fact: FactId,
        assertion: Assertion<Sex>,
    ) -> Result<(), Refusal> {
        self.check_citation(assertion.citation)?;
        self.check_single(fact, Location::Sex(person))?;
        let owner = self.live_person_mut(person)?;
        owner
            .sex
            .get_or_insert_with(|| Fact::new(fact))
            .claims
            .assert(assertion)?;
        self.facts.insert(fact, Location::Sex(person));
        Ok(())
    }

    /// Asserts something about one of a person's attributes, creating it if it is new.
    ///
    /// # Errors
    ///
    /// As for [`Archive::assert_name`], and [`Refusal::Missing`] for a place.
    pub fn assert_attribute(
        &mut self,
        person: PersonId,
        fact: FactId,
        assertion: Assertion<Attribute>,
    ) -> Result<(), Refusal> {
        self.check_citation(assertion.citation)?;
        if let Some(place) = assertion.value.place {
            self.require_place(place)?;
        }
        let owner = self.live_person_mut(person)?;
        fact_in(&mut owner.attributes, fact).assert(assertion)?;
        self.facts
            .entry(fact)
            .or_insert(Location::Attribute(person));
        Ok(())
    }

    /// Asserts an event's date.
    ///
    /// # Errors
    ///
    /// [`Refusal::Missing`] for the event or citation, [`Refusal::WrongFact`], or
    /// [`Refusal::Claim`].
    pub fn assert_event_date(
        &mut self,
        event: EventId,
        fact: FactId,
        assertion: Assertion<RecordedDate>,
    ) -> Result<(), Refusal> {
        self.check_citation(assertion.citation)?;
        self.check_single(fact, Location::EventDate(event))?;
        let owner = self.live_event_mut(event)?;
        owner
            .date
            .get_or_insert_with(|| Fact::new(fact))
            .claims
            .assert(assertion)?;
        self.facts.insert(fact, Location::EventDate(event));
        Ok(())
    }

    /// Asserts an event's place.
    ///
    /// # Errors
    ///
    /// As for [`Archive::assert_event_date`], and [`Refusal::Missing`] for the place.
    pub fn assert_event_place(
        &mut self,
        event: EventId,
        fact: FactId,
        assertion: Assertion<PlaceId>,
    ) -> Result<(), Refusal> {
        self.check_citation(assertion.citation)?;
        self.require_place(assertion.value)?;
        self.check_single(fact, Location::EventPlace(event))?;
        let owner = self.live_event_mut(event)?;
        owner
            .place
            .get_or_insert_with(|| Fact::new(fact))
            .claims
            .assert(assertion)?;
        self.facts.insert(fact, Location::EventPlace(event));
        Ok(())
    }

    /// Asserts that someone took part in an event, and in what role.
    ///
    /// # Errors
    ///
    /// [`Refusal::Missing`] for the event, person or citation; [`Refusal::Removed`] for a removed
    /// person; [`Refusal::WrongFact`] if the fact already records a different person.
    pub fn assert_participation(
        &mut self,
        event: EventId,
        person: PersonId,
        fact: FactId,
        assertion: Assertion<Role>,
    ) -> Result<(), Refusal> {
        self.check_citation(assertion.citation)?;
        self.require_live_person(person)?;
        let owner = self.live_event_mut(event)?;
        membership_claims(
            &mut owner.participants,
            person,
            fact,
            |p| (p.person, &mut p.role),
            |fact| Participation { person, role: fact },
        )?
        .assert(assertion)?;
        self.facts.insert(fact, Location::Participation(event));
        self.in_events.entry(person).or_default().insert(event);
        Ok(())
    }

    /// Asserts that someone is a partner in a family.
    ///
    /// # Errors
    ///
    /// Everything [`Archive::assert_participation`] can refuse, and three refusals that protect
    /// the shape of the tree: [`Refusal::PartnerAndChild`], [`Refusal::ThirdPartner`], and
    /// [`Refusal::Cycle`] if the partner is descended from one of the family's children — which
    /// would make them their own ancestor.
    pub fn assert_partner(
        &mut self,
        family: FamilyId,
        person: PersonId,
        fact: FactId,
        assertion: Assertion<Partnership>,
    ) -> Result<(), Refusal> {
        self.check_citation(assertion.citation)?;
        self.require_live_person(person)?;
        let current = self.live_family(family)?;

        if current.current_children().any(|child| child == person) {
            return Err(Refusal::PartnerAndChild { person, family });
        }
        let already = current.current_partners().any(|p| p == person);
        if !already && current.current_partners().count() >= 2 {
            return Err(Refusal::ThirdPartner { family });
        }
        let children: Vec<PersonId> = current.current_children().collect();
        if children
            .iter()
            .any(|child| self.is_ancestor(*child, person))
        {
            return Err(Refusal::Cycle { person, family });
        }

        let owner = self.live_family_mut(family)?;
        membership_claims(
            &mut owner.partners,
            person,
            fact,
            |m| (m.person, &mut m.fact),
            |fact| Membership { person, fact },
        )?
        .assert(assertion)?;
        self.facts.insert(fact, Location::Partner(family));
        self.as_partner.entry(person).or_default().insert(family);
        Ok(())
    }

    /// Asserts that someone is a child of a family, and how.
    ///
    /// # Errors
    ///
    /// As for [`Archive::assert_partner`]: [`Refusal::PartnerAndChild`], and [`Refusal::Cycle`]
    /// if the child is an ancestor of one of the partners.
    pub fn assert_child(
        &mut self,
        family: FamilyId,
        person: PersonId,
        fact: FactId,
        assertion: Assertion<ChildRelation>,
    ) -> Result<(), Refusal> {
        self.check_citation(assertion.citation)?;
        self.require_live_person(person)?;
        let current = self.live_family(family)?;

        if current.current_partners().any(|partner| partner == person) {
            return Err(Refusal::PartnerAndChild { person, family });
        }
        let partners: Vec<PersonId> = current.current_partners().collect();
        if partners
            .iter()
            .any(|partner| self.is_ancestor(person, *partner))
        {
            return Err(Refusal::Cycle { person, family });
        }

        let owner = self.live_family_mut(family)?;
        membership_claims(
            &mut owner.children,
            person,
            fact,
            |m| (m.person, &mut m.fact),
            |fact| Membership { person, fact },
        )?
        .assert(assertion)?;
        self.facts.insert(fact, Location::Child(family));
        self.as_child.entry(person).or_default().insert(family);
        Ok(())
    }

    /// Withdraws an assertion from whichever fact it belongs to.
    ///
    /// # Errors
    ///
    /// [`Refusal::Missing`] for an unknown fact, or [`Refusal::Claim`].
    pub fn retract(
        &mut self,
        fact: FactId,
        assertion: crate::id::AssertionId,
        by: Provenance,
    ) -> Result<(), Refusal> {
        self.with_claims(fact, |claims| claims.retract(assertion, by))
    }

    /// Withdraws the standing conclusion on a fact.
    ///
    /// # Errors
    ///
    /// [`Refusal::Missing`] for an unknown fact, or [`Refusal::Claim`] if there is none.
    pub fn withdraw_conclusion(&mut self, fact: FactId, by: Provenance) -> Result<(), Refusal> {
        self.with_claims(fact, |claims| claims.withdraw_conclusion(by))
    }

    /// Records a conclusion about an event's date.
    ///
    /// # Errors
    ///
    /// [`Refusal::Missing`], [`Refusal::WrongFact`] or [`Refusal::Claim`].
    pub fn conclude_event_date(
        &mut self,
        fact: FactId,
        conclusion: Conclusion<RecordedDate>,
    ) -> Result<(), Refusal> {
        let Some(Location::EventDate(event)) = self.facts.get(&fact).copied() else {
            return Err(self.missing_or_wrong(fact));
        };
        let owner = self.live_event_mut(event)?;
        let target = owner
            .date
            .as_mut()
            .filter(|f| f.id == fact)
            .ok_or(Refusal::WrongFact(fact))?;
        target.claims.conclude(conclusion)?;
        Ok(())
    }

    // ── removing ────────────────────────────────────────────────────────────────────────────

    /// Removes a person from the tree. A tombstone: the person and everything asserted about them
    /// stay in the history, and stop appearing in the tree.
    ///
    /// # Errors
    ///
    /// [`Refusal::Missing`] or [`Refusal::Removed`].
    pub fn remove_person(&mut self, person: PersonId, by: Provenance) -> Result<(), Refusal> {
        self.live_person_mut(person)?.removed = Some(by);
        Ok(())
    }

    // ── internals ───────────────────────────────────────────────────────────────────────────

    /// Applies an operation that does not depend on the value type to whichever fact `fact` is.
    fn with_claims(
        &mut self,
        fact: FactId,
        operation: impl FnOnce(&mut dyn ErasedClaims) -> Result<(), ClaimError>,
    ) -> Result<(), Refusal> {
        let location = self
            .facts
            .get(&fact)
            .copied()
            .ok_or(Refusal::Missing(Entity::Fact(fact)))?;
        let wrong = || Refusal::WrongFact(fact);
        match location {
            Location::Name(person) => operation(
                claims_by_id(&mut self.live_person_mut(person)?.names, fact).ok_or_else(wrong)?,
            ),
            Location::Attribute(person) => operation(
                claims_by_id(&mut self.live_person_mut(person)?.attributes, fact)
                    .ok_or_else(wrong)?,
            ),
            Location::Sex(person) => operation(
                &mut self
                    .live_person_mut(person)?
                    .sex
                    .as_mut()
                    .filter(|f| f.id == fact)
                    .ok_or_else(wrong)?
                    .claims,
            ),
            Location::EventDate(event) => operation(
                &mut self
                    .live_event_mut(event)?
                    .date
                    .as_mut()
                    .filter(|f| f.id == fact)
                    .ok_or_else(wrong)?
                    .claims,
            ),
            Location::EventPlace(event) => operation(
                &mut self
                    .live_event_mut(event)?
                    .place
                    .as_mut()
                    .filter(|f| f.id == fact)
                    .ok_or_else(wrong)?
                    .claims,
            ),
            Location::Participation(event) => operation(
                &mut self
                    .live_event_mut(event)?
                    .participants
                    .iter_mut()
                    .find(|p| p.role.id == fact)
                    .ok_or_else(wrong)?
                    .role
                    .claims,
            ),
            Location::Partner(family) => operation(
                &mut self
                    .live_family_mut(family)?
                    .partners
                    .iter_mut()
                    .find(|m| m.fact.id == fact)
                    .ok_or_else(wrong)?
                    .fact
                    .claims,
            ),
            Location::Child(family) => operation(
                &mut self
                    .live_family_mut(family)?
                    .children
                    .iter_mut()
                    .find(|m| m.fact.id == fact)
                    .ok_or_else(wrong)?
                    .fact
                    .claims,
            ),
        }
        .map_err(Refusal::Claim)
    }

    fn register_facts(&mut self, facts: &mut [(FactId, Location)]) -> Result<(), Refusal> {
        let mut seen = BTreeSet::new();
        for (fact, _) in facts.iter() {
            if self.facts.contains_key(fact) || !seen.insert(*fact) {
                return Err(Refusal::Duplicate(Entity::Fact(*fact)));
            }
        }
        for (fact, location) in facts.iter() {
            self.facts.insert(*fact, *location);
        }
        Ok(())
    }

    /// A single-valued fact may be created once per owner, under one identity.
    fn check_single(&self, fact: FactId, location: Location) -> Result<(), Refusal> {
        match self.facts.get(&fact) {
            Some(existing) if *existing != location => Err(Refusal::WrongFact(fact)),
            _ => Ok(()),
        }
    }

    fn missing_or_wrong(&self, fact: FactId) -> Refusal {
        if self.facts.contains_key(&fact) {
            Refusal::WrongFact(fact)
        } else {
            Refusal::Missing(Entity::Fact(fact))
        }
    }

    fn check_citation(&self, citation: Option<CitationId>) -> Result<(), Refusal> {
        match citation {
            Some(id) if !self.citations.contains_key(&id) => {
                Err(Refusal::Missing(Entity::Citation(id)))
            }
            _ => Ok(()),
        }
    }

    fn require_person(&self, person: PersonId) -> Result<(), Refusal> {
        if self.people.contains_key(&person) {
            Ok(())
        } else {
            Err(Refusal::Missing(Entity::Person(person)))
        }
    }

    fn require_live_person(&self, person: PersonId) -> Result<(), Refusal> {
        match self.people.get(&person) {
            None => Err(Refusal::Missing(Entity::Person(person))),
            Some(p) if p.is_removed() => Err(Refusal::Removed(Entity::Person(person))),
            Some(_) => Ok(()),
        }
    }

    fn require_place(&self, place: PlaceId) -> Result<(), Refusal> {
        if self.places.contains_key(&place) {
            Ok(())
        } else {
            Err(Refusal::Missing(Entity::Place(place)))
        }
    }

    fn live_person_mut(&mut self, person: PersonId) -> Result<&mut Person, Refusal> {
        match self.people.get_mut(&person) {
            None => Err(Refusal::Missing(Entity::Person(person))),
            Some(p) if p.is_removed() => Err(Refusal::Removed(Entity::Person(person))),
            Some(p) => Ok(p),
        }
    }

    fn live_event_mut(&mut self, event: EventId) -> Result<&mut Event, Refusal> {
        match self.events.get_mut(&event) {
            None => Err(Refusal::Missing(Entity::Event(event))),
            Some(e) if e.removed.is_some() => Err(Refusal::Removed(Entity::Event(event))),
            Some(e) => Ok(e),
        }
    }

    fn live_family(&self, family: FamilyId) -> Result<&Family, Refusal> {
        match self.families.get(&family) {
            None => Err(Refusal::Missing(Entity::Family(family))),
            Some(f) if f.removed.is_some() => Err(Refusal::Removed(Entity::Family(family))),
            Some(f) => Ok(f),
        }
    }

    fn live_family_mut(&mut self, family: FamilyId) -> Result<&mut Family, Refusal> {
        match self.families.get_mut(&family) {
            None => Err(Refusal::Missing(Entity::Family(family))),
            Some(f) if f.removed.is_some() => Err(Refusal::Removed(Entity::Family(family))),
            Some(f) => Ok(f),
        }
    }
}

/// The part of [`Claims`] that does not depend on the value type, so one closure can reach every
/// kind of fact.
trait ErasedClaims {
    fn retract(&mut self, id: crate::id::AssertionId, by: Provenance) -> Result<(), ClaimError>;
    fn withdraw_conclusion(&mut self, by: Provenance) -> Result<(), ClaimError>;
}

impl<V: crate::claim::Claimable> ErasedClaims for Claims<V> {
    fn retract(&mut self, id: crate::id::AssertionId, by: Provenance) -> Result<(), ClaimError> {
        Self::retract(self, id, by)
    }
    fn withdraw_conclusion(&mut self, by: Provenance) -> Result<(), ClaimError> {
        Self::withdraw_conclusion(self, by)
    }
}

/// The claims of a fact in a list, creating the fact if the identity is new.
fn fact_in<V: crate::claim::Claimable>(facts: &mut Vec<Fact<V>>, id: FactId) -> &mut Claims<V> {
    let index = facts
        .iter()
        .position(|fact| fact.id == id)
        .unwrap_or_else(|| {
            facts.push(Fact::new(id));
            facts.len() - 1
        });
    &mut facts[index].claims
}

fn claims_by_id<V: crate::claim::Claimable>(
    facts: &mut [Fact<V>],
    id: FactId,
) -> Option<&mut Claims<V>> {
    facts
        .iter_mut()
        .find(|fact| fact.id == id)
        .map(|fact| &mut fact.claims)
}

/// The claims of a membership-like fact, keyed by person, creating it if new.
///
/// Refuses a fact identity already recording a different person: one membership fact is about one
/// person, and reusing it for another would merge two relationships into one history.
fn membership_claims<T, V: crate::claim::Claimable>(
    list: &mut Vec<T>,
    person: PersonId,
    fact: FactId,
    view: impl Fn(&mut T) -> (PersonId, &mut Fact<V>),
    make: impl FnOnce(Fact<V>) -> T,
) -> Result<&mut Claims<V>, Refusal> {
    let mut found = None;
    for (index, item) in list.iter_mut().enumerate() {
        let (who, existing) = view(item);
        if existing.id == fact {
            if who != person {
                return Err(Refusal::WrongFact(fact));
            }
            found = Some(index);
            break;
        }
    }
    let index = found.unwrap_or_else(|| {
        list.push(make(Fact::new(fact)));
        list.len() - 1
    });
    let (_, existing) = view(&mut list[index]);
    Ok(&mut existing.claims)
}

fn insert_unique<K: Ord + Copy, V>(
    map: &mut BTreeMap<K, V>,
    id: K,
    value: V,
    entity: impl FnOnce(K) -> Entity,
) -> Result<(), Refusal> {
    if map.contains_key(&id) {
        return Err(Refusal::Duplicate(entity(id)));
    }
    map.insert(id, value);
    Ok(())
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod tests;
