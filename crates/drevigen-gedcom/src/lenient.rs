//! Reading the GEDCOM files people actually have.
//!
//! [`Document::parse`] refuses a GEDCOM 7 stream at its first breach, which is right for 7.0 and
//! of no use for the files families hold: 5.5.1 and older, written by programs that each bent the
//! rules their own way. [`read`] builds the same forest of [`Structure`]s from them — converting
//! 5.5.1's notation to 7.0's as it goes, repairing what breaks even 5.5.1's rules — and lists each
//! repair with its line, for the loss report.
//!
//! 5.5.1 is read as the GEDCOM 7 editor's own conversion reads it; his expected outputs in
//! `gedcom7code/test-files` are this module's golden tests:
//!
//! - `CONC` joins a value without a separator, `CONT` with a line break.
//! - `@@` is one `@` anywhere in a value, as 5.5.1 required; a lone `@` is itself.
//! - A calendar escape becomes 7.0's calendar word — `@#DJULIAN@` is `JULIAN`, `@#DFRENCH R@` is
//!   `FRENCH_R` — taking one space after it along. Any other escape is removed, and reported.
//! - Cross-reference identifiers match exactly, spaces and case included: `@test@` does not point
//!   at `@TEST@`.
//! - Whitespace before a level, blank lines and LF CR terminators are read as 5.5.1 told readers
//!   to read them: as nothing.
//!
//! A file whose header declares 7.0 keeps 7.0's notation, in which only a leading `@@` is an
//! escape; everything else here applies to it alike.

use core::fmt;
use std::collections::BTreeSet;

use crate::document::{Document, Payload, Structure, lines};

/// A document read leniently, and what reading it took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// The document.
    pub document: Document,
    /// What was repaired, in line order, up to [`Reading::LISTED_REPAIRS`] of them.
    pub repairs: Vec<Repair>,
    /// How many more repairs there were than are listed.
    pub unlisted_repairs: usize,
}

impl Reading {
    /// How many repairs are listed one by one; past this many, they are only counted.
    pub const LISTED_REPAIRS: usize = 1000;
}

/// One repair, and where it was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repair {
    /// The line, counted as [`crate::encoding::Flaw::line`] counts.
    pub line: usize,
    /// What was repaired.
    pub kind: RepairKind,
}

/// How much a repair changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Only the notation; the meaning is certain.
    Notation,
    /// All the data is there, but where it belongs was a judgement.
    Interpretation,
    /// Something did not survive into the document, and is kept only in the repair.
    Loss,
}

/// What was repaired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairKind {
    /// The file does not begin with a header. Nothing is invented in its place.
    NoHeader,
    /// The file does not end with `TRLR`.
    NoTrailer,
    /// Lines after `TRLR`, read like any others.
    AfterTrailer,
    /// A tag written in small letters, read in capitals. Reported once for each spelling.
    TagCase {
        /// The tag as written.
        written: String,
    },
    /// A line more than one level deeper than the line before it, read as one level deeper.
    LevelJump {
        /// The level written.
        written: usize,
        /// The level it was read at.
        read: usize,
    },
    /// A cross-reference identifier on a substructure, where only a record may have one. Kept.
    XrefOnSubstructure {
        /// The identifier.
        xref: String,
    },
    /// A `CONT` or `CONC` that does not directly continue the value before it, joined to the value
    /// its level points at.
    MisplacedContinuation,
    /// A `CONT` or `CONC` with no value anywhere to continue, kept as a structure of its own.
    OrphanContinuation,
    /// A line that is not a GEDCOM line — most often a note's line that an editor wrapped —
    /// joined to the value before it as a further line.
    StrayLine {
        /// The line.
        text: String,
    },
    /// A line that is not a GEDCOM line and follows nothing it could belong to.
    LostLine {
        /// The line.
        text: String,
    },
    /// Control characters, which no GEDCOM version allows, removed from a line.
    ControlCharacters {
        /// How many.
        count: usize,
    },
    /// A 5.5.1 escape other than a calendar's, which means nothing in 7.0, removed.
    Escape {
        /// The escape as written.
        escape: String,
    },
    /// Two records with one identifier. Both are kept; which one a pointer means is the
    /// importer's question.
    DuplicateXref {
        /// The identifier.
        xref: String,
    },
    /// A pointer to an identifier nothing has.
    DanglingPointer {
        /// The identifier.
        xref: String,
    },
}

