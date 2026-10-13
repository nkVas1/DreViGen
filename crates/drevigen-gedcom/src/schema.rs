//! GEDCOM 7's rules beyond the container: which structures may appear where, how many of each a
//! structure may have, and what each one's payload must be. The container — levels, tags,
//! cross-references that resolve — is [`mod@crate::document`]'s; this is the rest of what makes a
//! document valid GEDCOM 7.
//!
//! The rules are the specification's own, from the files FamilySearch extracts from it and
//! publishes beside it (`tools/generate-gedcom7-schema.mjs` turns them into tables). The payload
//! grammars are the specification's ABNF, each implemented beside the production it implements.
//!
//! An extension structure — a tag beginning `_` — may appear anywhere, and what is beneath it is
//! its extension's business: nothing below an extension tag is checked. A dataset that documents
//! its extensions in `HEAD.SCHMA` is not read for what they mean.

mod grammar;
mod tables;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fmt;

use crate::document::{Document, Payload, Structure};

/// How many of a substructure a structure may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cardinality {
    /// At least one.
    pub required: bool,
    /// More than one.
    pub many: bool,
}

/// What a payload must be: one of GEDCOM 7's data types, or none at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// No payload.
    None,
    /// Free text.
    Text,
    /// `Y` or nothing: an event known to have happened, or one only discussed.
    Y,
    /// A pointer to a record with this tag, or `@VOID@`.
    Pointer(&'static str),
    /// A value from the structure's enumeration set, or an extension value.
    Enum,
    /// Enumeration values separated by commas.
    EnumList,
    /// Text values separated by commas.
    TextList,
    /// A non-negative integer.
    Integer,
    /// A `DateValue`: a date, a range, a period, an approximation, or nothing.
    Date,
    /// A `DateExact`: day, month and year, in the Gregorian calendar.
    DateExact,
    /// A `DatePeriod`: from, to, both, or nothing.
    DatePeriod,
    /// A time of day.
    Time,
    /// An age, in years, months, weeks and days.
    Age,
    /// A personal name, its surname between slashes.
    PersonalName,
    /// A BCP 47 language tag.
    Language,
    /// A media type.
    MediaType,
    /// A file's location, as a URL.
    FilePath,
    /// A URI reference.
    Uri,
    /// An extension tag and the URI it stands for.
    TagDefinition,
    /// `N` or `S`, and degrees.
    Latitude,
    /// `E` or `W`, and degrees.
    Longitude,
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("no payload"),
            Self::Text => f.write_str("text"),
            Self::Y => f.write_str("Y or nothing"),
            Self::Pointer(record) => write!(f, "a pointer to a {record} record"),
            Self::Enum => f.write_str("an enumeration value"),
            Self::EnumList => f.write_str("a list of enumeration values"),
            Self::TextList => f.write_str("a list of text"),
            Self::Integer => f.write_str("a non-negative integer"),
            Self::Date => f.write_str("a date"),
            Self::DateExact => f.write_str("an exact date"),
            Self::DatePeriod => f.write_str("a date period"),
            Self::Time => f.write_str("a time"),
            Self::Age => f.write_str("an age"),
            Self::PersonalName => f.write_str("a personal name"),
            Self::Language => f.write_str("a language tag"),
            Self::MediaType => f.write_str("a media type"),
            Self::FilePath => f.write_str("a file path"),
            Self::Uri => f.write_str("a URI"),
            Self::TagDefinition => f.write_str("an extension tag and a URI"),
            Self::Latitude => f.write_str("a latitude"),
            Self::Longitude => f.write_str("a longitude"),
        }
    }
}

/// Something a document does that GEDCOM 7 does not allow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The line of the structure at fault; for a missing substructure, of the structure it is
    /// missing from.
    pub line: usize,
    /// What is wrong.
    pub kind: ViolationKind,
}

/// What is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationKind {
    /// A standard tag the specification does not place here.
    NotAllowed {
        /// The tag.
        tag: String,
        /// The tag of the structure it is in; `None` for a record.
        within: Option<String>,
    },
    /// A substructure the specification requires is absent.
    Missing {
        /// Its tag.
        tag: String,
        /// The tag of the structure it is missing from.
        within: String,
    },
    /// A second of a substructure that may appear once.
    Repeated {
        /// Its tag.
        tag: String,
        /// The tag of the structure it is in.
        within: String,
    },
    /// A payload that is not of the structure's data type.
    Payload {
        /// The structure's tag.
        tag: String,
        /// What the payload must be.
        expected: DataType,
    },
    /// A pointer to a record of another kind.
    PointsAt {
        /// The structure's tag.
        tag: String,
        /// The tag of the record it must point at.
        expected: &'static str,
        /// The tag of the record it does point at.
        found: String,
    },
    /// An enumeration value that is not one of the structure's.
    NotInSet {
        /// The structure's tag.
        tag: String,
        /// The value.
        value: String,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match &self.kind {
            ViolationKind::NotAllowed { tag, within } => match within {
                Some(within) => write!(f, "{tag} is not allowed in {within}"),
                None => write!(f, "{tag} is not a record"),
            },
            ViolationKind::Missing { tag, within } => write!(f, "{within} must have {tag}"),
            ViolationKind::Repeated { tag, within } => {
                write!(f, "{within} may have only one {tag}")
            }
            ViolationKind::Payload { tag, expected } => {
                write!(f, "the payload of {tag} must be {expected}")
            }
            ViolationKind::PointsAt {
                tag,
                expected,
                found,
            } => write!(f, "{tag} must point at a {expected} record, not {found}"),
            ViolationKind::NotInSet { tag, value } => write!(f, "{value} is not a value of {tag}"),
        }
    }
}

