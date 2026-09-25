//! The entities: people, events, families, places, and the evidence apparatus.
//!
//! Two kinds of thing live here, and ADR 0010 is the line between them.
//!
//! **Facts about people and relationships are claims.** A person's names and sex, an event's date
//! and place, who took part in it and as what, who belongs to a family and how — each is a
//! [`Fact`] holding the assertions sources make about it. A person is barely more than an identity
//! that facts attach to, which is the point: nothing about them is recorded except as something a
//! source said.
//!
//! **The evidence apparatus is plain data.** Sources, citations, repositories, notes, media and
//! place definitions are what claims cite. A source's title is not evidence of anything; it is the
//! thing the evidence is in.

use drevigen_date::{DateValue, RecordedDate};

use crate::claim::{Claimable, Claims, Provenance, Resolution};
use crate::id::{
    CitationId, EventId, FactId, FamilyId, MediaId, NoteId, PersonId, PlaceId, RepositoryId,
    SourceId,
};
use crate::name::{PersonName, fold};
use crate::value::Sex;

/// One fact, and everything asserted and concluded about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact<V> {
    /// This fact.
    pub id: FactId,
    /// What sources say about it.
    pub claims: Claims<V>,
}

impl<V: Claimable> Fact<V> {
    /// A fact nobody has asserted anything about yet.
    #[must_use]
    pub fn new(id: FactId) -> Self {
        Self {
            id,
            claims: Claims::new(),
        }
    }

    /// What the fact comes to.
    #[must_use]
    pub fn resolve(&self) -> Resolution<'_, V> {
        self.claims.resolve()
    }

    /// Whether anything is asserted that still counts. A membership whose every assertion has
    /// been withdrawn no longer holds — and its history remains.
    #[must_use]
    pub fn holds(&self) -> bool {
        !matches!(self.resolve(), Resolution::Unknown)
    }
}

// ── people ──────────────────────────────────────────────────────────────────────────────────

/// A person: an identity, and the facts sources record about them.
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    /// This person.
    pub id: PersonId,
    /// Every name, each its own fact. Several names are normal, not a conflict.
    pub names: Vec<Fact<PersonName>>,
    /// Sex, once anything is asserted about it.
    pub sex: Option<Fact<Sex>>,
    /// Occupation, estate, religion, residence and the rest.
    pub attributes: Vec<Fact<Attribute>>,
    /// Whether the person is living, where someone has said so. `None` leaves it to be computed
    /// from the facts — the research obligation in `data-standards.md` §7 is a computed flag
    /// with a manual override, and this is the override.
    pub living: Option<bool>,
    /// Who removed this person from the tree, and when. Removal is a tombstone: the person and
    /// everything asserted about them stay in the history.
    pub removed: Option<Provenance>,
}

impl Person {
    /// A person nothing is known about yet.
    #[must_use]
    pub const fn new(id: PersonId) -> Self {
        Self {
            id,
            names: Vec::new(),
            sex: None,
            attributes: Vec::new(),
            living: None,
            removed: None,
        }
    }

    /// The name to show: the first name fact that resolves to anything.
    ///
    /// Birth names are preferred over the rest, because the tree is organised by birth family and
    /// a married name shown on a node among the person's parents reads as a stranger.
    #[must_use]
    pub fn display_name(&self) -> Option<&PersonName> {
        let resolved = || self.names.iter().filter_map(|fact| fact.resolve().value());
        resolved()
            .find(|name| name.kind == Some(crate::name::NameKind::Birth))
            .or_else(|| resolved().next())
    }

    /// Whether the person has been removed.
    #[must_use]
    pub const fn is_removed(&self) -> bool {
        self.removed.is_some()
    }
}

/// What kind of attribute a fact records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeKind {
    /// Occupation: *крестьянин-хлебопашец*, *учитель*.
    Occupation,
    /// Social estate — *сословие*: peasant, townsman, merchant, clergy, nobility. The attribute
    /// Russian records state before any other and GEDCOM has no tag for.
    Estate,
    /// Religion or confession.
    Religion,
    /// Where the person lived.
    Residence,
    /// Nationality or ethnicity as recorded.
    Nationality,
    /// Education.
    Education,
    /// A title of nobility or rank.
    Title,
    /// Property held.
    Property,
    /// Physical description.
    Description,
    /// Something else, named in the label.
    Other,
}