impl RepairKind {
    /// How much the repair changed.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        match self {
            Self::NoHeader
            | Self::NoTrailer
            | Self::AfterTrailer
            | Self::TagCase { .. }
            | Self::XrefOnSubstructure { .. } => Severity::Notation,
            Self::LevelJump { .. }
            | Self::MisplacedContinuation
            | Self::OrphanContinuation
            | Self::StrayLine { .. }
            | Self::DuplicateXref { .. } => Severity::Interpretation,
            Self::LostLine { .. }
            | Self::ControlCharacters { .. }
            | Self::Escape { .. }
            | Self::DanglingPointer { .. } => Severity::Loss,
        }
    }
}

impl fmt::Display for Repair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match &self.kind {
            RepairKind::NoHeader => f.write_str("the file does not begin with a header"),
            RepairKind::NoTrailer => f.write_str("the file does not end with TRLR"),
            RepairKind::AfterTrailer => f.write_str("lines after TRLR were read as well"),
            RepairKind::TagCase { written } => write!(f, "tag {written} read in capitals"),
            RepairKind::LevelJump { written, read } => {
                write!(f, "level {written} read as level {read}")
            }
            RepairKind::XrefOnSubstructure { xref } => {
                write!(
                    f,
                    "@{xref}@ is on a substructure; only records have identifiers"
                )
            }
            RepairKind::MisplacedContinuation => {
                f.write_str("a continuation joined to the value its level points at")
            }
            RepairKind::OrphanContinuation => {
                f.write_str("a continuation with no value to continue, kept as it is")
            }
            RepairKind::StrayLine { text } => {
                write!(
                    f,
                    "not a GEDCOM line, joined to the value before it: {text}"
                )
            }
            RepairKind::LostLine { text } => {
                write!(f, "not a GEDCOM line, and nothing to join it to: {text}")
            }
            RepairKind::ControlCharacters { count } => {
                write!(f, "{count} control character(s) removed")
            }
            RepairKind::Escape { escape } => write!(f, "escape {escape} removed"),
            RepairKind::DuplicateXref { xref } => write!(f, "@{xref}@ names two records"),
            RepairKind::DanglingPointer { xref } => write!(f, "nothing is @{xref}@"),
        }
    }
}

/// Why text is not a GEDCOM file at all: its first line is not a GEDCOM record line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotGedcom {
    /// The first line that is not blank, counting from 1.
    pub line: usize,
}

impl fmt::Display for NotGedcom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {} is not a GEDCOM record line, and a GEDCOM file begins with one",
            self.line
        )
    }
}

impl core::error::Error for NotGedcom {}

/// Reads a GEDCOM file of any version that has been decoded to text, repairing what it must.
///
/// # Errors
///
/// [`NotGedcom`] when the first line that is not blank is not a level-0 GEDCOM line — when the
/// text is not a GEDCOM file at all. Anything after that line is read, whatever it is.
pub fn read(text: &str) -> Result<Reading, NotGedcom> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut builder = Builder::new(Dialect::of(text));
    let mut started = false;
    let mut trailer_seen = false;
    let mut after_trailer = false;
    let mut last = 1;

    for (index, raw) in lines(text).enumerate() {
        let number = index + 1;
        let (raw, controls) = without_controls(raw);
        // A line of whitespace is nothing, and so is the 0x1A that DOS programs ended files with.
        if raw.trim().is_empty() {
            continue;
        }
        last = number;
        let parsed = RawLine::parse(&raw);
        if !started {
            if parsed.as_ref().is_none_or(|line| line.level != 0) {
                return Err(NotGedcom { line: number });
            }
            started = true;
        }
        if controls > 0 {
            builder.repair(number, RepairKind::ControlCharacters { count: controls });
        }
        if after_trailer {
            builder.repair(number, RepairKind::AfterTrailer);
            after_trailer = false;
        }
        match parsed {
            Some(line) if line.level == 0 && line.tag.eq_ignore_ascii_case("TRLR") => {
                builder.close_to(0);
                trailer_seen = true;
                after_trailer = true;
            }
            Some(line) => builder.push(&line, &raw, number),
            None => builder.stray(&raw, number),
        }
    }
    if !started {
        return Err(NotGedcom { line: 1 });
    }
    if !trailer_seen {
        builder.repair(last, RepairKind::NoTrailer);
    }
    Ok(builder.finish())
}

