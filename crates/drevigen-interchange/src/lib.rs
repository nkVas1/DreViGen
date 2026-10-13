//! GEDCOM files into the archive, and back.
//!
//! `drevigen-gedcom` reads a file of any version and encoding into a forest of structures. This
//! crate maps those structures into the archive's entities and claims, under the rules of
//! [ADR 0011]:
//!
//! - **Every value is an assertion** by the person importing, at the moment of import, cited by
//!   the citations the file gives for it — or, where it gives none, by the file itself, which
//!   becomes a source: a cousin's tree is a claim with a provenance, not a memory.
//! - **Nothing is dropped.** Each mapping matches the substructures it understands, and whatever
//!   it does not is kept verbatim as foreign data under the nearest subject, to be written back on
//!   export. Mapping more of GEDCOM later only shrinks what is kept that way.
//! - **One report** says what decoding, reading and mapping could not keep exactly, line by line.
//!
//! [`export()`] writes an archive back out as GEDCOM 7, foreign data in the places it was found.
//!
//! [ADR 0011]: https://github.com/nkVas1/DreViGen/blob/main/docs/02-architecture/adr/0011-import-keeps-everything.md

pub mod export;
pub mod import;

pub use export::{Export, export};
pub use import::{Import, Options, Report, import};
