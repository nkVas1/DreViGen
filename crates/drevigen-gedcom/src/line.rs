//! One line of a GEDCOM 7 data stream.
//!
//! ```abnf
//! Line    = Level D [Xref D] Tag [D LineVal] EOL
//! Level   = "0" / nonzero *DIGIT
//! Xref    = atsign 1*tagchar atsign         ; but not "@VOID@"
//! Tag     = stdTag / extTag
//! LineVal = pointer / lineStr
//! lineStr = (nonAt / atsign atsign) *nonEOL ; leading @ doubled
//! ```
//!
//! From `gedcom-1-hierarchical-container-format.md` at `FamilySearch/GEDCOM@512e38d`, read rather
//! than recalled. The rules that differ from what older files do, and which this reader enforces:
//!
//! - **Exactly one space** between components. A second space after the tag belongs to the value.
//! - **No whitespace before the level.** 5.5.1 allowed it; 7.0 removed the permission.
//! - **Only a leading `@` is doubled.** 5.5.1 required every `@` in a value doubled, which few
//!   programs did; 7.0 doubles only the first character, so `me@example.com` is written as is.
//! - **A value beginning with a single `@` must be a pointer.** Anything else there was a 5.5.1
//!   escape such as `@#DJULIAN@`, now prohibited.
//! - **"Tag, space, nothing" is not a line.** An empty value is written by omitting the space.

use core::fmt;

/// What a line's value is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'a> {
    /// A pointer to the structure with this cross-reference identifier, without its `@`s.
    Pointer(&'a str),
    /// `@VOID@`: a pointer to nothing, used where a pointer is required and there is no target.
    Void,
    /// A line string, with a leading `@@` already read back as `@`.
    Text(&'a str),
}

/// One parsed line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line<'a> {
    /// The level: 0 for a record, one more than its superstructure otherwise.
    pub level: usize,
    /// The cross-reference identifier, without its `@`s.
    pub xref: Option<&'a str>,
    /// The tag.
    pub tag: &'a str,
    /// The value, if the line has one.
    pub value: Option<Value<'a>>,
}

/// Why a line is not a GEDCOM 7 line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineError {
    /// The line is empty. 7.0 has no blank lines.
    Empty,
    /// Whitespace before the level. Allowed by 5.5.1, removed in 7.0.
    LeadingWhitespace,
    /// The level is not `0` or a number without leading zeros.
    Level,
    /// A component is missing or separated by something other than one space.
    Delimiter,
    /// The cross-reference identifier is malformed, or is `@VOID@`, which is reserved.
    Xref,
    /// The tag is not an upper-case standard tag or an `_` extension tag.
    Tag,
    /// A space after the tag with nothing after it.
    TrailingSpace,
    /// A value beginning with a single `@` that is not a pointer — a 5.5.1 escape.
    StrayAt,
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "the line is empty",
            Self::LeadingWhitespace => "the line starts with whitespace",
            Self::Level => "the level is not a number without leading zeros",
            Self::Delimiter => "components must be separated by exactly one space",
            Self::Xref => "the cross-reference identifier is malformed or reserved",
            Self::Tag => "the tag is not a standard or extension tag",
            Self::TrailingSpace => "a space after the tag with no value after it",
            Self::StrayAt => "a value starting with one @ must be a pointer",
        })
    }
}

impl core::error::Error for LineError {}

impl<'a> Line<'a> {
    /// Reads one line, without its terminator.
    ///
    /// # Errors
    ///
    /// A [`LineError`] naming the first rule the line breaks.
    pub fn parse(text: &'a str) -> Result<Self, LineError> {
        if text.is_empty() {
            return Err(LineError::Empty);
        }
        if text.starts_with([' ', '\t']) {
            return Err(LineError::LeadingWhitespace);
        }

        let (level_text, rest) = text.split_once(' ').ok_or(LineError::Delimiter)?;
        let level = parse_level(level_text)?;

        let (xref, rest) = if rest.starts_with('@') {
            let (candidate, rest) = rest.split_once(' ').ok_or(LineError::Delimiter)?;
            (Some(parse_xref(candidate)?), rest)
        } else {
            (None, rest)
        };

        let (tag, value) = match rest.split_once(' ') {
            Some((tag, value)) => (tag, Some(value)),
            None => (rest, None),
        };
        if !is_tag(tag) {
            return Err(if tag.is_empty() {
                LineError::Delimiter
            } else {
                LineError::Tag
            });
        }

        let value = match value {
            None => None,
            Some("") => return Err(LineError::TrailingSpace),
            Some(value) => Some(parse_value(value)?),
        };

        Ok(Self {
            level,
            xref,
            tag,
            value,
        })
    }
}

fn parse_level(text: &str) -> Result<usize, LineError> {
    let digits = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    let canonical = text == "0" || !text.starts_with('0');
    if !(digits && canonical) {
        return Err(LineError::Level);
    }
    text.parse().map_err(|_| LineError::Level)
}

fn parse_xref(text: &str) -> Result<&str, LineError> {
    let inner = text
        .strip_prefix('@')
        .and_then(|rest| rest.strip_suffix('@'))
        .ok_or(LineError::Xref)?;
    if inner.is_empty() || inner == "VOID" || !inner.bytes().all(is_tagchar) {
        return Err(LineError::Xref);
    }
    Ok(inner)
}