/// Levels deeper than this are not levels: 5.5.1 allowed two digits.
const DEEPEST: usize = 99;

/// How many lines of a header to search for the version before assuming 5.5.1.
const HEADER_LINES: usize = 1000;

/// Which notation a file is in: whether `@@` is an escape anywhere, or only at the start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dialect {
    /// 5.5.1 and older: every `@` doubled, escapes such as `@#DJULIAN@`.
    Legacy,
    /// 7.0: only a leading `@` doubled, no escapes.
    Seven,
}

impl Dialect {
    /// The dialect the header's `GEDC.VERS` declares, 5.5.1 when it declares none.
    fn of(text: &str) -> Self {
        let mut records = 0;
        let mut in_gedc = false;
        for raw in lines(text).take(HEADER_LINES) {
            let Some(line) = RawLine::parse(raw) else {
                continue;
            };
            match line.level {
                0 => {
                    records += 1;
                    if records > 1 {
                        break;
                    }
                }
                1 => in_gedc = line.tag.eq_ignore_ascii_case("GEDC"),
                2 if in_gedc && line.tag.eq_ignore_ascii_case("VERS") => {
                    let seven = line.value.is_some_and(|v| v.trim_start().starts_with('7'));
                    return if seven { Self::Seven } else { Self::Legacy };
                }
                _ => {}
            }
        }
        Self::Legacy
    }
}

/// A line as 5.5.1 writes one, read as loosely as its components allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawLine<'a> {
    level: usize,
    xref: Option<&'a str>,
    tag: &'a str,
    /// Everything after the one delimiter that follows the tag.
    value: Option<&'a str>,
}

impl<'a> RawLine<'a> {
    /// Whitespace before the level, any run of spaces or tabs between the components, and an
    /// identifier with spaces in it, as 5.5.1's `pointer_char` allows. `None` for a line that has
    /// no level, or no tag after it.
    fn parse(text: &'a str) -> Option<Self> {
        let text = text.trim_start_matches([' ', '\t']);
        let digits = text.bytes().take_while(u8::is_ascii_digit).count();
        let level = text[..digits].parse().ok()?;
        let rest = delimited(&text[digits..])?;

        let (xref, rest) = match rest.strip_prefix('@') {
            Some(inside) => {
                let close = inside.find('@')?;
                let xref = &inside[..close];
                if xref.is_empty() {
                    return None;
                }
                (Some(xref), delimited(&inside[close + 1..])?)
            }
            None => (None, rest),
        };

        let end = rest.find([' ', '\t']).unwrap_or(rest.len());
        let tag = &rest[..end];
        let plausible = tag.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && tag.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !plausible {
            return None;
        }
        let value = (end < rest.len()).then(|| &rest[end + 1..]);
        Some(Self {
            level,
            xref,
            tag,
            value,
        })
    }
}

/// The rest of a line after a run of at least one space or tab.
fn delimited(text: &str) -> Option<&str> {
    let rest = text.trim_start_matches([' ', '\t']);
    (rest.len() < text.len() && !rest.is_empty()).then_some(rest)
}

/// The line without control characters other than tab, and how many there were.
fn without_controls(raw: &str) -> (std::borrow::Cow<'_, str>, usize) {
    let count = raw.chars().filter(|&c| c.is_control() && c != '\t').count();
    if count == 0 {
        return (raw.into(), 0);
    }
    let kept = raw
        .chars()
        .filter(|&c| !c.is_control() || c == '\t')
        .collect();
    (std::borrow::Cow::Owned(kept), count)
}

/// Builds the forest, repairing as it goes.
struct Builder {
    dialect: Dialect,
    /// The structures still open, outermost first.
    open: Vec<Structure>,
    records: Vec<Structure>,
    repairs: Vec<Repair>,
    /// The tag spellings already reported as written in small letters.
    lower_tags: BTreeSet<String>,
}

