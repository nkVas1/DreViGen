//! Importing a GEDCOM file: decode it, read it, map it, and say how it went.
//!
//! The result is an archive of its own — what the file holds, and nothing else — so that the
//! person importing can look at it, and at the report, before any of it is merged into a tree
//! they already have.

mod mapper;
mod names;
mod report;

use drevigen_core::{Archive, ContributorId, SourceId, Timestamp};
use drevigen_gedcom::encoding;
use drevigen_gedcom::lenient::{self, NotGedcom};

pub use report::{Counts, Entry, Mapping, Report, What};

/// What an import needs from its caller: who is importing, when, and what the file is called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The file's name, which becomes its source's title: *Смирновы.ged*.
    pub file_name: String,
    /// Who is importing. Every assertion is theirs.
    pub by: ContributorId,
    /// When. Every assertion carries it, and every identity minted is stamped with it.
    pub at: Timestamp,
}

/// A file, imported.
#[derive(Debug, Clone)]
pub struct Import {
    /// What the file holds, as an archive of its own.
    pub archive: Archive,
    /// The source the file became, which every value it gives without a citation cites.
    pub source: SourceId,
    /// How it went.
    pub report: Report,
}

/// Imports a GEDCOM file of any version and encoding into a new archive.
///
/// `entropy` supplies the random bits of each new identity; the caller's clock is
/// [`Options::at`]. Neither is reached for here, which keeps an import reproducible in tests.
///
/// # Errors
///
/// [`NotGedcom`] when the bytes are not a GEDCOM file at all. Anything that is one imports,
/// however damaged; the damage is in the report.
pub fn import(
    bytes: &[u8],
    options: &Options,
    entropy: impl FnMut() -> u128,
) -> Result<Import, NotGedcom> {
    let decoded = encoding::decode(bytes);
    let reading = lenient::read(&decoded.text)?;
    let mut report = Report::new(&decoded, &reading);
    let (archive, source) = mapper::map(&reading.document, options, entropy, &mut report);
    report.finish();
    Ok(Import {
        archive,
        source,
        report,
    })
}

#[cfg(test)]
mod tests;