/// An attribute as a source records it: what it was, and optionally when and where.
///
/// Composite on purpose. "In the 1897 census he was a peasant in Туношна" is one statement from
/// one source, and splitting it into three facts would let a second source contradict the date
/// without contradicting the occupation, which is not what it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// What kind.
    pub kind: AttributeKind,
    /// For [`AttributeKind::Other`], what it is called.
    pub label: Option<String>,
    /// What the source says: *крестьянин*, *православная*.
    pub value: String,
    /// When it held.
    pub date: Option<RecordedDate>,
    /// Where it held.
    pub place: Option<PlaceId>,
}

/// Two attributes agree when they are of the same kind, say the same thing, and could have held
/// at the same time and place. A missing date or place is a refinement, not a disagreement.
impl Claimable for Attribute {
    fn compatible(&self, other: &Self) -> bool {
        let dates = match (&self.date, &other.date) {
            (Some(a), Some(b)) => a.compatible(b),
            _ => true,
        };
        let places = match (self.place, other.place) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        };
        self.kind == other.kind
            && self.label.as_deref().map(fold) == other.label.as_deref().map(fold)
            && fold(&self.value) == fold(&other.value)
            && dates
            && places
    }

    fn more_specific_than(&self, other: &Self) -> bool {
        let detail = |attribute: &Self| {
            usize::from(attribute.date.is_some()) + usize::from(attribute.place.is_some())
        };
        let by_date = match (&self.date, &other.date) {
            (Some(a), Some(b)) => a.more_specific_than(b),
            _ => false,
        };
        detail(self) > detail(other) || by_date
    }
}

// ── events ──────────────────────────────────────────────────────────────────────────────────

/// What kind of thing happened.
///
/// GEDCOM 7's individual and family event types, named rather than tagged, plus `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// `BIRT`.
    Birth,
    /// `BAPM` — baptism, and the Orthodox *крещение*.
    Baptism,
    /// `CHR` — the christening of a child.
    Christening,
    /// `DEAT`.
    Death,
    /// `BURI`.
    Burial,
    /// `CREM`.
    Cremation,
    /// `ENGA`.
    Engagement,
    /// `MARB` — the banns, *оглашение*.
    Banns,
    /// `MARR` — marriage, and the Orthodox *венчание*.
    Marriage,
    /// `DIV`.
    Divorce,
    /// `ANUL`.
    Annulment,
    /// `CENS` — being enumerated. The revision lists (*ревизские сказки*) and confession
    /// registers (*исповедные ведомости*) are recorded this way.
    Census,
    /// `EMIG`.
    Emigration,
    /// `IMMI`.
    Immigration,
    /// `NATU`.
    Naturalisation,
    /// `ADOP`.
    Adoption,
    /// `CONF`.
    Confirmation,
    /// `ORDN` — ordination.
    Ordination,
    /// `GRAD`.
    Graduation,
    /// Military service: conscription, a campaign, a discharge. GEDCOM records it as a generic
    /// event with a type.
    Military,
    /// `RETI`.
    Retirement,
    /// `WILL`.
    Will,
    /// `PROB`.
    Probate,
    /// Anything else.
    Other,
}

impl EventKind {
    /// Whether this kind of event can only happen once in a life and fixes its beginning or end.
    /// Used by the rules that check a life's order.
    #[must_use]
    pub const fn is_vital(self) -> bool {
        matches!(
            self,
            Self::Birth
                | Self::Baptism
                | Self::Christening
                | Self::Death
                | Self::Burial
                | Self::Cremation
        )
    }
}

/// What part someone played in an event.
///
/// GEDCOM 7's roles, with the principal made explicit: GEDCOM implies it by where an event is
/// written in the file, and a model with no file layout has to say it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// The person the event is about: the child at a birth, the deceased at a burial.
    Principal,
    /// `FATH`.
    Father,
    /// `MOTH`.
    Mother,
    /// `PARENT`, where the record does not say which.
    Parent,
    /// `CHIL`.
    Child,
    /// `HUSB`.
    Husband,
    /// `WIFE`.
    Wife,
    /// `SPOU`, where the record does not say which.
    Spouse,
    /// `GODP` — a godparent, *восприемник*.
    Godparent,
    /// `WITN` — a witness, *поручитель* at a wedding.
    Witness,
    /// `CLERGY`.
    Clergy,
    /// `OFFICIATOR`.
    Officiator,
    /// Whoever reported the event to the recorder. GEDCOM has no tag and files it as `OTHER`.
    Informant,
    /// `FRIEND`.
    Friend,
    /// `NGHBR`.
    Neighbour,
    /// `OTHER`.
    Other,
}