impl Builder {
    fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            open: Vec::new(),
            records: Vec::new(),
            repairs: Vec::new(),
            lower_tags: BTreeSet::new(),
        }
    }

    fn repair(&mut self, line: usize, kind: RepairKind) {
        self.repairs.push(Repair { line, kind });
    }

    fn push(&mut self, line: &RawLine<'_>, raw: &str, number: usize) {
        let depth = self.open.len();
        // A "line" that jumps levels and is written in small letters is prose that happens to
        // begin with a number — "1890 was a hard year" — not a structure.
        let jumps = line.level > depth;
        if line.level > DEEPEST || (jumps && line.tag.bytes().any(|b| b.is_ascii_lowercase())) {
            self.stray(raw, number);
            return;
        }

        let tag = self.tag(line.tag, number);
        if tag == "CONT" || tag == "CONC" {
            self.continuation(&tag, line, number);
        } else {
            self.open_structure(tag, line, number);
        }
    }

    /// The tag in capitals, reporting the first time each small-letter spelling is seen.
    fn tag(&mut self, written: &str, number: usize) -> String {
        if !written.bytes().any(|b| b.is_ascii_lowercase()) {
            return written.to_owned();
        }
        if self.lower_tags.insert(written.to_owned()) {
            self.repair(
                number,
                RepairKind::TagCase {
                    written: written.to_owned(),
                },
            );
        }
        written.to_ascii_uppercase()
    }

    fn open_structure(&mut self, tag: String, line: &RawLine<'_>, number: usize) {
        let depth = self.open.len();
        let level = if line.level > depth {
            self.repair(
                number,
                RepairKind::LevelJump {
                    written: line.level,
                    read: depth,
                },
            );
            depth
        } else {
            line.level
        };
        self.close_to(level);

        let xref = line.xref.map(str::to_owned);
        if level > 0
            && let Some(xref) = &xref
        {
            self.repair(
                number,
                RepairKind::XrefOnSubstructure { xref: xref.clone() },
            );
        }
        let payload = self.payload(line.value);
        self.open.push(Structure {
            tag,
            xref,
            payload,
            children: Vec::new(),
            line: number,
        });
    }

    fn payload(&self, value: Option<&str>) -> Payload {
        let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
            return Payload::None;
        };
        if value == "@VOID@" {
            return Payload::Void;
        }
        if let Some(target) = pointer(value) {
            return Payload::Pointer(target.to_owned());
        }
        Payload::Text(match self.dialect {
            Dialect::Seven => undoubled(value).to_owned(),
            // Read once the value is whole: a CONC line can split an escape.
            Dialect::Legacy => value.to_owned(),
        })
    }

    fn continuation(&mut self, tag: &str, line: &RawLine<'_>, number: usize) {
        let depth = self.open.len();
        let proper = line.level == depth
            && self
                .open
                .last()
                .is_some_and(|top| top.children.is_empty() && holds_text(&top.payload));
        let target = if proper {
            Some(depth - 1)
        } else {
            // The value it was meant for: the one its level points at, or else the latest that
            // takes text.
            line.level
                .checked_sub(1)
                .filter(|&index| index < depth && holds_text(&self.open[index].payload))
                .or_else(|| {
                    depth
                        .checked_sub(1)
                        .filter(|&index| holds_text(&self.open[index].payload))
                })
        };
        let Some(target) = target else {
            self.repair(number, RepairKind::OrphanContinuation);
            self.open_structure(tag.to_owned(), line, number);
            return;
        };
        if !proper {
            self.repair(number, RepairKind::MisplacedContinuation);
        }

        let more = line.value.unwrap_or("");
        let more = match self.dialect {
            Dialect::Seven => undoubled(more),
            Dialect::Legacy => more,
        };
        let joiner = if tag == "CONT" { "\n" } else { "" };
        let payload = &mut self.open[target].payload;
        match payload {
            Payload::Text(text) => {
                text.push_str(joiner);
                text.push_str(more);
            }
            _ => *payload = Payload::Text(format!("{joiner}{more}")),
        }
    }

    /// A line that is not a GEDCOM line: a further line of the latest value that takes text, if
    /// there is one; reported either way.
    fn stray(&mut self, raw: &str, number: usize) {
        let text = raw.trim().to_owned();
        match self.open.last_mut() {
            Some(top) if holds_text(&top.payload) => {
                match &mut top.payload {
                    Payload::Text(value) => {
                        value.push('\n');
                        value.push_str(&text);
                    }
                    payload => *payload = Payload::Text(text.clone()),
                }
                self.repair(number, RepairKind::StrayLine { text });
            }
            _ => self.repair(number, RepairKind::LostLine { text }),
        }
    }

    fn close_to(&mut self, level: usize) {
        while self.open.len() > level {
            self.close();
        }
    }

    fn close(&mut self) {
        let Some(mut finished) = self.open.pop() else {
            return;
        };
        if self.dialect == Dialect::Legacy
            && let Payload::Text(raw) = &finished.payload
        {
            let (text, removed) = unescape(raw);
            finished.payload = Payload::Text(text);
            for escape in removed {
                self.repair(finished.line, RepairKind::Escape { escape });
            }
        }
        match self.open.last_mut() {
            Some(parent) => parent.children.push(finished),
            None => self.records.push(finished),
        }
    }

    fn finish(mut self) -> Reading {
        self.close_to(0);
        if self.records.first().is_none_or(|first| first.tag != "HEAD") {
            self.repair(1, RepairKind::NoHeader);
        }
        self.check_references();

        let mut repairs = self.repairs;
        repairs.sort_by_key(|repair| repair.line);
        let unlisted_repairs = repairs.len().saturating_sub(Reading::LISTED_REPAIRS);
        repairs.truncate(Reading::LISTED_REPAIRS);
        Reading {
            document: Document {
                records: self.records,
            },
            repairs,
            unlisted_repairs,
        }
    }

    /// Identifiers used twice, and pointers to identifiers nothing has. Reported, not changed:
    /// what to make of them is the importer's decision.
    fn check_references(&mut self) {
        let mut defined = BTreeSet::new();
        let mut found = Vec::new();
        let mut stack: Vec<&Structure> = self.records.iter().rev().collect();
        while let Some(structure) = stack.pop() {
            if let Some(xref) = &structure.xref
                && !defined.insert(xref.as_str())
            {
                found.push((
                    structure.line,
                    RepairKind::DuplicateXref { xref: xref.clone() },
                ));
            }
            stack.extend(structure.children.iter().rev());
        }
        let mut stack: Vec<&Structure> = self.records.iter().rev().collect();
        while let Some(structure) = stack.pop() {
            if let Payload::Pointer(target) = &structure.payload
                && !defined.contains(target.as_str())
            {
                found.push((
                    structure.line,
                    RepairKind::DanglingPointer {
                        xref: target.clone(),
                    },
                ));
            }
            stack.extend(structure.children.iter().rev());
        }
        for (line, kind) in found {
            self.repair(line, kind);
        }
    }
}