fn parse_value(text: &str) -> Result<Value<'_>, LineError> {
    if text.starts_with("@@") {
        // An escaped leading @: the string really begins with one.
        return Ok(Value::Text(&text[1..]));
    }
    if text.starts_with('@') {
        if text == "@VOID@" {
            return Ok(Value::Void);
        }
        return parse_xref(text)
            .map(Value::Pointer)
            .map_err(|_| LineError::StrayAt);
    }
    Ok(Value::Text(text))
}

/// `stdTag = ucletter *tagchar`, `extTag = underscore 1*tagchar`.
pub(crate) fn is_tag(text: &str) -> bool {
    let bytes = text.as_bytes();
    match bytes.first() {
        Some(b'_') => bytes.len() > 1 && bytes[1..].iter().copied().all(is_tagchar),
        Some(first) if first.is_ascii_uppercase() => bytes[1..].iter().copied().all(is_tagchar),
        _ => false,
    }
}

/// `tagchar = ucletter / DIGIT / underscore`.
const fn is_tagchar(byte: u8) -> bool {
    byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'
}

/// Whether a character is banned anywhere in a GEDCOM 7 stream: C0 controls other than tab and
/// the line terminators, DEL, the C1 controls, and the two noncharacters `U+FFFE` and `U+FFFF`.
/// Surrogates cannot occur in a Rust string.
#[must_use]
pub const fn is_banned(character: char) -> bool {
    matches!(character,
        '\u{00}'..='\u{08}' | '\u{0B}'..='\u{0C}' | '\u{0E}'..='\u{1F}'
        | '\u{7F}'..='\u{9F}'
        | '\u{FFFE}' | '\u{FFFF}')
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{Line, LineError, Value, is_banned};

    #[test]
    fn the_specifications_example_lines() {
        let record = Line::parse("0 @I1234@ INDI").unwrap();
        assert_eq!(record.level, 0);
        assert_eq!(record.xref, Some("I1234"));
        assert_eq!(record.tag, "INDI");
        assert_eq!(record.value, None);

        let child = Line::parse("1 CHIL @I1234@").unwrap();
        assert_eq!(child.value, Some(Value::Pointer("I1234")));

        let note = Line::parse("1 NOTE This is a note field that").unwrap();
        assert_eq!(note.value, Some(Value::Text("This is a note field that")));
    }

    #[test]
    fn a_second_space_after_the_tag_belongs_to_the_value() {
        // "If the tag is followed by 2 spaces, the first space is a delimiter and the second
        // space is part of the line value."
        let line = Line::parse("2 CONT   spans four lines.").unwrap();
        assert_eq!(line.value, Some(Value::Text("  spans four lines.")));
        let line = Line::parse("1 NOTE  ").unwrap();
        assert_eq!(line.value, Some(Value::Text(" ")));
    }

    #[test]
    fn only_a_leading_at_is_doubled() {
        assert_eq!(
            Line::parse("2 CONT @@me and @I are my social media handles")
                .unwrap()
                .value,
            Some(Value::Text("@me and @I are my social media handles"))
        );
        assert_eq!(
            Line::parse("1 NOTE me@example.com is my email")
                .unwrap()
                .value,
            Some(Value::Text("me@example.com is my email"))
        );
    }

    #[test]
    fn a_pointer_to_nothing_is_its_own_value() {
        assert_eq!(
            Line::parse("1 FAMC @VOID@").unwrap().value,
            Some(Value::Void)
        );
        assert_eq!(
            Line::parse("0 @VOID@ INDI"),
            Err(LineError::Xref),
            "reserved"
        );
    }

    #[test]
    fn the_rules_that_older_files_break() {
        assert_eq!(Line::parse(""), Err(LineError::Empty));
        assert_eq!(Line::parse("  1 NAME"), Err(LineError::LeadingWhitespace));
        assert_eq!(Line::parse("01 NAME"), Err(LineError::Level));
        assert_eq!(Line::parse("1  NAME"), Err(LineError::Delimiter));
        assert_eq!(
            Line::parse("1 name"),
            Err(LineError::Tag),
            "tags are upper case"
        );
        assert_eq!(Line::parse("1 NAME "), Err(LineError::TrailingSpace));
        assert_eq!(
            Line::parse("2 DATE @#DJULIAN@ 25 OCT 1917"),
            Err(LineError::StrayAt),
            "a 5.5.1 calendar escape"
        );
        assert_eq!(Line::parse("0 @I 1@ INDI"), Err(LineError::Xref));
    }

    #[test]
    fn extension_tags_are_tags() {
        let line = Line::parse("1 _MILT Served in the Imperial Army").unwrap();
        assert_eq!(line.tag, "_MILT");
        assert_eq!(Line::parse("1 _"), Err(LineError::Tag));
    }

    #[test]
    fn cyrillic_values_are_text_like_any_other() {
        let line = Line::parse("1 NAME Иван Петрович /Смирнов/").unwrap();
        assert_eq!(line.value, Some(Value::Text("Иван Петрович /Смирнов/")));
    }

    #[test]
    fn the_banned_characters() {
        assert!(is_banned('\u{0}'));
        assert!(is_banned('\u{7F}'));
        assert!(is_banned('\u{85}'), "C1 controls");
        assert!(is_banned('\u{FFFF}'));
        assert!(!is_banned('\t'));
        assert!(!is_banned('Ж'));
    }
}
