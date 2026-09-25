//! Lines into structures, and structures back into lines.
//!
//! A GEDCOM document is a forest of structures: each record at level 0, each substructure one
//! level below the structure it belongs to. This module builds that forest from lines and writes
//! it back, knowing nothing about what any tag means — that is the job of the layers above. What it
//! does know is the container: levels, cross-references, pointers, and how a multi-line value is
//! split across `CONT` lines.
//!
//! Everything a structure held is kept, including tags nothing above understands. An extension
//! the importer cannot map is still a structure here, which is how it survives to be written back
//! out — the "verbatim preservation of unrepresentable structures" the roadmap asks for starts at
//! this layer, by never dropping one.

use std::collections::BTreeSet;

use crate::line::{Line, LineError, Value, is_banned};

/// What a structure's value is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Payload {
    /// No value. GEDCOM 7 treats an empty value and a missing one as the same.
    #[default]
    None,
    /// A pointer to another structure, by its cross-reference identifier without the `@`s.
    Pointer(String),
    /// `@VOID@`.
    Void,
    /// Text, with any `CONT` lines already joined by `\n`.
    Text(String),
}

impl Payload {
    /// The text, if this is text.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The target, if this is a pointer to something.
    #[must_use]
    pub fn pointer(&self) -> Option<&str> {
        match self {
            Self::Pointer(target) => Some(target),
            _ => None,
        }
    }
}

/// One structure and everything beneath it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structure {
    /// The tag.
    pub tag: String,
    /// The cross-reference identifier, without its `@`s. Records only.
    pub xref: Option<String>,
    /// The value.
    pub payload: Payload,
    /// The substructures, in order.
    pub children: Vec<Structure>,
    /// The line number it began on, counting from 1; `0` for a structure built in memory. Kept so
    /// that anything said about it later can say where.
    pub line: usize,
}

impl Structure {
    /// A structure with a tag and nothing else.
    #[must_use]
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_owned(),
            xref: None,
            payload: Payload::None,
            children: Vec::new(),
            line: 0,
        }
    }

    /// The same, with a text value.
    #[must_use]
    pub fn with_text(mut self, text: &str) -> Self {
        self.payload = Payload::Text(text.to_owned());
        self
    }

    /// The same, with a substructure appended.
    #[must_use]
    pub fn with_child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }

    /// The first substructure with a tag.
    #[must_use]
    pub fn child(&self, tag: &str) -> Option<&Self> {
        self.children.iter().find(|child| child.tag == tag)
    }

    /// Every substructure with a tag.
    pub fn children_tagged<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = &'a Self> + 'a {
        self.children.iter().filter(move |child| child.tag == tag)
    }
}

/// A whole data stream: the header first, then the records. The trailer is implied — it carries
/// nothing, so it is written rather than stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The header and every record, in file order. The header is the first.
    pub records: Vec<Structure>,
}

/// Why a data stream is not a GEDCOM 7 document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentError {
    /// What is wrong.
    pub kind: DocumentErrorKind,
    /// The line number, counting from 1.
    pub line: usize,
}

/// What is wrong with a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentErrorKind {
    /// A line does not match the line grammar.
    Line(LineError),
    /// A character GEDCOM 7 bans anywhere in a stream.
    BannedCharacter(char),
    /// A line is more than one level deeper than the line before it.
    LevelJump,
    /// A `CONT` that does not immediately continue a text value.
    MisplacedContinuation,
    /// `CONC`, which GEDCOM 7 removed.
    Concatenation,
    /// A substructure with a cross-reference identifier; only records may have one.
    XrefOnSubstructure,
    /// Two structures with the same cross-reference identifier.
    DuplicateXref(String),
    /// A pointer to an identifier no structure has.
    DanglingPointer(String),
    /// The stream does not begin with `HEAD`.
    NoHeader,
    /// The stream does not end with `TRLR`, or has something after it.
    NoTrailer,
}

