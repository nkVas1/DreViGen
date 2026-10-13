//! What an import says about itself.

use core::fmt;
use std::collections::BTreeMap;

use drevigen_gedcom::Encoding;
use drevigen_gedcom::encoding::{Basis, Decoded, Flaw, FlawKind};
use drevigen_gedcom::lenient::{Reading, Repair, RepairKind, Severity};

/// How an import went, for the person importing to read before accepting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The encoding the file was read in.
    pub encoding: Encoding,
    /// How that encoding was settled.
    pub basis: Basis,
    /// What the header said the encoding was, if it said.
    pub declared_encoding: Option<String>,
    /// Whether the bytes overruled that declaration.
    pub encoding_overruled: bool,
    /// The GEDCOM version the header declares: `5.5.1`, `7.0`.
    pub version: Option<String>,
    /// The program that wrote the file, as its header names it.
    pub program: Option<String>,
    /// What could not be kept exactly, in line order.
    pub entries: Vec<Entry>,
    /// Entries the layers below counted but did not list, past their limits.
    pub unlisted: usize,
    /// What was kept as foreign data, counted by where it was found: `INDI._MILT`.
    pub kept: BTreeMap<String, usize>,
    /// What the archive now holds.
    pub counts: Counts,
}

/// One thing that could not be kept exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The line of the file it concerns.
    pub line: usize,
    /// What it was, and which layer found it.
    pub what: What,
}

/// Which layer found it, and what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum What {
    /// Decoding the bytes.
    Decoding(FlawKind),
    /// Reading the lines into structures.
    Reading(RepairKind),
    /// Mapping the structures into the archive.
    Mapping(Mapping),
}

impl What {
    /// How much it changed.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        match self {
            Self::Decoding(FlawKind::LoneMark(_)) => Severity::Interpretation,
            Self::Decoding(_) => Severity::Loss,
            Self::Reading(kind) => kind.severity(),
            Self::Mapping(mapping) => mapping.severity(),
        }
    }
}

/// What mapping could not take as it stood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mapping {
    /// A name with nothing in it, which records nothing.
    EmptyName,
    /// A sex none of GEDCOM's values; kept as foreign data.
    UnknownSex {
        /// The value as written.
        value: String,
    },
    /// A pointer to nothing the file has, or to a record of a kind that cannot be there; the
    /// structure holding it is kept as foreign data.
    Unresolved {
        /// The identifier pointed at.
        xref: String,
    },
    /// A child or partner the file records on one side of the link only — in the family but not
    /// in the person's record, or the other way round — read as if both sides recorded it.
    OneSidedLink {
        /// The family's identifier.
        family: String,
    },
    /// A record whose identifier an earlier record already has. Imported as an entity of its own,
    /// which no pointer reaches: pointers go to the first.
    SecondRecord {
        /// The identifier.
        xref: String,
    },
    /// A source with no title, given its abbreviation as one where it has one.
    UntitledSource,
    /// The archive refused an edit the import made: a defect in the importer, reported rather
    /// than hidden.
    Refused {
        /// The refusal, as the archive put it.
        refusal: String,
    },
}

impl Mapping {
    /// How much it changed.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        match self {
            Self::EmptyName | Self::UntitledSource => Severity::Notation,
            Self::UnknownSex { .. }
            | Self::Unresolved { .. }
            | Self::OneSidedLink { .. }
            | Self::SecondRecord { .. } => Severity::Interpretation,
            Self::Refused { .. } => Severity::Loss,
        }
    }
}

/// What an archive holds after an import.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// People.
    pub people: usize,
    /// Families.
    pub families: usize,
    /// Events.
    pub events: usize,
    /// Places.
    pub places: usize,
    /// Sources, the file's own included.
    pub sources: usize,
    /// Citations.
    pub citations: usize,
    /// Repositories.
    pub repositories: usize,
    /// Notes.
    pub notes: usize,
    /// Foreign items.
    pub foreign: usize,
}

impl Report {
    /// A report holding what decoding and reading found; mapping adds the rest.
    pub(super) fn new(decoded: &Decoded, reading: &Reading) -> Self {
        let decoding = decoded.flaws.iter().map(|flaw| Entry {
            line: flaw.line,
            what: What::Decoding(flaw.kind.clone()),
        });
        let repairs = reading.repairs.iter().map(|repair| Entry {
            line: repair.line,
            what: What::Reading(repair.kind.clone()),
        });
        Self {
            encoding: decoded.encoding,
            basis: decoded.basis,
            declared_encoding: decoded.declared.clone(),
            encoding_overruled: decoded.overrules_declaration(),
            version: None,
            program: None,
            entries: decoding.chain(repairs).collect(),
            unlisted: decoded.unlisted_flaws + reading.unlisted_repairs,
            kept: BTreeMap::new(),
            counts: Counts::default(),
        }
    }

    pub(super) fn note(&mut self, line: usize, mapping: Mapping) {
        self.entries.push(Entry {
            line,
            what: What::Mapping(mapping),
        });
    }

    pub(super) fn finish(&mut self) {
        self.entries.sort_by_key(|entry| entry.line);
    }

    /// The entries of a severity and worse.
    pub fn at_least(&self, severity: Severity) -> impl Iterator<Item = &Entry> {
        self.entries
            .iter()
            .filter(move |entry| entry.what.severity() >= severity)
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.what {
            What::Decoding(kind) => Flaw {
                line: self.line,
                kind: kind.clone(),
            }
            .fmt(f),
            What::Reading(kind) => Repair {
                line: self.line,
                kind: kind.clone(),
            }
            .fmt(f),
            What::Mapping(mapping) => {
                write!(f, "line {}: ", self.line)?;
                match mapping {
                    Mapping::EmptyName => f.write_str("a name with nothing in it"),
                    Mapping::UnknownSex { value } => {
                        write!(f, "sex {value} is none of GEDCOM's; kept as it was")
                    }
                    Mapping::Unresolved { xref } => {
                        write!(f, "@{xref}@ is nothing that can be here; kept as it was")
                    }
                    Mapping::OneSidedLink { family } => {
                        write!(f, "a link to @{family}@ recorded on one side only")
                    }
                    Mapping::SecondRecord { xref } => {
                        write!(f, "a second record with @{xref}@, which no pointer reaches")
                    }
                    Mapping::UntitledSource => f.write_str("a source with no title"),
                    Mapping::Refused { refusal } => write!(f, "the archive refused: {refusal}"),
                }
            }
        }
    }
}