/// Two roles agree when they are the same, or when one is the general form of the other: a parent
/// and a father, a spouse and a wife. `Other` says least and agrees with anything.
impl Claimable for Role {
    fn compatible(&self, other: &Self) -> bool {
        self == other || generalises(*self, *other) || generalises(*other, *self)
    }

    fn more_specific_than(&self, other: &Self) -> bool {
        self != other && generalises(*other, *self)
    }
}

/// Whether `general` is a less specific way of saying `specific`.
const fn generalises(general: Role, specific: Role) -> bool {
    matches!(
        (general, specific),
        (Role::Parent, Role::Father | Role::Mother)
            | (Role::Spouse, Role::Husband | Role::Wife)
            | (Role::Other, _)
    )
}

/// Someone who took part in an event, and the facts about the part they played.
#[derive(Debug, Clone, PartialEq)]
pub struct Participation {
    /// Who.
    pub person: PersonId,
    /// In what role, as sources record it. Whether they took part at all is whether this holds.
    pub role: Fact<Role>,
}

/// Something that happened.
///
/// The kind is intrinsic — a baptism record is about a baptism — and everything else is a fact.
/// One marriage with six named participants is one event with six roles, not two people with a
/// marriage field and four orphaned notes.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// This event.
    pub id: EventId,
    /// What kind.
    pub kind: EventKind,
    /// When, as sources record it.
    pub date: Option<Fact<RecordedDate>>,
    /// Where, as sources record it.
    pub place: Option<Fact<PlaceId>>,
    /// Who took part.
    pub participants: Vec<Participation>,
    /// Who removed it, and when.
    pub removed: Option<Provenance>,
}

impl Event {
    /// An event of a kind, with nothing yet recorded about it.
    #[must_use]
    pub const fn new(id: EventId, kind: EventKind) -> Self {
        Self {
            id,
            kind,
            date: None,
            place: None,
            participants: Vec::new(),
            removed: None,
        }
    }

    /// The recorded date, as resolved.
    #[must_use]
    pub fn resolved_date(&self) -> Option<&RecordedDate> {
        self.date.as_ref().and_then(|fact| fact.resolve().value())
    }

    /// The people taking part in a role — resolved, and only where the participation holds.
    pub fn in_role(&self, role: Role) -> impl Iterator<Item = PersonId> + '_ {
        self.participants.iter().filter_map(move |participation| {
            let resolved = participation.role.resolve();
            (resolved.value() == Some(&role)).then_some(participation.person)
        })
    }
}

/// A place is equal to itself and, until the place hierarchy exists, to nothing else. ADR 0010
/// records why that errs in the right direction.
impl Claimable for PlaceId {
    fn compatible(&self, other: &Self) -> bool {
        self == other
    }

    fn more_specific_than(&self, _other: &Self) -> bool {
        false
    }
}

// ── families ────────────────────────────────────────────────────────────────────────────────

/// How a partner is recorded in a union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Partnership {
    /// `HUSB`.
    Husband,
    /// `WIFE`.
    Wife,
    /// A partner, where the record does not say which.
    Partner,
}

impl Claimable for Partnership {
    fn compatible(&self, other: &Self) -> bool {
        self == other || *self == Self::Partner || *other == Self::Partner
    }

    fn more_specific_than(&self, other: &Self) -> bool {
        *other == Self::Partner && *self != Self::Partner
    }
}

/// How a child belongs to a family: GEDCOM 7's `PEDI`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildRelation {
    /// Born to the partners.
    Birth,
    /// `ADOPTED`.
    Adopted,
    /// `FOSTER`.
    Foster,
    /// `SEALING`.
    Sealing,
    /// `OTHER`, described in the source.
    Other,
    /// A child of the family, where the record does not say how.
    Unspecified,
}