impl core::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match &self.kind {
            DocumentErrorKind::Line(error) => write!(f, "{error}"),
            DocumentErrorKind::BannedCharacter(c) => {
                write!(f, "U+{:04X} is not allowed in GEDCOM 7", u32::from(*c))
            }
            DocumentErrorKind::LevelJump => {
                f.write_str("more than one level deeper than the line before")
            }
            DocumentErrorKind::MisplacedContinuation => {
                f.write_str("CONT must immediately continue a text value")
            }
            DocumentErrorKind::Concatenation => f.write_str("CONC does not exist in GEDCOM 7"),
            DocumentErrorKind::XrefOnSubstructure => {
                f.write_str("only a record may have a cross-reference identifier")
            }
            DocumentErrorKind::DuplicateXref(xref) => write!(f, "@{xref}@ is used twice"),
            DocumentErrorKind::DanglingPointer(xref) => write!(f, "nothing is @{xref}@"),
            DocumentErrorKind::NoHeader => f.write_str("the document does not begin with HEAD"),
            DocumentErrorKind::NoTrailer => f.write_str("the document does not end with TRLR"),
        }
    }
}

impl core::error::Error for DocumentError {}

impl Document {
    /// Reads a GEDCOM 7 data stream that has already been decoded to text.
    ///
    /// Strict: every rule of the container format is enforced, and the first breach is returned
    /// with its line number. For older and messier files, decode and read through the lenient
    /// reader instead.
    ///
    /// # Errors
    ///
    /// A [`DocumentError`] for the first line that breaks a rule.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);

        let mut builder = Builder::default();
        let mut trailer_at = None;

        // A stream that ends with a terminator yields one empty piece after it, which is not a
        // line. Only that one: an empty line anywhere else is an error the grammar reports.
        let mut pieces: Vec<&str> = lines(text).collect();
        if text.ends_with(['\n', '\r']) && pieces.last() == Some(&"") {
            pieces.pop();
        }

        for (index, raw) in pieces.into_iter().enumerate() {
            let number = index + 1;
            let fail = |kind| Err(DocumentError { kind, line: number });

            if let Some(banned) = raw.chars().find(|c| is_banned(*c)) {
                return fail(DocumentErrorKind::BannedCharacter(banned));
            }
            if trailer_at.is_some() {
                return fail(DocumentErrorKind::NoTrailer);
            }

            let line = Line::parse(raw).map_err(|error| DocumentError {
                kind: DocumentErrorKind::Line(error),
                line: number,
            })?;

            if line.level == 0 && line.tag == "TRLR" {
                trailer_at = Some(number);
                continue;
            }
            builder.push(&line, number)?;
        }

        let records = builder.finish();
        if records.first().map(|r| r.tag.as_str()) != Some("HEAD") {
            return Err(DocumentError {
                kind: DocumentErrorKind::NoHeader,
                line: 1,
            });
        }
        if trailer_at.is_none() {
            return Err(DocumentError {
                kind: DocumentErrorKind::NoTrailer,
                line: text.lines().count().max(1),
            });
        }

        let document = Self { records };
        document.check_references()?;
        Ok(document)
    }

    /// Every cross-reference identifier must be unique, and every pointer must land somewhere.
    fn check_references(&self) -> Result<(), DocumentError> {
        let mut defined = BTreeSet::new();
        for record in &self.records {
            if let Some(xref) = &record.xref
                && !defined.insert(xref.as_str())
            {
                return Err(DocumentError {
                    kind: DocumentErrorKind::DuplicateXref(xref.clone()),
                    line: record.line,
                });
            }
        }
        let mut stack: Vec<&Structure> = self.records.iter().collect();
        while let Some(structure) = stack.pop() {
            if let Payload::Pointer(target) = &structure.payload
                && !defined.contains(target.as_str())
            {
                return Err(DocumentError {
                    kind: DocumentErrorKind::DanglingPointer(target.clone()),
                    line: structure.line,
                });
            }
            stack.extend(&structure.children);
        }
        Ok(())
    }

    /// Writes the document as GEDCOM 7: a byte-order mark, one line per structure and `CONT` line,
    /// `\n` terminators, and the trailer.
    ///
    /// A value's line breaks become `CONT` lines, and a line string that begins with `@` has it
    /// doubled — only there, as 7.0 requires.
    #[must_use]
    pub fn write(&self) -> String {
        let mut out = String::from('\u{FEFF}');
        for record in &self.records {
            write_structure(&mut out, record, 0);
        }
        out.push_str("0 TRLR\n");
        out
    }
}