/// Every way the document departs from GEDCOM 7's rules, in document order. Empty for a valid
/// document.
#[must_use]
pub fn validate(document: &Document) -> Vec<Violation> {
    let mut checker = Checker {
        records: document
            .records
            .iter()
            .filter_map(|record| Some((record.xref.as_deref()?, record.tag.as_str())))
            .collect(),
        found: Vec::new(),
    };
    for record in &document.records {
        checker.placed(record, "", None);
    }
    checker.found
}

struct Checker<'d> {
    /// Each record's tag, by cross-reference identifier.
    records: BTreeMap<&'d str, &'d str>,
    found: Vec<Violation>,
}

impl<'d> Checker<'d> {
    /// A structure in its place: of the type the specification gives its tag there, or not
    /// allowed there at all. Returns its type.
    fn placed(
        &mut self,
        structure: &'d Structure,
        within: &'static str,
        parent: Option<&Structure>,
    ) -> Option<&'static str> {
        if structure.tag.starts_with('_') {
            return None;
        }
        let Some(kind) = placement(within, &structure.tag) else {
            self.found.push(Violation {
                line: structure.line,
                kind: ViolationKind::NotAllowed {
                    tag: structure.tag.clone(),
                    within: parent.map(|parent| parent.tag.clone()),
                },
            });
            return None;
        };
        self.payload(structure, kind);
        self.substructures(structure, kind);
        Some(kind)
    }

    /// The substructures of a structure of a known type, and how many there are of each.
    fn substructures(&mut self, structure: &'d Structure, kind: &'static str) {
        let mut counts: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
        for child in &structure.children {
            if let Some(child_kind) = self.placed(child, kind, Some(structure)) {
                let (count, second) = counts.entry(child_kind).or_insert((0, 0));
                *count += 1;
                if *count == 2 {
                    *second = child.line;
                }
            }
        }
        for &(_, child_kind, tag, cardinality) in cardinalities(kind) {
            let (count, second) = counts.get(child_kind).copied().unwrap_or_default();
            if cardinality.required && count == 0 {
                self.found.push(Violation {
                    line: structure.line,
                    kind: ViolationKind::Missing {
                        tag: tag.to_owned(),
                        within: structure.tag.clone(),
                    },
                });
            }
            if !cardinality.many && count > 1 {
                self.found.push(Violation {
                    line: second,
                    kind: ViolationKind::Repeated {
                        tag: tag.to_owned(),
                        within: structure.tag.clone(),
                    },
                });
            }
        }
    }

    fn payload(&mut self, structure: &Structure, kind: &'static str) {
        let Some(expected) = data_type(kind) else {
            return;
        };
        let wrong = |found: &mut Vec<Violation>| {
            found.push(Violation {
                line: structure.line,
                kind: ViolationKind::Payload {
                    tag: structure.tag.clone(),
                    expected,
                },
            });
        };
        let text = match (&structure.payload, expected) {
            (Payload::Void, DataType::Pointer(_)) => return,
            (Payload::Pointer(target), DataType::Pointer(record)) => {
                // A pointer to nothing is the container's to report.
                if let Some(&found) = self.records.get(target.as_str())
                    && found != record
                {
                    self.found.push(Violation {
                        line: structure.line,
                        kind: ViolationKind::PointsAt {
                            tag: structure.tag.clone(),
                            expected: record,
                            found: found.to_owned(),
                        },
                    });
                }
                return;
            }
            (Payload::Pointer(_) | Payload::Void, _) | (_, DataType::Pointer(_)) => {
                return wrong(&mut self.found);
            }
            (Payload::None, _) => "",
            (Payload::Text(text), _) => text.as_str(),
        };
        if !grammar::accepts(expected, text) {
            return wrong(&mut self.found);
        }
        if matches!(expected, DataType::Enum | DataType::EnumList)
            && let Some(set) = enumeration(kind)
        {
            for value in grammar::list_items(text) {
                if !value.starts_with('_') && !in_set(set, value) {
                    self.found.push(Violation {
                        line: structure.line,
                        kind: ViolationKind::NotInSet {
                            tag: structure.tag.clone(),
                            value: value.to_owned(),
                        },
                    });
                }
            }
        }
    }
}

/// The type of the structure a standard tag is in this place, if the specification places it.
fn placement(within: &str, tag: &str) -> Option<&'static str> {
    let index = tables::SUBSTRUCTURES
        .binary_search_by(|&(place, written, _)| (place, written).cmp(&(within, tag)))
        .ok()?;
    Some(tables::SUBSTRUCTURES[index].2)
}

/// Every substructure a structure of this type has a cardinality for.
fn cardinalities(kind: &str) -> &'static [(&'static str, &'static str, &'static str, Cardinality)] {
    let rows = &tables::CARDINALITIES;
    let start = rows.partition_point(|row| row.0 < kind);
    let end = rows.partition_point(|row| row.0 <= kind);
    &rows[start..end]
}

fn data_type(kind: &str) -> Option<DataType> {
    let rows = &tables::PAYLOADS;
    let index = rows.binary_search_by(|row| row.0.cmp(kind)).ok()?;
    Some(rows[index].1)
}

fn enumeration(kind: &str) -> Option<&'static str> {
    let rows = &tables::ENUMERATIONS;
    let index = rows.binary_search_by(|row| row.0.cmp(kind)).ok()?;
    Some(rows[index].1)
}

fn in_set(set: &str, value: &str) -> bool {
    tables::SET_VALUES
        .binary_search_by(|&(found, tag)| (found, tag).cmp(&(set, value)))
        .is_ok()
}