impl Claimable for ChildRelation {
    fn compatible(&self, other: &Self) -> bool {
        self == other || *self == Self::Unspecified || *other == Self::Unspecified
    }

    fn more_specific_than(&self, other: &Self) -> bool {
        *other == Self::Unspecified && *self != Self::Unspecified
    }
}

/// A person's belonging to a family, as sources record it.
#[derive(Debug, Clone, PartialEq)]
pub struct Membership<V> {
    /// Who.
    pub person: PersonId,
    /// How, as sources record it. Whether they belong at all is whether this holds.
    pub fact: Fact<V>,
}

/// A family: a union of up to two partners, and the children of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Family {
    /// This family.
    pub id: FamilyId,
    /// The partners.
    pub partners: Vec<Membership<Partnership>>,
    /// The children, each with how they belong.
    pub children: Vec<Membership<ChildRelation>>,
    /// Events of the union itself: engagement, marriage, divorce.
    pub events: Vec<EventId>,
    /// Who removed it, and when.
    pub removed: Option<Provenance>,
}

impl Family {
    /// A family with nobody in it yet.
    #[must_use]
    pub const fn new(id: FamilyId) -> Self {
        Self {
            id,
            partners: Vec::new(),
            children: Vec::new(),
            events: Vec::new(),
            removed: None,
        }
    }

    /// The partners whose membership holds.
    pub fn current_partners(&self) -> impl Iterator<Item = PersonId> + '_ {
        self.partners
            .iter()
            .filter(|membership| membership.fact.holds())
            .map(|membership| membership.person)
    }

    /// The children whose membership holds.
    pub fn current_children(&self) -> impl Iterator<Item = PersonId> + '_ {
        self.children
            .iter()
            .filter(|membership| membership.fact.holds())
            .map(|membership| membership.person)
    }
}

// ── places ──────────────────────────────────────────────────────────────────────────────────

/// What sort of place.
///
/// The Russian administrative levels are named because the records use them and translating them
/// loses the distinction: a *губерния* is not a province in the sense a reader of English
/// assumes, and a *село* and a *деревня* differ in whether they have a church — which decides
/// which parish register a family is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    /// A country.
    Country,
    /// A *губерния*, or a province or state elsewhere.
    Governorate,
    /// An *уезд*, or a district or county.
    Uyezd,
    /// A *волость*.
    Volost,
    /// A city.
    City,
    /// A town.
    Town,
    /// A *село*: a village with a church.
    Selo,
    /// A *деревня*: a village without one.
    Village,
    /// A parish, *приход*.
    Parish,
    /// A church.
    Church,
    /// A cemetery.
    Cemetery,
    /// A street.
    Street,
    /// A building.
    Building,
    /// Something else.
    Other,
}

/// A name a place had, and when.
///
/// Places are renamed, and a record must be shown with the name the place had when the record was
/// made: Тверь was Калинин from 1931 to 1990, and a birth in 1950 happened in Калинин.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceName {
    /// The name.
    pub name: String,
    /// The language it is in, as a BCP 47 tag.
    pub language: Option<String>,
    /// When it was the name. `None` for a name with no known period.
    pub valid: Option<DateValue>,
}

/// A place a place was part of, and when. Borders moved too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceParent {
    /// The containing place.
    pub place: PlaceId,
    /// When it contained this one.
    pub valid: Option<DateValue>,
}

/// A position on the Earth, in ten-millionths of a degree.
///
/// Integers rather than floating point so that a place compares equal to itself after a round
/// trip through storage, and so that the type can be hashed. A ten-millionth of a degree is about
/// a centimetre, which is finer than any record deserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coordinates {
    /// Latitude, positive north.
    pub latitude_e7: i32,
    /// Longitude, positive east.
    pub longitude_e7: i32,
}

impl Coordinates {
    /// From degrees.
    #[must_use]
    pub fn from_degrees(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude_e7: (latitude * 1e7).round() as i32,
            longitude_e7: (longitude * 1e7).round() as i32,
        }
    }

    /// In degrees, as `(latitude, longitude)`.
    #[must_use]
    pub fn degrees(self) -> (f64, f64) {
        (
            f64::from(self.latitude_e7) / 1e7,
            f64::from(self.longitude_e7) / 1e7,
        )
    }
}