/// Splits on the three terminators GEDCOM 7 allows: CR LF, CR, and LF.
fn lines(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = Some(text);
    core::iter::from_fn(move || {
        let current = rest?;
        let Some(end) = current.find(['\r', '\n']) else {
            rest = None;
            return Some(current);
        };
        let skip = if current[end..].starts_with("\r\n") {
            2
        } else {
            1
        };
        rest = Some(&current[end + skip..]);
        Some(&current[..end])
    })
}

/// Builds the forest one line at a time.
#[derive(Default)]
struct Builder {
    /// The structures still open, outermost first. Its length is the level of the next child.
    open: Vec<Structure>,
    records: Vec<Structure>,
    /// Whether the previous line could be continued by a `CONT`.
    continuable: bool,
}

impl Builder {
    fn push(&mut self, line: &Line<'_>, number: usize) -> Result<(), DocumentError> {
        let fail = |kind| Err(DocumentError { kind, line: number });

        if line.tag == "CONC" {
            return fail(DocumentErrorKind::Concatenation);
        }
        if line.tag == "CONT" {
            return self.continue_text(line, number);
        }

        if line.level > self.open.len() {
            return fail(DocumentErrorKind::LevelJump);
        }
        if line.level > 0 && line.xref.is_some() {
            return fail(DocumentErrorKind::XrefOnSubstructure);
        }

        // Close everything at this level and deeper; the new line is their sibling or an uncle.
        while self.open.len() > line.level {
            self.close();
        }

        let payload = match &line.value {
            None => Payload::None,
            Some(Value::Void) => Payload::Void,
            Some(Value::Pointer(target)) => Payload::Pointer((*target).to_owned()),
            Some(Value::Text(text)) => Payload::Text((*text).to_owned()),
        };
        self.continuable = !matches!(payload, Payload::Pointer(_) | Payload::Void);
        self.open.push(Structure {
            tag: line.tag.to_owned(),
            xref: line.xref.map(str::to_owned),
            payload,
            children: Vec::new(),
            line: number,
        });
        Ok(())
    }

    /// A `CONT` line: one more line of the value of the structure just opened.
    fn continue_text(&mut self, line: &Line<'_>, number: usize) -> Result<(), DocumentError> {
        let misplaced = DocumentError {
            kind: DocumentErrorKind::MisplacedContinuation,
            line: number,
        };
        let depth = self.open.len();
        let Some(target) = self.open.last_mut() else {
            return Err(misplaced);
        };
        // A continuation sits one level below the line it continues, and before any substructure.
        if !self.continuable
            || line.level != depth
            || line.xref.is_some()
            || !target.children.is_empty()
        {
            return Err(misplaced);
        }
        let more = match &line.value {
            None => "",
            Some(Value::Text(text)) => text,
            Some(Value::Pointer(_) | Value::Void) => return Err(misplaced),
        };
        match &mut target.payload {
            Payload::Text(text) => {
                text.push('\n');
                text.push_str(more);
            }
            payload @ Payload::None => {
                // An empty first line: "1 NOTE" followed by "2 CONT something".
                *payload = Payload::Text(format!("\n{more}"));
            }
            Payload::Pointer(_) | Payload::Void => return Err(misplaced),
        }
        Ok(())
    }

    fn close(&mut self) {
        if let Some(finished) = self.open.pop() {
            match self.open.last_mut() {
                Some(parent) => parent.children.push(finished),
                None => self.records.push(finished),
            }
        }
        self.continuable = false;
    }

    fn finish(mut self) -> Vec<Structure> {
        while !self.open.is_empty() {
            self.close();
        }
        self.records
    }
}

