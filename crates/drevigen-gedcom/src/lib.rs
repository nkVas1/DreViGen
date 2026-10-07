//! GEDCOM: the format every genealogy program reads and writes, and the one a family's existing
//! work arrives in.
//!
//! The obligation, from `docs/01-research/data-standards.md`: read GEDCOM 7 exactly, write it
//! exactly, and read everything older — 5.5.1 and 5.5, in whatever encoding and dialect a program
//! produced — without losing a line. What cannot be mapped onto the domain model is kept and
//! reported, never silently dropped.
//!
//! The rules implemented here are read from the specification source at
//! `FamilySearch/GEDCOM@512e38d`, not recalled; where memory and the specification disagreed, the
//! specification is quoted beside the code.
//!
//! This crate is built in layers, lowest first:
//!
//! - [`mod@encoding`] — bytes into text: which encoding a file is in, declared or not, and what
//!   could not be decoded.
//! - [`mod@line`] — one line: level, cross-reference, tag, value.
//! - [`mod@document`] — lines into a forest of structures and back, `CONT` included.

pub mod document;
pub mod encoding;
pub mod line;

pub use document::{Document, DocumentError, DocumentErrorKind, Payload, Structure};
pub use encoding::{Decoded, Encoding};
pub use line::{Line, LineError, Value};