/// A place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// This place.
    pub id: PlaceId,
    /// What sort.
    pub kind: Option<PlaceKind>,
    /// Every name it has had.
    pub names: Vec<PlaceName>,
    /// What it has been part of.
    pub within: Vec<PlaceParent>,
    /// Where it is.
    pub position: Option<Coordinates>,
}

// ── evidence ────────────────────────────────────────────────────────────────────────────────

/// Where a source is kept, within its repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallNumber {
    /// A Russian archival reference: *фонд*, *опись*, *дело*. The structure every Russian and
    /// Soviet-era archive uses, kept structured so that "everything from fond 29" can be asked.
    /// Each part is text, because fonds are written *Р-1234* and files *12а*.
    Archival {
        /// *Фонд*.
        fond: String,
        /// *Опись*.
        inventory: String,
        /// *Дело*.
        file: String,
    },
    /// Anything else, as written.
    Free(String),
}

impl CallNumber {
    /// Written as a Russian archive writes it: *ф. 29 оп. 1 д. 204*.
    #[must_use]
    pub fn written(&self) -> String {
        match self {
            Self::Archival {
                fond,
                inventory,
                file,
            } => format!("ф. {fond} оп. {inventory} д. {file}"),
            Self::Free(text) => text.clone(),
        }
    }
}

/// A source as a whole: a register, a census, a letter, a book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// This source.
    pub id: SourceId,
    /// Its title: *Метрическая книга Воскресенской церкви с. Туношна за 1871 г.*
    pub title: String,
    /// Who made it.
    pub author: Option<String>,
    /// Publication details, for a published source.
    pub publication: Option<String>,
    /// Where it is kept.
    pub repository: Option<RepositoryId>,
    /// Where within the repository.
    pub call_number: Option<CallNumber>,
}

/// Where within a source a statement is found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Locator {
    /// A leaf, *лист*, and which side of it. Russian archival files are numbered by leaf, and the
    /// back of a leaf is *оборот* — *л. 17 об.* — so a page number cannot express it.
    Leaf {
        /// The leaf number, as written: *17*, *17а*.
        leaf: String,
        /// Whether it is the back.
        verso: bool,
    },
    /// Anything else, as written: a page, an entry number, a microfilm frame.
    Free(String),
}

impl Locator {
    /// Written as a Russian citation writes it: *л. 17 об.*
    #[must_use]
    pub fn written(&self) -> String {
        match self {
            Self::Leaf { leaf, verso: true } => format!("л. {leaf} об."),
            Self::Leaf { leaf, verso: false } => format!("л. {leaf}"),
            Self::Free(text) => text.clone(),
        }
    }
}

/// A particular place in a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// This citation.
    pub id: CitationId,
    /// The source.
    pub source: SourceId,
    /// Where in it.
    pub locator: Option<Locator>,
    /// The words as read, in the source's own spelling. What a transcription is for.
    pub transcription: Option<String>,
    /// Images of the page.
    pub media: Vec<MediaId>,
}

/// Where sources are kept: an archive, a library, a family's cupboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    /// This repository.
    pub id: RepositoryId,
    /// Its name: *Государственный архив Ярославской области*.
    pub name: String,
    /// Its short name as citations write it: *ГАЯО*.
    pub abbreviation: Option<String>,
    /// Its address.
    pub address: Option<String>,
}

/// A note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// This note.
    pub id: NoteId,
    /// Its text.
    pub text: String,
}

/// A photograph, a scan, a recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Media {
    /// This item.
    pub id: MediaId,
    /// The content address of the file: the hash that names its bytes wherever they are stored.
    pub content: String,
    /// The media type: `image/jpeg`.
    pub mime: String,
    /// A title.
    pub title: Option<String>,
    /// Whether the item was generated rather than recorded. Synthetic portraits exist for the
    /// demonstration tree, and this flag is what keeps them out of a real archive's exports.
    pub synthetic: bool,
}