fn write_structure(out: &mut String, structure: &Structure, level: usize) {
    use core::fmt::Write as _;

    let _ = write!(out, "{level}");
    if let Some(xref) = &structure.xref {
        let _ = write!(out, " @{xref}@");
    }
    let _ = write!(out, " {}", structure.tag);

    match &structure.payload {
        Payload::None => out.push('\n'),
        Payload::Void => out.push_str(" @VOID@\n"),
        Payload::Pointer(target) => {
            let _ = writeln!(out, " @{target}@");
        }
        Payload::Text(text) => {
            let mut pieces = text.split('\n');
            let first = pieces.next().unwrap_or_default();
            write_line_string(out, first);
            for piece in pieces {
                let _ = write!(out, "{} CONT", level + 1);
                write_line_string(out, piece);
            }
        }
    }

    for child in &structure.children {
        write_structure(out, child, level + 1);
    }
}

/// Writes ` value\n`, or just `\n` for an empty value — "tag, space, nothing" is not a line.
fn write_line_string(out: &mut String, text: &str) {
    if !text.is_empty() {
        out.push(' ');
        if text.starts_with('@') {
            out.push('@');
        }
        out.push_str(text);
    }
    out.push('\n');
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{Document, DocumentErrorKind, Payload, Structure};
    use crate::line::LineError;

    const MINIMAL: &str = "0 HEAD\n1 GEDC\n2 VERS 7.0\n0 TRLR\n";

    fn parse(text: &str) -> Document {
        Document::parse(text).unwrap_or_else(|error| panic!("{error}"))
    }

    fn error(text: &str) -> DocumentErrorKind {
        Document::parse(text).unwrap_err().kind
    }

    #[test]
    fn the_smallest_valid_document() {
        let document = parse(MINIMAL);
        assert_eq!(document.records.len(), 1);
        let head = &document.records[0];
        assert_eq!(head.tag, "HEAD");
        assert_eq!(
            head.child("GEDC").unwrap().child("VERS").unwrap().payload,
            Payload::Text("7.0".to_owned())
        );
    }

    #[test]
    fn continuations_join_with_line_breaks_and_keep_every_space() {
        // The specification's own example, four lines with the third one blank.
        let text = "0 HEAD\n0 @N1@ SNOTE This is a note field that\n1 CONT   spans four lines.\n1 CONT\n1 CONT (the third line was blank)\n0 TRLR\n";
        let note = &parse(text).records[1];
        assert_eq!(
            note.payload.text(),
            Some("This is a note field that\n  spans four lines.\n\n(the third line was blank)")
        );
        assert!(
            note.children.is_empty(),
            "a continuation is not a substructure"
        );
    }

    #[test]
    fn every_terminator_the_specification_allows() {
        for terminator in ["\n", "\r\n", "\r"] {
            let text = MINIMAL.replace('\n', terminator);
            assert_eq!(parse(&text), parse(MINIMAL), "{terminator:?}");
        }
        assert_eq!(
            parse(&format!("\u{FEFF}{MINIMAL}")),
            parse(MINIMAL),
            "a BOM is not content"
        );
    }

    #[test]
    fn records_are_nested_by_level_and_pointers_are_resolved() {
        let text = "0 HEAD\n0 @I1@ INDI\n1 NAME Иван /Смирнов/\n2 GIVN Иван\n1 FAMS @F1@\n0 @F1@ FAM\n1 HUSB @I1@\n1 CHIL @VOID@\n0 TRLR\n";
        let document = parse(text);
        let person = &document.records[1];
        assert_eq!(person.xref.as_deref(), Some("I1"));
        assert_eq!(
            person
                .child("NAME")
                .unwrap()
                .child("GIVN")
                .unwrap()
                .payload
                .text(),
            Some("Иван")
        );
        assert_eq!(person.child("FAMS").unwrap().payload.pointer(), Some("F1"));
        assert_eq!(
            document.records[2].child("CHIL").unwrap().payload,
            Payload::Void
        );
        assert_eq!(person.line, 2);
    }

    #[test]
    fn the_container_rules() {
        assert_eq!(error("0 TRLR\n"), DocumentErrorKind::NoHeader);
        assert_eq!(error("0 HEAD\n"), DocumentErrorKind::NoTrailer);
        assert_eq!(
            error("0 HEAD\n0 TRLR\n0 @I1@ INDI\n"),
            DocumentErrorKind::NoTrailer
        );
        assert_eq!(
            error("0 HEAD\n2 GEDC\n0 TRLR\n"),
            DocumentErrorKind::LevelJump
        );
        assert_eq!(
            error("0 HEAD\n1 NOTE a\n1 CONC b\n0 TRLR\n"),
            DocumentErrorKind::Concatenation
        );
        assert_eq!(
            error("0 HEAD\n1 @X1@ NOTE a\n0 TRLR\n"),
            DocumentErrorKind::XrefOnSubstructure
        );
        assert_eq!(
            error("0 HEAD\n0 @I1@ INDI\n0 @I1@ INDI\n0 TRLR\n"),
            DocumentErrorKind::DuplicateXref("I1".to_owned())
        );
        assert_eq!(
            error("0 HEAD\n0 @F1@ FAM\n1 HUSB @I9@\n0 TRLR\n"),
            DocumentErrorKind::DanglingPointer("I9".to_owned())
        );
        assert_eq!(
            error("0 HEAD\n1 NOTE a\n2 SOUR b\n2 CONT c\n0 TRLR\n"),
            DocumentErrorKind::MisplacedContinuation,
            "a continuation comes before any substructure"
        );
        assert_eq!(
            error("0 HEAD\n1 NOTE a\u{7}\n0 TRLR\n"),
            DocumentErrorKind::BannedCharacter('\u{7}')
        );
        assert_eq!(
            error("0 HEAD\n  1 GEDC\n0 TRLR\n"),
            DocumentErrorKind::Line(LineError::LeadingWhitespace)
        );
    }

    #[test]
    fn a_blank_line_inside_a_document_is_not_forgiven() {
        // Only the empty piece after the final terminator is not a line; 7.0 has no blank lines.
        assert_eq!(
            error("0 HEAD\n\n0 TRLR\n"),
            DocumentErrorKind::Line(LineError::Empty)
        );
    }

    #[test]
    fn an_error_names_its_line() {
        let failure = Document::parse("0 HEAD\n1 GEDC\n1 bad\n0 TRLR\n").unwrap_err();
        assert_eq!(failure.line, 3);
        assert!(failure.to_string().starts_with("line 3: "));
    }

    #[test]
    fn writing_splits_values_and_doubles_only_a_leading_at() {
        let document = Document {
            records: vec![
                Structure::new("HEAD"),
                Structure::new("SNOTE")
                    .with_text("me@example.com is my email\n@me and @I are handles"),
            ],
        };
        let written = document.write();
        assert!(written.starts_with('\u{FEFF}'));
        assert!(
            written
                .contains("0 SNOTE me@example.com is my email\n1 CONT @@me and @I are handles\n")
        );
        assert!(written.ends_with("0 TRLR\n"));
    }

    #[test]
    fn a_document_survives_being_written_and_read() {
        let text = "0 HEAD\n1 GEDC\n2 VERS 7.0\n0 @I1@ INDI\n1 NAME Иван Петрович /Смирнов/\n1 NOTE  leading space, trailing space \n2 CONT\n2 CONT @@handle\n1 FAMC @VOID@\n0 @S1@ SOUR\n1 TITL Метрическая книга\n0 TRLR\n";
        let once = parse(text);
        let twice = parse(&once.write());
        // Line numbers are where a structure was read from, and are allowed to differ.
        assert_eq!(strip_lines(once), strip_lines(twice));
    }

    fn strip_lines(mut document: Document) -> Document {
        fn strip(structure: &mut Structure) {
            structure.line = 0;
            structure.children.iter_mut().for_each(strip);
        }
        document.records.iter_mut().for_each(strip);
        document
    }
}
