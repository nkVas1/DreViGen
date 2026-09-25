//! Identities.
//!
//! Every entity, fact and claim has a 128-bit identity that any device can mint without asking
//! any other. Two relatives adding people on two phones that have never been online together must
//! not produce the same identity, and a model that needs a server to hand out numbers cannot work
//! offline, which this one must.
//!
//! The layout is **UUID version 7** (RFC 9562): 48 bits of milliseconds since the Unix epoch, then
//! 74 random bits. Two properties follow. Collisions are negligible — 2⁷⁴ possibilities per
//! millisecond. And identities sort by creation time, so inserts land at the right-hand edge of
//! a B-tree instead of scattering across it, which is the difference between an import of fifty
//! thousand people that appends and one that rewrites the index.
//!
//! GEDCOM 7's `UID` structure is a UUID, so these export as they are.
//!
//! # The core does not roll dice
//!
//! [`Id::mint`] takes its randomness as an argument. A native build and a WASM build get entropy
//! differently, and a domain model that reaches for a random number generator is one that cannot
//! be tested deterministically.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::str::FromStr;

/// What kind of thing an identity names. Only ever used as a type parameter.
pub trait Kind {
    /// A short lower-case name, used when an identity is printed for a person to read.
    const NAME: &'static str;
}

/// An identity for one kind of thing.
///
/// The kind is part of the type, so a person's identity cannot be passed where a family's is
/// expected — the mistake is a compile error rather than a corrupted tree.
pub struct Id<K: Kind> {
    raw: u128,
    kind: PhantomData<fn() -> K>,
}

impl<K: Kind> Id<K> {
    /// Wraps a raw 128-bit value, such as one read back from storage.
    #[must_use]
    pub const fn from_raw(raw: u128) -> Self {
        Self {
            raw,
            kind: PhantomData,
        }
    }

    /// The raw value.
    #[must_use]
    pub const fn raw(self) -> u128 {
        self.raw
    }

    /// Mints a new identity from a timestamp and a source of randomness.
    ///
    /// Only the low 74 bits of `random` are used. The caller supplies both, which keeps this
    /// crate free of clocks and entropy and its tests reproducible.
    #[must_use]
    pub const fn mint(millis: u64, random: u128) -> Self {
        let timestamp = (millis as u128 & 0xFFFF_FFFF_FFFF) << 80;
        let version = 0x7 << 76;
        let random_a = ((random >> 62) & 0xFFF) << 64;
        let variant = 0b10 << 62;
        let random_b = random & ((1 << 62) - 1);
        Self::from_raw(timestamp | version | random_a | variant | random_b)
    }

    /// When a version 7 identity was minted, in milliseconds since the Unix epoch.
    ///
    /// `None` for identities of other versions, which arrive from other applications' GEDCOM
    /// `UID`s and carry no time.
    #[must_use]
    pub const fn minted_at(self) -> Option<u64> {
        if self.version() == 7 {
            Some((self.raw >> 80) as u64)
        } else {
            None
        }
    }

    /// The UUID version number.
    #[must_use]
    pub const fn version(self) -> u8 {
        ((self.raw >> 76) & 0xF) as u8
    }

    /// Changes what kind of thing the identity is said to name.
    ///
    /// For storage code that has read an identity from a column whose kind it knows from
    /// context. Anywhere else it defeats the point of having kinds.
    #[must_use]
    pub const fn retype<L: Kind>(self) -> Id<L> {
        Id::from_raw(self.raw)
    }
}

// The derives would demand `K: Clone` and so on, which a marker type has no reason to satisfy.
impl<K: Kind> Clone for Id<K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K: Kind> Copy for Id<K> {}
impl<K: Kind> PartialEq for Id<K> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<K: Kind> Eq for Id<K> {}
impl<K: Kind> PartialOrd for Id<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<K: Kind> Ord for Id<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}
impl<K: Kind> Hash for Id<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<K: Kind> fmt::Display for Id<K> {
    /// The canonical UUID form: `0190c3e4-8f2a-7b31-9d4e-5a6b7c8d9e0f`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = format!("{:032x}", self.raw);
        write!(
            f,
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }
}

impl<K: Kind> fmt::Debug for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{self}", K::NAME)
    }
}

/// Why a string is not an identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseIdError {
    /// Not 32 hexadecimal digits once hyphens are ignored.
    Length,
    /// A character that is neither a hexadecimal digit nor a hyphen.
    Character,
}

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length => f.write_str("an identity is 32 hexadecimal digits"),
            Self::Character => f.write_str("an identity contains only hexadecimal digits"),
        }
    }
}

impl core::error::Error for ParseIdError {}

impl<K: Kind> FromStr for Id<K> {
    type Err = ParseIdError;

