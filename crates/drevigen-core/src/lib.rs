//! The domain model.
//!
//! People, families, events and places — held not as fields but as **claims**: what sources say
//! about each fact, and what a researcher concluded. That split is the project's central
//! structural decision ([ADR 0005]), and [ADR 0010] settles what it means in practice: claims
//! attach to facts, a fact's claims are typed, and two assertions agree when both could be true,
//! not only when they are identical.
//!
//! What lives here is the model and its rules. What does not: storage, which is
//! `drevigen-store`; interchange, which is `drevigen-gedcom`; and anything that needs a clock or a
//! random number generator, which the caller supplies so that this crate stays deterministic and
//! its tests reproducible.
//!
//! [ADR 0005]: https://github.com/nkVas1/DreViGen/blob/main/docs/02-architecture/adr/0005-assertion-conclusion-split.md
//! [ADR 0010]: https://github.com/nkVas1/DreViGen/blob/main/docs/02-architecture/adr/0010-claims-per-fact-agreement-is-compatibility.md

pub mod claim;
pub mod id;
pub mod name;
pub mod value;

pub use claim::{
    Assertion, ClaimError, Claimable, Claims, Conclusion, Confidence, Provenance, Resolution,
    Timestamp,
};
pub use id::{
    AssertionId, CitationId, ConclusionId, ContributorId, EventId, FactId, FamilyId, Id, Kind,
    MediaId, NoteId, ParseIdError, PersonId, PlaceId, RepositoryId, SourceId,
};
pub use name::{NameKind, NameOrder, PersonName, fold};
pub use value::Sex;