/// Where a citation is written for a person to read: *ГАЯО ф. 230 оп. 1 д. 1437 л. 17 об.*
#[must_use]
pub fn written_citation(
    repository: Option<&Repository>,
    source: &Source,
    citation: &Citation,
) -> String {
    let archive = repository.and_then(|r| r.abbreviation.as_deref().or(Some(r.name.as_str())));
    [
        archive.map(str::to_owned),
        source.call_number.as_ref().map(CallNumber::written),
        citation.locator.as_ref().map(Locator::written),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use drevigen_date::RecordedDate;

    use super::{
        Attribute, AttributeKind, CallNumber, ChildRelation, Citation, Coordinates, Locator,
        Partnership, Repository, Role, Source, written_citation,
    };
    use crate::claim::Claimable;
    use crate::id::{CitationId, PlaceId, RepositoryId, SourceId};

    #[test]
    fn a_general_role_agrees_with_its_specific_form() {
        assert!(Role::Parent.compatible(&Role::Father));
        assert!(Role::Father.compatible(&Role::Parent));
        assert!(Role::Father.more_specific_than(&Role::Parent));
        assert!(Role::Wife.more_specific_than(&Role::Spouse));
        assert!(!Role::Father.compatible(&Role::Mother));
        assert!(!Role::Godparent.compatible(&Role::Witness));
        assert!(Role::Other.compatible(&Role::Godparent));
        assert!(!Role::Other.more_specific_than(&Role::Godparent));
    }

    #[test]
    fn an_unspecified_relation_agrees_with_any_and_says_less() {
        assert!(ChildRelation::Unspecified.compatible(&ChildRelation::Adopted));
        assert!(ChildRelation::Adopted.more_specific_than(&ChildRelation::Unspecified));
        assert!(!ChildRelation::Birth.compatible(&ChildRelation::Adopted));
        assert!(Partnership::Partner.compatible(&Partnership::Wife));
        assert!(!Partnership::Husband.compatible(&Partnership::Wife));
    }

    fn occupation(value: &str, date: Option<&str>, place: Option<u128>) -> Attribute {
        Attribute {
            kind: AttributeKind::Occupation,
            label: None,
            value: value.to_owned(),
            date: date.map(|d| RecordedDate::parse_gedcom7(d, None).unwrap()),
            place: place.map(PlaceId::from_raw),
        }
    }

    #[test]
    fn an_attribute_with_a_date_refines_one_without() {
        let bare = occupation("крестьянинъ", None, None);
        let dated = occupation("крестьянин", Some("1897"), Some(1));
        assert!(bare.compatible(&dated), "pre-reform spelling is folded");
        assert!(dated.more_specific_than(&bare));
    }

    #[test]
    fn attributes_that_say_different_things_disagree() {
        assert!(
            !occupation("крестьянин", None, None).compatible(&occupation("мещанин", None, None))
        );
        assert!(
            !occupation("крестьянин", Some("1850"), None).compatible(&occupation(
                "крестьянин",
                Some("1897"),
                None
            ))
        );
    }

    #[test]
    fn a_russian_archival_citation_reads_the_way_archives_write_it() {
        let repository = Repository {
            id: RepositoryId::from_raw(1),
            name: "Государственный архив Ярославской области".to_owned(),
            abbreviation: Some("ГАЯО".to_owned()),
            address: None,
        };
        let source = Source {
            id: SourceId::from_raw(1),
            title: "Метрическая книга Воскресенской церкви с. Туношна за 1871 г.".to_owned(),
            author: None,
            publication: None,
            repository: Some(repository.id),
            call_number: Some(CallNumber::Archival {
                fond: "230".to_owned(),
                inventory: "1".to_owned(),
                file: "1437".to_owned(),
            }),
        };
        let citation = Citation {
            id: CitationId::from_raw(1),
            source: source.id,
            locator: Some(Locator::Leaf {
                leaf: "17".to_owned(),
                verso: true,
            }),
            transcription: None,
            media: Vec::new(),
        };
        assert_eq!(
            written_citation(Some(&repository), &source, &citation),
            "ГАЯО ф. 230 оп. 1 д. 1437 л. 17 об."
        );
    }

    #[test]
    fn coordinates_survive_a_round_trip_exactly() {
        let position = Coordinates::from_degrees(57.556_2, 40.078_9);
        let (latitude, longitude) = position.degrees();
        assert_eq!(Coordinates::from_degrees(latitude, longitude), position);
    }
}