const fn holds_text(payload: &Payload) -> bool {
    matches!(payload, Payload::Text(_) | Payload::None)
}

/// The target of a value that is a pointer and nothing else. 5.5.1 allowed spaces inside one.
fn pointer(value: &str) -> Option<&str> {
    let inner = value.strip_prefix('@')?.strip_suffix('@')?;
    (!inner.is_empty() && !inner.starts_with('#') && !inner.contains('@')).then_some(inner)
}

/// 7.0's one escape: a leading `@@` is `@`.
fn undoubled(value: &str) -> &str {
    match value.strip_prefix('@') {
        Some(rest) if rest.starts_with('@') => rest,
        _ => value,
    }
}

/// 5.5.1's escapes, read as 7.0 reads them. Returns the text, and the escapes that were removed
/// because 7.0 has nothing they could become.
fn unescape(raw: &str) -> (String, Vec<String>) {
    let mut text = String::with_capacity(raw.len());
    let mut removed = Vec::new();
    let mut rest = raw;
    while let Some(at) = rest.find('@') {
        text.push_str(&rest[..at]);
        let tail = &rest[at..];
        if let Some(after) = tail.strip_prefix("@@") {
            text.push('@');
            rest = after;
        } else if let Some(inside) = tail.strip_prefix("@#")
            && let Some(close) = inside.find('@')
        {
            let body = &inside[..close];
            let after = &inside[close + 1..];
            if let Some(calendar) = body.strip_prefix(['D', 'd']) {
                text.push_str(&calendar.replace(' ', "_"));
                text.push(' ');
            } else {
                removed.push(format!("@#{body}@"));
            }
            rest = after.strip_prefix(' ').unwrap_or(after);
        } else {
            text.push('@');
            rest = &tail[1..];
        }
    }
    text.push_str(rest);
    (text, removed)
}

#[cfg(test)]
mod tests;