    /// Reads a UUID with or without hyphens, in either case — the forms other applications
    /// actually write in a GEDCOM `UID`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut raw: u128 = 0;
        let mut digits = 0;
        for character in text.chars().filter(|c| *c != '-') {
            let value = character.to_digit(16).ok_or(ParseIdError::Character)?;
            digits += 1;
            if digits > 32 {
                return Err(ParseIdError::Length);
            }
            raw = (raw << 4) | u128::from(value);
        }
        if digits == 32 {
            Ok(Self::from_raw(raw))
        } else {
            Err(ParseIdError::Length)
        }
    }
}

macro_rules! kinds {
    ($($(#[$doc:meta])* $marker:ident, $alias:ident, $name:literal;)*) => {
        $(
            $(#[$doc])*
            #[derive(Debug)]
            pub enum $marker {}
            impl Kind for $marker {
                const NAME: &'static str = $name;
            }
            $(#[$doc])*
            pub type $alias = Id<$marker>;
        )*
    };
}

kinds! {
    /// A person.
    PersonKind, PersonId, "person";
    /// A family: a union and the children of it.
    FamilyKind, FamilyId, "family";
    /// Something that happened, with the people who took part.
    EventKind, EventId, "event";
    /// A place.
    PlaceKind, PlaceId, "place";
    /// A source as a whole: a register, a census, a letter.
    SourceKind, SourceId, "source";
    /// A particular location within a source: a page, an entry, a frame.
    CitationKind, CitationId, "citation";
    /// Where a source is kept.
    RepositoryKind, RepositoryId, "repository";
    /// A note.
    NoteKind, NoteId, "note";
    /// A photograph, a scan, a recording.
    MediaKind, MediaId, "media";
    /// One fact about an entity: a name, a date, a membership.
    FactKind, FactId, "fact";
    /// One assertion that a source makes about a fact.
    AssertionKind, AssertionId, "assertion";
    /// One conclusion a researcher reached about a fact.
    ConclusionKind, ConclusionId, "conclusion";
    /// Someone who records things: a relative, a researcher, an importer acting for one.
    ContributorKind, ContributorId, "contributor";
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{FamilyId, ParseIdError, PersonId};

    #[test]
    fn a_minted_identity_is_a_version_seven_uuid() {
        let id = PersonId::mint(1_758_758_400_000, 0x1234_5678_9ABC_DEF0_1234_5678);
        assert_eq!(id.version(), 7);
        // The RFC 9562 variant: the two bits after the third group are 10.
        assert_eq!((id.raw() >> 62) & 0b11, 0b10);
        assert_eq!(id.minted_at(), Some(1_758_758_400_000));
    }

    #[test]
    fn identities_sort_by_when_they_were_minted() {
        // Whatever the randomness, a later millisecond sorts later — which is what keeps a large
        // import appending to an index rather than scattering across it.
        let earlier = PersonId::mint(1_000, u128::MAX);
        let later = PersonId::mint(1_001, 0);
        assert!(earlier < later);
    }

    #[test]
    fn the_text_form_is_the_canonical_uuid() {
        let id = PersonId::from_raw(0x0190_c3e4_8f2a_7b31_9d4e_5a6b_7c8d_9e0f);
        assert_eq!(id.to_string(), "0190c3e4-8f2a-7b31-9d4e-5a6b7c8d9e0f");
        assert_eq!(
            format!("{id:?}"),
            "person:0190c3e4-8f2a-7b31-9d4e-5a6b7c8d9e0f"
        );
    }

    #[test]
    fn other_applications_uids_are_read_however_they_are_written() {
        let canonical: PersonId = "0190c3e4-8f2a-7b31-9d4e-5a6b7c8d9e0f".parse().unwrap();
        let bare: PersonId = "0190C3E48F2A7B319D4E5A6B7C8D9E0F".parse().unwrap();
        assert_eq!(canonical, bare);

        // A version 4 UUID from another application is an identity, just not a dated one.
        let v4: PersonId = "f47ac10b-58cc-4372-a567-0e02b2c3d479".parse().unwrap();
        assert_eq!(v4.version(), 4);
        assert_eq!(v4.minted_at(), None);

        assert_eq!("0190c3e4".parse::<PersonId>(), Err(ParseIdError::Length));
        assert_eq!(
            "0190c3e4-8f2a-7b31-9d4e-5a6b7c8d9e0g".parse::<PersonId>(),
            Err(ParseIdError::Character)
        );
        assert_eq!(
            "0190c3e4-8f2a-7b31-9d4e-5a6b7c8d9e0f00".parse::<PersonId>(),
            Err(ParseIdError::Length)
        );
    }

    #[test]
    fn a_kind_can_be_restated_only_on_purpose() {
        let person = PersonId::mint(5, 5);
        let family: FamilyId = person.retype();
        assert_eq!(person.raw(), family.raw());
    }
}
