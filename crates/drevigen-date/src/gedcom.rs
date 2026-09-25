//! The GEDCOM 7 date grammar, and a lenient reader for everything older.
//!
//! Two readers, on purpose. [`DateValue::parse_gedcom7`] is the grammar exactly as the
//! specification writes it — case-sensitive keywords, single spaces, no free text in the payload
//! — and it reports precisely where and why a payload fails, because a validator that says
//! "invalid date" helps nobody fix a file. [`RecordedDate::parse_lenient`] is for everything real
//! files contain: GEDCOM 5.5.1 escapes, lower case, spelled-out months, dual years, sentences.
//! It never fails, it never discards text, and it lists every liberty it took so that an import
//! can show them to the person whose data it is.
//!
//! The rules here were checked against the specification source, not recalled:
//! `FamilySearch/GEDCOM` at revision `512e38d`, files `gedcom-2-data-types.md` (§ Date) and
//! `gedcom-6-appendix-calendars.md`. Three of them contradict what memory suggested, and each is
//! marked where it is implemented:
//!
//! - **`ADS` is plain Adar in a common year, and `ADR` exists only in leap years.** 5.5.1 used
//!   `ADR` for plain Adar, so a legacy file writes the tag 7.0 forbids — and the specification
//!   recommends exactly the repair [`Repair::AdarInCommonYear`] makes.
//! - **`BCE` is permitted in the Gregorian and Julian calendars and in no other.**
//! - **7.0 removed the dual-year slash.** `1648/49` meant three different things in historical
//!   documents — a new-year convention, a Julian/Gregorian pair, an approximate year — and the
//!   notation could not say which. A 7.0 file stores the interpreted date and keeps the words in
//!   `PHRASE`; the lenient reader does the same, and says which reading it chose.
//!
//! # `AFT` and `BEF` changed meaning
//!
//! Under 5.5.1, `AFT 1850` meant "the event happened after 1850", which suggests 1 January 1851
//! or later. Under 7.0 it means "no earlier than 1850". The two readings differ by the stated year
//! itself, and the 7.0 reading always contains the 5.5.1 one. So a legacy date read under 7.0
//! rules is at worst slightly wider than intended and never wrong, and this reader applies 7.0
//! rules to both without recording a repair.

use core::fmt;

use crate::value::{Approximation, DateValue, RecordedDate};
use crate::{Calendar, CalendarDate, DateError};

/// Month tags of the Gregorian and Julian calendars, in order.
const GREGORIAN_MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// Month tags of the French Republican calendar. `COMP` is the complementary days.
const FRENCH_MONTHS: [&str; 13] = [
    "VEND", "BRUM", "FRIM", "NIVO", "PLUV", "VENT", "GERM", "FLOR", "PRAI", "MESS", "THER", "FRUC",
    "COMP",
];

/// Month tags of the Hebrew calendar, in the order the specification lists them.
///
/// Thirteen tags for a year of twelve or thirteen months. `ADR` is Adar I and exists only in a
/// leap year; `ADS` is plain Adar in a common year and Adar II in a leap year. So `ADS` is present
/// every year, and in a common year every tag after it sits one month earlier than its position
/// in this list.
const HEBREW_MONTHS: [&str; 13] = [
    "TSH", "CSH", "KSL", "TVT", "SHV", "ADR", "ADS", "NSN", "IYR", "SVN", "TMZ", "AAV", "ELL",
];

/// The words that introduce a form. No calendar, month or epoch may equal one of these, which is
/// what lets the grammar be read one word at a time.
const RESTRICTED: [&str; 9] = [
    "FROM", "TO", "BET", "AND", "BEF", "AFT", "ABT", "CAL", "EST",
];

/// Why a payload is not a GEDCOM 7 date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Two spaces together, or a space at either end. GEDCOM 7 separates every component with
    /// exactly one.
    ExtraSpace,
    /// The payload stopped where something else was required.
    UnexpectedEnd {
        /// What was required.
        expected: &'static str,
    },
    /// A word appeared where it does not fit.
    Unexpected {
        /// What was found.
        found: String,
        /// What would have fitted.
        expected: &'static str,
    },
    /// An extension calendar, month or epoch — a tag beginning with `_`. The grammar allows
    /// them; this implementation cannot know what they mean.
    Extension(String),
    /// A month tag that does not belong to the calendar in force.
    UnknownMonth {
        /// The tag as written.
        found: String,
        /// The calendar it was read in.
        calendar: Calendar,
    },
    /// A month that exists in the calendar but not in that year: Adar I in a common year.
    MonthNotInYear {
        /// The tag as written.
        month: String,
        /// The year it was written against.
        year: i32,
    },
    /// `BCE` on a calendar that has no epoch marker.
    EpochNotPermitted {
        /// The calendar in force.
        calendar: Calendar,
    },
    /// A year of zero. Every calendar here either counts from 1 or has no year 0 between 1 BCE
    /// and 1 CE.
    YearZero,
    /// The words are well formed but name a day that does not exist.
    InvalidDate(DateError),
}

/// A payload that could not be read, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// What is wrong.
    pub kind: ParseErrorKind,
    /// The byte offset into the payload of the word at fault, or its length when the payload
    /// ended early — enough for an editor to underline the problem.
    pub at: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ParseErrorKind::ExtraSpace => {
                write!(
                    f,
                    "extra space at {}: separate words with one space",
                    self.at
                )
            }
            ParseErrorKind::UnexpectedEnd { expected } => {
                write!(f, "the date ends where {expected} was expected")
            }
            ParseErrorKind::Unexpected { found, expected } => {
                write!(
                    f,
                    "found {found:?} at {} where {expected} was expected",
                    self.at
                )
            }
            ParseErrorKind::Extension(tag) => {
                write!(f, "{tag} is an extension this application cannot interpret")
            }
            ParseErrorKind::UnknownMonth { found, calendar } => {
                write!(
                    f,
                    "{found:?} is not a month of the {} calendar",
                    calendar.tag()
                )
            }
            ParseErrorKind::MonthNotInYear { month, year } => {
                write!(
                    f,
                    "{month} does not exist in year {year}, which has no Adar I"
                )
            }
            ParseErrorKind::EpochNotPermitted { calendar } => {
                write!(f, "the {} calendar has no BCE", calendar.tag())
            }
            ParseErrorKind::YearZero => f.write_str("there is no year 0"),
            ParseErrorKind::InvalidDate(error) => write!(f, "{error}"),
        }
    }
}

impl core::error::Error for ParseError {}

impl DateValue {
    /// Reads a GEDCOM 7 `DateValue` payload.
    ///
    /// Returns `Ok(None)` for the empty payload, which the specification allows when a `PHRASE`
    /// or `TIME` substructure carries what the date could not.
    ///
    /// ```
    /// use drevigen_date::{Calendar, DateValue};
    ///
    /// let value = DateValue::parse_gedcom7("JULIAN 25 OCT 1917").unwrap().unwrap();
    /// assert_eq!(value.calendar(), Some(Calendar::Julian));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`ParseError`] carrying what is wrong and the byte offset of the word at fault.
    pub fn parse_gedcom7(payload: &str) -> Result<Option<Self>, ParseError> {
        let mut parser = Parser::new(payload)?;
        let value = parser.value()?;
        parser.finish()?;
        Ok(value)
    }

    /// Writes this value as a GEDCOM 7 payload.
    ///
    /// Calendar names follow the specification's recommendation: omitted when every date is
    /// Gregorian, written on every date when any one is not. A rule that applies each calendar
    /// only to the date after it is correct and easy to misread, and a reader who sees
    /// `FROM 1670 TO JULIAN 1800` without knowing it may assume both ends are Julian.
    #[must_use]
    pub fn to_gedcom7(&self) -> String {
        let name_calendars = self
            .dates()
            .iter()
            .any(|date| date.calendar() != Calendar::Gregorian);
        let write = |date: &CalendarDate| write_date(date, name_calendars);

        match self {
            Self::Exact(date) => write(date),
            Self::Approximate { date, kind } => format!("{} {}", kind.tag(), write(date)),
            Self::Between { earliest, latest } => {
                format!("BET {} AND {}", write(earliest), write(latest))
            }
            Self::Before(date) => format!("BEF {}", write(date)),
            Self::After(date) => format!("AFT {}", write(date)),
            Self::Period { from, to } => match (from, to) {
                (Some(start), Some(end)) => format!("FROM {} TO {}", write(start), write(end)),
                (Some(start), None) => format!("FROM {}", write(start)),
                (None, Some(end)) => format!("TO {}", write(end)),
                (None, None) => String::new(),
            },
        }
    }
}

impl CalendarDate {
    /// Reads a GEDCOM 7 `DateExact` payload: day, month and year, Gregorian, nothing else.
    ///
    /// The form used for timestamps — when a record was changed, when a file was written — where
    /// a partial date would be a defect rather than a fact about the past.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseError`] if any of the three parts is missing, if a calendar is named, or
    /// if the day does not exist.
    pub fn parse_gedcom7_exact(payload: &str) -> Result<Self, ParseError> {
        let mut parser = Parser::new(payload)?;
        let day = parser.next("a day")?;
        if !is_integer(day.text) {
            return Err(unexpected(day, "a day"));
        }
        let month = parser.next("a month")?;
        if !is_month_like(month.text) {
            return Err(unexpected(month, "a month"));
        }
        let year = parser.next("a year")?;
        parser.finish()?;
        build(Calendar::Gregorian, Some(day), Some(month), year, None)
    }
}

impl RecordedDate {
    /// Reads a GEDCOM 7 date: the `DATE` payload and, if present, its `PHRASE`.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseError`] if the payload is not a `DateValue`. The phrase is free text and
    /// cannot be wrong.
    pub fn parse_gedcom7(payload: &str, phrase: Option<&str>) -> Result<Self, ParseError> {
        Ok(Self {
            value: DateValue::parse_gedcom7(payload)?,
            phrase: phrase.map(str::to_owned),
        })
    }

    /// Writes this date as GEDCOM 7: the `DATE` payload, and the `PHRASE` to put beneath it.
    ///
    /// The payload is empty when there is no value, which is how 7.0 records a date that exists
    /// only as words.
    #[must_use]
    pub fn to_gedcom7(&self) -> (String, Option<String>) {
        (
            self.value
                .as_ref()
                .map(DateValue::to_gedcom7)
                .unwrap_or_default(),
            self.phrase.clone(),
        )
    }

    /// Reads whatever a file actually contains, and says what it did to read it.
    ///
    /// Never fails and never drops text. What cannot be read as a date is kept as a phrase, with
    /// the reason recorded, so the person importing can see it and decide. See [`Repair`].
    ///
    /// ```
    /// use drevigen_date::{Calendar, RecordedDate};
    ///
    /// let read = RecordedDate::parse_lenient("@#DJULIAN@ 23 Feb 1747/48");
    /// let value = read.date.value.unwrap();
    /// assert_eq!(value.calendar(), Some(Calendar::Julian));
    /// // The words are kept beside the value, and every liberty taken is listed.
    /// assert_eq!(read.date.phrase.as_deref(), Some("@#DJULIAN@ 23 Feb 1747/48"));
    /// assert!(!read.repairs.is_empty());
    /// ```
    #[must_use]
    pub fn parse_lenient(text: &str) -> Lenient {
        lenient(text)
    }
}

// ── the strict grammar ──────────────────────────────────────────────────────────────────────

/// One word of a payload and where it starts.
#[derive(Debug, Clone, Copy)]
struct Token<'a> {
    text: &'a str,
    at: usize,
}

/// A recursive-descent reader over the words of one payload.
struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    position: usize,
    length: usize,
}

impl<'a> Parser<'a> {
    /// Splits a payload into words, refusing anything but single spaces between them.
    fn new(payload: &'a str) -> Result<Self, ParseError> {
        let mut tokens = Vec::new();
        if !payload.is_empty() {
            let mut at = 0;
            for text in payload.split(' ') {
                if text.is_empty() {
                    return Err(ParseError {
                        kind: ParseErrorKind::ExtraSpace,
                        at,
                    });
                }
                tokens.push(Token { text, at });
                at += text.len() + 1;
            }
        }
        Ok(Self {
            tokens,
            position: 0,
            length: payload.len(),
        })
    }

    fn peek(&self) -> Option<Token<'a>> {
        self.tokens.get(self.position).copied()
    }

    fn next(&mut self, expected: &'static str) -> Result<Token<'a>, ParseError> {
        let token = self.peek().ok_or(ParseError {
            kind: ParseErrorKind::UnexpectedEnd { expected },
            at: self.length,
        })?;
        self.position += 1;
        Ok(token)
    }

    /// Consumes the word if it is the given keyword.
    fn eat(&mut self, word: &str) -> bool {
        if self.peek().is_some_and(|token| token.text == word) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, word: &'static str) -> Result<(), ParseError> {
        let token = self.next(word)?;
        if token.text == word {
            Ok(())
        } else {
            Err(unexpected(token, word))
        }
    }

    fn finish(&self) -> Result<(), ParseError> {
        match self.peek() {
            Some(token) => Err(unexpected(token, "the end of the date")),
            None => Ok(()),
        }
    }

    /// `DateValue = [ date / DatePeriod / dateRange / dateApprox ]`
    fn value(&mut self) -> Result<Option<DateValue>, ParseError> {
        let Some(first) = self.peek() else {
            return Ok(None);
        };

        let value = match first.text {
            "FROM" => {
                self.position += 1;
                let from = self.date()?;
                let to = if self.eat("TO") {
                    Some(self.date()?)
                } else {
                    None
                };
                DateValue::Period {
                    from: Some(from),
                    to,
                }
            }
            "TO" => {
                self.position += 1;
                DateValue::Period {
                    from: None,
                    to: Some(self.date()?),
                }
            }
            "BET" => {
                self.position += 1;
                let earliest = self.date()?;
                self.expect("AND")?;
                let latest = self.date()?;
                DateValue::Between { earliest, latest }
            }
            "BEF" => {
                self.position += 1;
                DateValue::Before(self.date()?)
            }
            "AFT" => {
                self.position += 1;
                DateValue::After(self.date()?)
            }
            "ABT" | "CAL" | "EST" => {
                self.position += 1;
                let kind = match first.text {
                    "ABT" => Approximation::About,
                    "CAL" => Approximation::Calculated,
                    _ => Approximation::Estimated,
                };
                DateValue::Approximate {
                    date: self.date()?,
                    kind,
                }
            }
            _ => DateValue::Exact(self.date()?),
        };
        Ok(Some(value))
    }

    /// `date = [calendar D] [[day D] month D] year [D epoch]`
    fn date(&mut self) -> Result<CalendarDate, ParseError> {
        let first = self.next("a date")?;

        let (calendar, first) = if let Some(calendar) = calendar_word(first.text) {
            (calendar, self.next("a day, month or year")?)
        } else if first.text.starts_with('_') {
            // In this position an extension tag is a calendar or a month, and either way it
            // cannot be interpreted.
            return Err(extension(first));
        } else {
            (Calendar::Gregorian, first)
        };

        let (day, month, year) = if is_integer(first.text) {
            match self.peek() {
                Some(next) if is_month_like(next.text) => {
                    self.position += 1;
                    (Some(first), Some(next), self.next("a year")?)
                }
                _ => (None, None, first),
            }
        } else if is_month_like(first.text) {
            (None, Some(first), self.next("a year")?)
        } else {
            return Err(unexpected(first, "a day, month or year"));
        };

        let epoch = match self.peek() {
            Some(token) if token.text == "BCE" => {
                self.position += 1;
                Some(token)
            }
            Some(token) if token.text.starts_with('_') => return Err(extension(token)),
            _ => None,
        };

        build(calendar, day, month, year, epoch)
    }
}

/// Turns the words of one `date` into a checked [`CalendarDate`].
fn build(
    calendar: Calendar,
    day: Option<Token<'_>>,
    month: Option<Token<'_>>,
    year: Token<'_>,
    epoch: Option<Token<'_>>,
) -> Result<CalendarDate, ParseError> {
    if !is_integer(year.text) {
        return Err(unexpected(year, "a year"));
    }
    // A year too long to hold is reported as the word written. Converting it to some maximum
    // first would make the error quote a number the file never contained.
    let Ok(written) = year.text.parse::<i32>() else {
        return Err(unexpected(year, "a year within the supported range"));
    };
    if written == 0 {
        return Err(ParseError {
            kind: ParseErrorKind::YearZero,
            at: year.at,
        });
    }

    if let Some(epoch) = epoch
        && !matches!(calendar, Calendar::Gregorian | Calendar::Julian)
    {
        return Err(ParseError {
            kind: ParseErrorKind::EpochNotPermitted { calendar },
            at: epoch.at,
        });
    }
    // "Year y BCE indicates a year y years before year 1. Thus there is no year 0." In the
    // astronomical numbering this crate stores, 1 BCE is year 0.
    let astronomical = if epoch.is_some() {
        1 - written
    } else {
        written
    };

    let month_number = match month {
        Some(token) => Some(
            month_number(calendar, astronomical, token.text)
                .map_err(|kind| ParseError { kind, at: token.at })?,
        ),
        None => None,
    };

    let day_number = match day {
        Some(token) => match token.text.parse::<u8>() {
            Ok(number) => Some(number),
            // No calendar has a month of more than 36 days, so a day that does not fit a byte is
            // not a day, and reporting it as "day 255" would misquote the file.
            Err(_) => return Err(unexpected(token, "a day of the month")),
        },
        None => None,
    };

    CalendarDate::new(calendar, astronomical, month_number, day_number).map_err(|error| {
        let at = match error {
            DateError::DayOutOfRange { .. } => day.map_or(year.at, |token| token.at),
            DateError::MonthOutOfRange { .. } => month.map_or(year.at, |token| token.at),
            _ => year.at,
        };
        ParseError {
            kind: ParseErrorKind::InvalidDate(error),
            at,
        }
    })
}

/// Resolves a month tag to the month's number in the order its year runs.
fn month_number(calendar: Calendar, year: i32, tag: &str) -> Result<u8, ParseErrorKind> {
    if tag.starts_with('_') {
        return Err(ParseErrorKind::Extension(tag.to_owned()));
    }
    let unknown = || ParseErrorKind::UnknownMonth {
        found: tag.to_owned(),
        calendar,
    };

    let position = match calendar {
        Calendar::Gregorian | Calendar::Julian => GREGORIAN_MONTHS.iter().position(|m| *m == tag),
        Calendar::FrenchRepublican => FRENCH_MONTHS.iter().position(|m| *m == tag),
        Calendar::Hebrew => {
            let index = HEBREW_MONTHS
                .iter()
                .position(|m| *m == tag)
                .ok_or_else(unknown)?;
            return hebrew_month_number(year, index, tag);
        }
    };

    position
        // The tables hold at most thirteen entries.
        .map(|index| index as u8 + 1)
        .ok_or_else(unknown)
}

/// The Hebrew month number for the tag at `index` in [`HEBREW_MONTHS`], in a given year.
fn hebrew_month_number(year: i32, index: usize, tag: &str) -> Result<u8, ParseErrorKind> {
    let position = index as u8 + 1;
    if crate::hebrew::is_leap_year(year) {
        return Ok(position);
    }
    match position {
        // Adar I, in a year with no Adar I.
        6 => Err(ParseErrorKind::MonthNotInYear {
            month: tag.to_owned(),
            year,
        }),
        // ADS and everything after it: one place earlier, because Adar I is not there.
        7.. => Ok(position - 1),
        _ => Ok(position),
    }
}

/// The tag for a month number in a given calendar and year.
#[must_use]
pub fn month_tag(calendar: Calendar, year: i32, month: u8) -> Option<&'static str> {
    let index = usize::from(month.checked_sub(1)?);
    match calendar {
        Calendar::Gregorian | Calendar::Julian => GREGORIAN_MONTHS.get(index).copied(),
        Calendar::FrenchRepublican => FRENCH_MONTHS.get(index).copied(),
        Calendar::Hebrew => {
            if crate::hebrew::is_leap_year(year) || index < 5 {
                HEBREW_MONTHS.get(index).copied()
            } else {
                // A common year skips ADR, so month 6 is ADS and every later month is one tag on.
                HEBREW_MONTHS.get(index + 1).copied()
            }
        }
    }
}

/// Writes one `date` production.
fn write_date(date: &CalendarDate, name_calendar: bool) -> String {
    let mut words: Vec<String> = Vec::with_capacity(5);
    if name_calendar {
        words.push(date.calendar().tag().to_owned());
    }
    if let Some(day) = date.day() {
        words.push(day.to_string());
    }
    if let Some(month) = date.month() {
        // A CalendarDate is validated on construction, so its month always has a tag.
        words.push(
            month_tag(date.calendar(), date.year(), month)
                .unwrap_or("?")
                .to_owned(),
        );
    }
    // Only the proleptic calendars reach year 0 and below; the others refuse to construct them.
    if date.year() <= 0 {
        words.push((1 - date.year()).to_string());
        words.push("BCE".to_owned());
    } else {
        words.push(date.year().to_string());
    }
    words.join(" ")
}

fn calendar_word(text: &str) -> Option<Calendar> {
    match text {
        "GREGORIAN" => Some(Calendar::Gregorian),
        "JULIAN" => Some(Calendar::Julian),
        "HEBREW" => Some(Calendar::Hebrew),
        "FRENCH_R" => Some(Calendar::FrenchRepublican),
        _ => None,
    }
}

fn is_integer(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Whether a word has the shape of a month tag: `stdTag` or `extTag`, and none of the words the
/// grammar reserves.
fn is_month_like(text: &str) -> bool {
    let mut bytes = text.bytes();
    let shaped = match bytes.next() {
        Some(b'_') => text.len() > 1,
        Some(first) if first.is_ascii_uppercase() => true,
        _ => false,
    };
    shaped
        && text
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        && !RESTRICTED.contains(&text)
        && text != "BCE"
}

fn unexpected(token: Token<'_>, expected: &'static str) -> ParseError {
    ParseError {
        kind: ParseErrorKind::Unexpected {
            found: token.text.to_owned(),
            expected,
        },
        at: token.at,
    }
}

fn extension(token: Token<'_>) -> ParseError {
    ParseError {
        kind: ParseErrorKind::Extension(token.text.to_owned()),
        at: token.at,
    }
}

// ── the lenient reader ──────────────────────────────────────────────────────────────────────

/// What the lenient reader made of a date, and what it had to do to get there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lenient {
    /// The date as it will be stored.
    pub date: RecordedDate,
    /// Every liberty taken, in the order taken. Empty when the input was already valid GEDCOM 7.
    pub repairs: Vec<Repair>,
}

/// How much a repair changed what the file said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Only the spelling changed; the meaning is certain. Legacy escapes, lower case, a month
    /// written out in full.
    Notation,
    /// One reading was chosen where the file allowed more than one. The words are kept beside
    /// the value, so the choice can be revisited.
    Interpretation,
    /// Nothing could be read as a date. The words are kept, and nothing can be computed from
    /// them until someone reads them.
    Unreadable,
}

/// One liberty the lenient reader took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repair {
    /// Keywords or months were not upper-case, which GEDCOM 7 requires.
    Capitalised,
    /// A GEDCOM 5.5.1 calendar escape such as `@#DJULIAN@`, rewritten as the 7.0 calendar name.
    LegacyCalendar {
        /// The escape as written.
        escape: String,
        /// The calendar it names.
        calendar: Calendar,
    },
    /// A GEDCOM 5.5.1 epoch marker, `B.C.` or `BC`, rewritten as `BCE`.
    LegacyEpoch {
        /// The marker as written.
        written: String,
    },
    /// A month written out or abbreviated with a full stop, rewritten as its tag.
    MonthSpelling {
        /// The word as written.
        written: String,
        /// The tag it became.
        tag: &'static str,
    },
    /// A 5.5.1 phrase in parentheses: the whole payload was words, and is kept as a phrase.
    LegacyPhrase,
    /// A 5.5.1 interpreted date, `INT date (phrase)`: the value and the words both kept, which is
    /// what 7.0 does with a `PHRASE` beside a `DATE`.
    Interpreted,
    /// Adar written as `ADR` in a year that has no Adar I, read as `ADS`. 5.5.1 used `ADR` for
    /// plain Adar, and the 7.0 specification recommends exactly this rewrite.
    AdarInCommonYear {
        /// The Hebrew year.
        year: i32,
    },
    /// A dual year such as `1648/49`, which 7.0 no longer allows because the slash meant
    /// different things in different documents. The words are kept as a phrase.
    DualYear {
        /// The year as written.
        written: String,
        /// Which reading was chosen.
        reading: DualReading,
    },
    /// A calendar this application cannot compute with: 5.5.1's `ROMAN` and `UNKNOWN`, or an
    /// extension. The payload is kept as a phrase.
    UnsupportedCalendar {
        /// The calendar as written.
        name: String,
    },
    /// Nothing could be read as a date. The text is kept, whole, as a phrase.
    Unreadable {
        /// Why the strict grammar refused it, after every other repair had been tried.
        error: ParseError,
    },
}

/// Which of the historical meanings of a slashed year was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DualReading {
    /// The new-year convention — England counted the year from 25 March until 1752, so
    /// "30 January 1648/49" is 30 January 1649 by modern count. Chosen when a month is given,
    /// because this is what 5.5.1 defined the notation for and a dated day makes the other
    /// readings unlikely.
    NewYearStyle,
    /// Either of the two years — a year computed from an age, written "1903/4". Chosen when only
    /// a year is given, because it is the wider reading and contains the new-year one: nothing
    /// true is excluded.
    EitherYear,
}

impl Repair {
    /// How much this repair changed what the file said.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        match self {
            Self::Capitalised
            | Self::LegacyCalendar { .. }
            | Self::LegacyEpoch { .. }
            | Self::MonthSpelling { .. }
            | Self::LegacyPhrase
            | Self::Interpreted
            | Self::AdarInCommonYear { .. } => Severity::Notation,
            Self::DualYear { .. } => Severity::Interpretation,
            Self::UnsupportedCalendar { .. } | Self::Unreadable { .. } => Severity::Unreadable,
        }
    }
}

impl Lenient {
    /// The most serious repair taken, or `None` if the input was already valid.
    #[must_use]
    pub fn worst(&self) -> Option<Severity> {
        self.repairs.iter().map(Repair::severity).max()
    }
}

/// GEDCOM 5.5.1 calendar escapes and what they became. `ROMAN` and `UNKNOWN` were declared and
/// never defined, so they have nothing to become.
const LEGACY_CALENDARS: [(&str, Option<Calendar>); 6] = [
    ("@#DGREGORIAN@", Some(Calendar::Gregorian)),
    ("@#DJULIAN@", Some(Calendar::Julian)),
    ("@#DHEBREW@", Some(Calendar::Hebrew)),
    ("@#DFRENCH R@", Some(Calendar::FrenchRepublican)),
    ("@#DROMAN@", None),
    ("@#DUNKNOWN@", None),
];

/// Months written out in English, and the tags they are. `MAY` needs no entry.
const ENGLISH_MONTHS: [(&str, &str); 12] = [
    ("JANUARY", "JAN"),
    ("FEBRUARY", "FEB"),
    ("MARCH", "MAR"),
    ("APRIL", "APR"),
    ("JUNE", "JUN"),
    ("JULY", "JUL"),
    ("AUGUST", "AUG"),
    ("SEPTEMBER", "SEP"),
    ("SEPT", "SEP"),
    ("OCTOBER", "OCT"),
    ("NOVEMBER", "NOV"),
    ("DECEMBER", "DEC"),
];

fn lenient(text: &str) -> Lenient {
    let original = text.trim();
    let mut repairs = Vec::new();

    if original.is_empty() {
        return Lenient {
            date: RecordedDate::default(),
            repairs,
        };
    }

    // `(words)` — a 5.5.1 date phrase with no date in it.
    if let Some(inner) = original
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    {
        repairs.push(Repair::LegacyPhrase);
        return Lenient {
            date: RecordedDate::from_phrase(inner.trim()),
            repairs,
        };
    }

    // `INT date (words)` — both halves, which is what 7.0 means by a DATE with a PHRASE.
    let (mut payload, mut phrase) = split_interpreted(original, &mut repairs);

    // Calendar escapes first: one of them contains a space, so it has to go before splitting.
    for (escape, calendar) in LEGACY_CALENDARS {
        let Some(found) = find_ignoring_case(&payload, escape) else {
            continue;
        };
        let written = payload[found..found + escape.len()].to_owned();
        match calendar {
            Some(calendar) => {
                payload.replace_range(found..found + escape.len(), calendar.tag());
                repairs.push(Repair::LegacyCalendar {
                    escape: written,
                    calendar,
                });
            }
            None => return unsupported(original, written, repairs),
        }
    }
    if let Some(found) = find_ignoring_case(&payload, "@#D") {
        let name = payload[found..]
            .split('@')
            .nth(1)
            .map_or_else(|| payload[found..].to_owned(), |inner| format!("@{inner}@"));
        return unsupported(original, name, repairs);
    }

    let (normalised, dual) = normalise_words(&payload, &mut repairs);
    let parsed = parse_repairing_adar(normalised, &mut repairs);

    let mut value = match parsed {
        Ok(value) => value,
        Err(error) => {
            // Everything was tried. Keep the whole text, including any INT phrase, so nothing
            // the file said is lost.
            repairs.push(Repair::Unreadable { error });
            return Lenient {
                date: RecordedDate::from_phrase(original),
                repairs,
            };
        }
    };

    if let Some((written, year)) = dual {
        let reading = widen_dual_year(&mut value, year);
        repairs.push(Repair::DualYear { written, reading });
        // The slash is exactly the information a value cannot carry, so the words are kept.
        if phrase.is_none() {
            phrase = Some(original.to_owned());
        }
    }

    // Non-empty text that produced neither a value nor words — `INT (` is the case that found
    // this — must still be kept. The promise is that nothing the file said is dropped, and a
    // promise with an exception for inputs nobody expected is not one.
    if value.is_none() && phrase.is_none() {
        repairs.push(Repair::Unreadable {
            error: ParseError {
                kind: ParseErrorKind::UnexpectedEnd { expected: "a date" },
                at: original.len(),
            },
        });
        return Lenient {
            date: RecordedDate::from_phrase(original),
            repairs,
        };
    }

    Lenient {
        date: RecordedDate { value, phrase },
        repairs,
    }
}

/// Rewrites a payload word by word into GEDCOM 7 spelling: upper case, `BCE` for the legacy
/// epoch markers, tags for spelled-out months, and the later year for the first slashed year.
///
/// Returns the rewritten payload, and the slashed year if there was one, as written and as read.
fn normalise_words(payload: &str, repairs: &mut Vec<Repair>) -> (String, Option<(String, i32)>) {
    let mut words = Vec::new();
    let mut capitalised = false;
    let mut dual = None;

    for word in payload.split_whitespace() {
        let upper = word.to_uppercase();
        capitalised |= upper != word;

        if matches!(upper.as_str(), "B.C." | "BC" | "B.C" | "BC.") {
            repairs.push(Repair::LegacyEpoch {
                written: word.to_owned(),
            });
            words.push("BCE".to_owned());
            continue;
        }

        // "September", "Sept.", "Jan." — spelled out, or abbreviated with a full stop.
        let bare = upper.trim_end_matches('.');
        let spelled = ENGLISH_MONTHS
            .iter()
            .find(|(name, _)| *name == bare)
            .map(|(_, tag)| *tag)
            .or_else(|| {
                (bare != upper)
                    .then(|| GREGORIAN_MONTHS.iter().find(|month| **month == bare))
                    .flatten()
                    .copied()
            });
        if let Some(tag) = spelled {
            repairs.push(Repair::MonthSpelling {
                written: word.to_owned(),
                tag,
            });
            words.push(tag.to_owned());
            continue;
        }

        if dual.is_none()
            && let Some(year) = new_style_year(&upper)
        {
            dual = Some((word.to_owned(), year));
            words.push(year.to_string());
            continue;
        }

        words.push(upper);
    }

    if capitalised {
        repairs.insert(0, Repair::Capitalised);
    }
    (words.join(" "), dual)
}

/// Parses a normalised payload, rewriting `ADR` as `ADS` wherever the grammar refuses it for a
/// common year — the specification's own recommendation. A value holds at most two dates, so at
/// most two repairs, each at the word the grammar pointed to.
fn parse_repairing_adar(
    mut normalised: String,
    repairs: &mut Vec<Repair>,
) -> Result<Option<DateValue>, ParseError> {
    let mut parsed = DateValue::parse_gedcom7(&normalised);
    for _ in 0..2 {
        let Err(ParseError {
            kind: ParseErrorKind::MonthNotInYear { year, .. },
            at,
        }) = &parsed
        else {
            break;
        };
        let (year, at) = (*year, *at);
        normalised.replace_range(at..at + "ADR".len(), "ADS");
        repairs.push(Repair::AdarInCommonYear { year });
        parsed = DateValue::parse_gedcom7(&normalised);
    }
    parsed
}

/// Splits `INT date (phrase)` into its halves. Anything else is returned whole, with no phrase.
fn split_interpreted(text: &str, repairs: &mut Vec<Repair>) -> (String, Option<String>) {
    let Some(rest) = strip_prefix_ignoring_case(text, "INT ") else {
        return (text.to_owned(), None);
    };
    repairs.push(Repair::Interpreted);
    match rest.find('(') {
        Some(open) => {
            let date = rest[..open].trim().to_owned();
            let words = rest[open + 1..].trim_end().trim_end_matches(')').trim();
            let phrase = (!words.is_empty()).then(|| words.to_owned());
            (date, phrase)
        }
        None => (rest.trim().to_owned(), None),
    }
}

/// A date in a calendar that cannot be computed with: keep it all as words, including any
/// `INT` phrase, which is inside `original`.
fn unsupported(original: &str, name: String, mut repairs: Vec<Repair>) -> Lenient {
    repairs.push(Repair::UnsupportedCalendar { name });
    Lenient {
        date: RecordedDate::from_phrase(original),
        repairs,
    }
}

/// Reads a slashed year — `1648/49`, `1648/9`, `1648/1649`, `1699/00` — as the later of two
/// consecutive years. Anything else, including two years that are not consecutive, is not a
/// dual year and is left for the grammar to refuse.
fn new_style_year(word: &str) -> Option<i32> {
    let (first, second) = word.split_once('/')?;
    if !is_integer(first) || !is_integer(second) || second.len() > first.len() {
        return None;
    }
    let old: i32 = first.parse().ok()?;
    let written: i32 = second.parse().ok()?;
    let digits = u32::try_from(second.len()).ok()?;
    let modulus = 10_i32.checked_pow(digits)?;

    let candidate = if second.len() == first.len() {
        written
    } else {
        let base = old - old.rem_euclid(modulus) + written;
        // `1699/00` rolls into the next century.
        if base <= old { base + modulus } else { base }
    };
    (candidate == old + 1).then_some(candidate)
}

/// Applies the reading of a dual year to a value that has already been read with the later year.
///
/// With a month, the new-year reading stands. With only a year, and nothing else in the value,
/// the wider reading is taken — both years — because it contains the other and excludes nothing
/// that might be true.
fn widen_dual_year(value: &mut Option<DateValue>, later: i32) -> DualReading {
    if let Some(DateValue::Exact(date)) = value
        && date.month().is_none()
        && let Ok(earlier) = CalendarDate::new(date.calendar(), later - 1, None, None)
    {
        *value = Some(DateValue::Between {
            earliest: earlier,
            latest: *date,
        });
        return DualReading::EitherYear;
    }
    DualReading::NewYearStyle
}

fn find_ignoring_case(haystack: &str, needle: &str) -> Option<usize> {
    haystack.to_ascii_uppercase().find(needle)
}

fn strip_prefix_ignoring_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use drevigen_testkit::Rng;

    use super::{
        DualReading, ParseError, ParseErrorKind, Repair, Severity, month_tag, new_style_year,
    };
    use crate::value::{Approximation, DateValue, RecordedDate};
    use crate::{Calendar, CalendarDate, DateError};

    fn parse(payload: &str) -> DateValue {
        DateValue::parse_gedcom7(payload)
            .unwrap_or_else(|error| panic!("{payload:?}: {error}"))
            .unwrap_or_else(|| panic!("{payload:?} parsed as empty"))
    }

    fn error(payload: &str) -> ParseError {
        DateValue::parse_gedcom7(payload).expect_err(payload)
    }

    fn date(calendar: Calendar, year: i32, month: Option<u8>, day: Option<u8>) -> CalendarDate {
        CalendarDate::new(calendar, year, month, day).unwrap()
    }

    // ── strict ───────────────────────────────────────────────────────────────────────────────

    #[test]
    fn the_empty_payload_is_a_date_that_only_has_substructures() {
        assert_eq!(DateValue::parse_gedcom7(""), Ok(None));
        let spec = RecordedDate::parse_gedcom7("", Some("5 January (year unknown)")).unwrap();
        assert_eq!(spec.value, None);
        assert_eq!(spec.phrase.as_deref(), Some("5 January (year unknown)"));
    }

    #[test]
    fn every_form_in_the_grammar_reads() {
        let g = Calendar::Gregorian;
        assert_eq!(
            parse("17 APR 1871"),
            DateValue::Exact(date(g, 1871, Some(4), Some(17)))
        );
        assert_eq!(
            parse("APR 1871"),
            DateValue::Exact(date(g, 1871, Some(4), None))
        );
        assert_eq!(parse("1871"), DateValue::Exact(date(g, 1871, None, None)));
        assert_eq!(
            parse("EST 1871"),
            DateValue::Approximate {
                date: date(g, 1871, None, None),
                kind: Approximation::Estimated
            }
        );
        assert_eq!(
            parse("BET 1869 AND 1873"),
            DateValue::Between {
                earliest: date(g, 1869, None, None),
                latest: date(g, 1873, None, None)
            }
        );
        assert_eq!(
            parse("BEF 1875"),
            DateValue::Before(date(g, 1875, None, None))
        );
        assert_eq!(
            parse("AFT 1869"),
            DateValue::After(date(g, 1869, None, None))
        );
        assert_eq!(
            parse("FROM 1902 TO 1914"),
            DateValue::Period {
                from: Some(date(g, 1902, None, None)),
                to: Some(date(g, 1914, None, None))
            }
        );
        assert_eq!(
            parse("FROM 1902"),
            DateValue::Period {
                from: Some(date(g, 1902, None, None)),
                to: None
            }
        );
        // `TO x` alone is a period, not a range — "lasted for multiple days, ending on x".
        assert_eq!(
            parse("TO 1914"),
            DateValue::Period {
                from: None,
                to: Some(date(g, 1914, None, None))
            }
        );
    }

    #[test]
    fn a_calendar_applies_to_the_date_after_it_and_no_further() {
        // The specification's own three examples.
        let value = parse("FROM 1670 TO JULIAN 1800");
        let DateValue::Period {
            from: Some(from),
            to: Some(to),
        } = value
        else {
            panic!("not a period")
        };
        assert_eq!(from.calendar(), Calendar::Gregorian);
        assert_eq!(to.calendar(), Calendar::Julian);

        let DateValue::Period {
            from: Some(from),
            to: Some(to),
        } = parse("FROM JULIAN 1670 TO 1800")
        else {
            panic!("not a period")
        };
        assert_eq!(from.calendar(), Calendar::Julian);
        assert_eq!(to.calendar(), Calendar::Gregorian);
    }

    #[test]
    fn writing_names_every_calendar_once_any_is_not_gregorian() {
        // The specification's recommended forms, verbatim.
        assert_eq!(parse("FROM 1670 TO 1800").to_gedcom7(), "FROM 1670 TO 1800");
        assert_eq!(
            parse("FROM 1670 TO JULIAN 1800").to_gedcom7(),
            "FROM GREGORIAN 1670 TO JULIAN 1800"
        );
        assert_eq!(
            parse("FROM JULIAN 1670 TO 1800").to_gedcom7(),
            "FROM JULIAN 1670 TO GREGORIAN 1800"
        );
    }

    #[test]
    fn the_october_revolution_round_trips() {
        let value = parse("JULIAN 25 OCT 1917");
        assert_eq!(value.to_gedcom7(), "JULIAN 25 OCT 1917");
        assert_eq!(
            value.to_calendar(Calendar::Gregorian).to_gedcom7(),
            "7 NOV 1917"
        );
    }

    #[test]
    fn bce_is_counted_without_a_year_zero() {
        // "Year 1 BCE was followed by year 1" — so 1 BCE is astronomical 0, 5 BCE is -4.
        let DateValue::Exact(one_bce) = parse("1 BCE") else {
            panic!()
        };
        assert_eq!(one_bce.year(), 0);
        let DateValue::Exact(five_bce) = parse("JULIAN 5 BCE") else {
            panic!()
        };
        assert_eq!(five_bce.year(), -4);
        assert_eq!(parse("JULIAN 5 BCE").to_gedcom7(), "JULIAN 5 BCE");
        assert_eq!(parse("15 MAR 44 BCE").to_gedcom7(), "15 MAR 44 BCE");
    }

    #[test]
    fn bce_belongs_only_to_the_proleptic_calendars() {
        for payload in ["HEBREW 5 BCE", "FRENCH_R 2 BCE"] {
            assert!(
                matches!(
                    error(payload).kind,
                    ParseErrorKind::EpochNotPermitted { .. }
                ),
                "{payload}"
            );
        }
    }

    #[test]
    fn hebrew_months_resolve_against_their_year() {
        // 5784 was a leap year, 5785 a common one. Nisan is the eighth month in the first and
        // the seventh in the second; the tag is the same.
        let DateValue::Exact(leap) = parse("HEBREW 15 NSN 5784") else {
            panic!()
        };
        assert_eq!(leap.month(), Some(8));
        let DateValue::Exact(common) = parse("HEBREW 15 NSN 5785") else {
            panic!()
        };
        assert_eq!(common.month(), Some(7));

        // ADS is plain Adar in a common year: month 6. In a leap year it is Adar II, month 7.
        let DateValue::Exact(adar) = parse("HEBREW ADS 5785") else {
            panic!()
        };
        assert_eq!(adar.month(), Some(6));
        let DateValue::Exact(adar_ii) = parse("HEBREW ADS 5784") else {
            panic!()
        };
        assert_eq!(adar_ii.month(), Some(7));

        // And back again.
        assert_eq!(month_tag(Calendar::Hebrew, 5785, 6), Some("ADS"));
        assert_eq!(month_tag(Calendar::Hebrew, 5784, 6), Some("ADR"));
        assert_eq!(month_tag(Calendar::Hebrew, 5785, 12), Some("ELL"));
        assert_eq!(month_tag(Calendar::Hebrew, 5784, 13), Some("ELL"));
        assert_eq!(month_tag(Calendar::Hebrew, 5785, 13), None);
    }

    #[test]
    fn adar_i_in_a_common_year_is_refused_by_the_strict_reader() {
        let refused = error("HEBREW ADR 5785");
        assert_eq!(
            refused.kind,
            ParseErrorKind::MonthNotInYear {
                month: "ADR".to_owned(),
                year: 5785
            }
        );
        assert_eq!(refused.at, 7, "points at the month");
    }

    #[test]
    fn republican_dates_read() {
        let DateValue::Exact(thermidor) = parse("FRENCH_R 9 THER 2") else {
            panic!()
        };
        assert_eq!(thermidor.month(), Some(11));
        assert_eq!(
            crate::gregorian::from_day(thermidor.earliest()),
            (1794, 7, 27)
        );
        // The complementary days are the thirteenth month.
        let DateValue::Exact(complementary) = parse("FRENCH_R 6 COMP 3") else {
            panic!()
        };
        assert_eq!(complementary.month(), Some(13));
        // Year III was a sextile year and has a sixth; year IV was not.
        assert!(matches!(
            error("FRENCH_R 6 COMP 4").kind,
            ParseErrorKind::InvalidDate(DateError::DayOutOfRange { given: 6, last: 5 })
        ));
    }

    #[test]
    fn errors_say_what_is_wrong_and_where() {
        // A double space: the second space is data under GEDCOM 7's line rules.
        assert_eq!(
            error("17  APR 1871"),
            ParseError {
                kind: ParseErrorKind::ExtraSpace,
                at: 3
            }
        );
        assert_eq!(error(" 1871").kind, ParseErrorKind::ExtraSpace);
        assert_eq!(error("1871 ").kind, ParseErrorKind::ExtraSpace);

        // Keywords are case-sensitive.
        assert!(matches!(
            error("abt 1871").kind,
            ParseErrorKind::Unexpected { .. }
        ));

        assert_eq!(
            error("BET 1869").kind,
            ParseErrorKind::UnexpectedEnd { expected: "AND" }
        );
        assert_eq!(error("0").kind, ParseErrorKind::YearZero);
        assert!(matches!(
            error("1871 1872").kind,
            ParseErrorKind::Unexpected {
                expected: "the end of the date",
                ..
            }
        ));
        assert_eq!(
            error("_ISLAMIC 1300").kind,
            ParseErrorKind::Extension("_ISLAMIC".to_owned())
        );
        assert!(matches!(
            error("17 FOO 1871").kind,
            ParseErrorKind::UnknownMonth { .. }
        ));

        let february = error("30 FEB 1871");
        assert!(matches!(
            february.kind,
            ParseErrorKind::InvalidDate(DateError::DayOutOfRange {
                given: 30,
                last: 28
            })
        ));
        assert_eq!(february.at, 0, "points at the day");
        assert!(february.to_string().contains("day 30"));

        // A day that does not fit a byte is reported as the word written, not as day 255.
        assert!(matches!(
            error("400 APR 1871").kind,
            ParseErrorKind::Unexpected { .. }
        ));
    }

    #[test]
    fn a_timestamp_date_must_be_complete_and_gregorian() {
        assert_eq!(
            CalendarDate::parse_gedcom7_exact("22 SEP 2026").unwrap(),
            date(Calendar::Gregorian, 2026, Some(9), Some(22))
        );
        assert!(CalendarDate::parse_gedcom7_exact("SEP 2026").is_err());
        assert!(CalendarDate::parse_gedcom7_exact("JULIAN 22 SEP 2026").is_err());
        assert!(CalendarDate::parse_gedcom7_exact("22 SEP 2026 BCE").is_err());
    }

    #[test]
    fn leading_zeros_have_no_meaning_and_are_not_written() {
        // "Leading zeros have no semantic meaning and should be omitted."
        assert_eq!(parse("07 APR 01871").to_gedcom7(), "7 APR 1871");
    }

    #[test]
    fn a_recorded_date_writes_its_phrase_beneath() {
        let recorded =
            RecordedDate::parse_gedcom7("30 JAN 1649", Some("30 January 1648/49")).unwrap();
        assert_eq!(
            recorded.to_gedcom7(),
            (
                "30 JAN 1649".to_owned(),
                Some("30 January 1648/49".to_owned())
            )
        );
        assert_eq!(
            RecordedDate::from_phrase("during the war").to_gedcom7(),
            (String::new(), Some("during the war".to_owned()))
        );
    }

    /// Every value the grammar can express survives being written and read again.
    ///
    /// The one property an import-export format must have, checked over six thousand values in
    /// all four calendars, every precision and every form rather than over the handful a person
    /// thinks to write down.
    #[test]
    fn every_value_round_trips() {
        let mut rng = Rng::new(0xD4E7_DA7E);
        for iteration in 0..6_000 {
            let value = random_value(&mut rng);
            let written = value.to_gedcom7();
            let read = DateValue::parse_gedcom7(&written)
                .unwrap_or_else(|error| panic!("#{iteration} {written:?} did not parse: {error}"));
            assert_eq!(read, Some(value.clone()), "#{iteration} {written:?}");
        }
    }

    fn random_date(rng: &mut Rng) -> CalendarDate {
        let calendar = *rng
            .pick(&[
                Calendar::Gregorian,
                Calendar::Gregorian,
                Calendar::Julian,
                Calendar::Hebrew,
                Calendar::FrenchRepublican,
            ])
            .unwrap();
        let year = match calendar {
            Calendar::Gregorian | Calendar::Julian => rng.range(-800, 2100),
            Calendar::Hebrew => rng.range(1, 6000),
            Calendar::FrenchRepublican => rng.range(1, 40),
        };
        let precision = rng.range(0, 2);
        if precision == 0 {
            return CalendarDate::new(calendar, year, None, None).unwrap();
        }
        let months = crate::last_month_of_year(calendar, year);
        let month = u8::try_from(rng.range(1, i32::from(months))).unwrap();
        if precision == 1 {
            return CalendarDate::new(calendar, year, Some(month), None).unwrap();
        }
        let days = crate::days_in_month(calendar, year, month).unwrap();
        let day = u8::try_from(rng.range(1, i32::from(days))).unwrap();
        CalendarDate::new(calendar, year, Some(month), Some(day)).unwrap()
    }

    fn random_value(rng: &mut Rng) -> DateValue {
        match rng.range(0, 7) {
            0 => DateValue::Exact(random_date(rng)),
            1 => DateValue::Approximate {
                date: random_date(rng),
                kind: *rng
                    .pick(&[
                        Approximation::About,
                        Approximation::Calculated,
                        Approximation::Estimated,
                    ])
                    .unwrap(),
            },
            2 => DateValue::Between {
                earliest: random_date(rng),
                latest: random_date(rng),
            },
            3 => DateValue::Before(random_date(rng)),
            4 => DateValue::After(random_date(rng)),
            5 => DateValue::Period {
                from: Some(random_date(rng)),
                to: rng.chance(0.5).then(|| random_date(rng)),
            },
            _ => DateValue::Period {
                from: None,
                to: Some(random_date(rng)),
            },
        }
    }

    // ── lenient ──────────────────────────────────────────────────────────────────────────────

    #[test]
    fn valid_seven_point_zero_needs_no_repairs() {
        let read = RecordedDate::parse_lenient("JULIAN 25 OCT 1917");
        assert!(read.repairs.is_empty());
        assert_eq!(read.worst(), None);
        assert_eq!(read.date.value, Some(parse("JULIAN 25 OCT 1917")));
    }

    #[test]
    fn legacy_escapes_become_seven_point_zero_names() {
        let read = RecordedDate::parse_lenient("@#DJULIAN@ 25 OCT 1917");
        assert_eq!(read.date.value, Some(parse("JULIAN 25 OCT 1917")));
        assert!(matches!(
            read.repairs.as_slice(),
            [Repair::LegacyCalendar {
                calendar: Calendar::Julian,
                ..
            }]
        ));
        assert_eq!(read.worst(), Some(Severity::Notation));

        // The one escape with a space in it.
        let french = RecordedDate::parse_lenient("@#DFRENCH R@ 9 THER 2");
        assert_eq!(french.date.value, Some(parse("FRENCH_R 9 THER 2")));
    }

    #[test]
    fn case_and_spelling_are_notation_not_meaning() {
        let read = RecordedDate::parse_lenient("abt 17 September 1871");
        assert_eq!(read.date.value, Some(parse("ABT 17 SEP 1871")));
        assert!(read.repairs.contains(&Repair::Capitalised));
        assert!(
            read.repairs
                .iter()
                .any(|repair| matches!(repair, Repair::MonthSpelling { tag: "SEP", .. }))
        );
        assert_eq!(read.worst(), Some(Severity::Notation));

        assert_eq!(
            RecordedDate::parse_lenient("17 Apr. 1871").date.value,
            Some(parse("17 APR 1871"))
        );
        assert_eq!(
            RecordedDate::parse_lenient("44 B.C.").date.value,
            Some(parse("44 BCE"))
        );
    }

    #[test]
    fn extra_whitespace_is_forgiven() {
        let read = RecordedDate::parse_lenient("  17   APR  1871 ");
        assert_eq!(read.date.value, Some(parse("17 APR 1871")));
    }

    #[test]
    fn a_legacy_phrase_is_kept_whole() {
        let read = RecordedDate::parse_lenient("(during the war)");
        assert_eq!(read.date, RecordedDate::from_phrase("during the war"));
        assert_eq!(read.repairs, vec![Repair::LegacyPhrase]);
    }

    #[test]
    fn an_interpreted_date_keeps_both_halves() {
        let read = RecordedDate::parse_lenient("INT 1850 (about the time of the fire)");
        assert_eq!(read.date.value, Some(parse("1850")));
        assert_eq!(
            read.date.phrase.as_deref(),
            Some("about the time of the fire")
        );
        assert!(read.repairs.contains(&Repair::Interpreted));
    }

    #[test]
    fn a_dual_year_with_a_month_is_read_as_the_new_year_convention() {
        // The specification's own example: "30 January 1648/49" is DATE 30 JAN 1649.
        let read = RecordedDate::parse_lenient("30 JAN 1648/49");
        assert_eq!(read.date.value, Some(parse("30 JAN 1649")));
        assert_eq!(read.date.phrase.as_deref(), Some("30 JAN 1648/49"));
        assert!(read.repairs.contains(&Repair::DualYear {
            written: "1648/49".to_owned(),
            reading: DualReading::NewYearStyle
        }));
        assert_eq!(read.worst(), Some(Severity::Interpretation));
    }

    #[test]
    fn a_bare_dual_year_is_read_as_either_year() {
        // "1903/4", the specification's approximate-year example, is BET 1903 AND 1904 — and
        // that reading also contains the new-year one, so nothing true is excluded.
        let read = RecordedDate::parse_lenient("1903/4");
        assert_eq!(read.date.value, Some(parse("BET 1903 AND 1904")));
        assert_eq!(read.date.phrase.as_deref(), Some("1903/4"));
        assert!(read.repairs.contains(&Repair::DualYear {
            written: "1903/4".to_owned(),
            reading: DualReading::EitherYear
        }));
    }

    #[test]
    fn slashed_years_are_only_dual_when_consecutive() {
        assert_eq!(new_style_year("1648/49"), Some(1649));
        assert_eq!(new_style_year("1648/9"), Some(1649));
        assert_eq!(new_style_year("1648/1649"), Some(1649));
        assert_eq!(new_style_year("1699/00"), Some(1700));
        assert_eq!(new_style_year("1648/52"), None, "not consecutive");
        assert_eq!(new_style_year("1648/"), None);
        assert_eq!(
            new_style_year("23/6"),
            None,
            "a Julian/Gregorian day pair, not a year"
        );
    }

    #[test]
    fn adar_is_repaired_the_way_the_specification_recommends() {
        let read = RecordedDate::parse_lenient("HEBREW ADR 5785");
        assert_eq!(read.date.value, Some(parse("HEBREW ADS 5785")));
        assert!(
            read.repairs
                .contains(&Repair::AdarInCommonYear { year: 5785 })
        );
        // In a leap year ADR is Adar I and nothing needs repairing.
        assert!(
            RecordedDate::parse_lenient("HEBREW ADR 5784")
                .repairs
                .is_empty()
        );
    }

    #[test]
    fn a_calendar_nobody_defined_is_kept_as_words() {
        let read = RecordedDate::parse_lenient("@#DROMAN@ 1850");
        assert_eq!(read.date, RecordedDate::from_phrase("@#DROMAN@ 1850"));
        assert_eq!(read.worst(), Some(Severity::Unreadable));
    }

    #[test]
    fn what_cannot_be_read_is_kept_and_the_reason_recorded() {
        let read = RecordedDate::parse_lenient("the spring after the flood, 1850");
        assert_eq!(read.date.value, None);
        assert_eq!(
            read.date.phrase.as_deref(),
            Some("the spring after the flood, 1850")
        );
        assert!(matches!(
            read.repairs.last(),
            Some(Repair::Unreadable { .. })
        ));
    }

    /// The lenient reader's promise, checked against noise rather than examples: it never
    /// panics, and whatever it could not read survives as words.
    #[test]
    fn the_lenient_reader_never_loses_text() {
        const WORDS: [&str; 24] = [
            "ABT",
            "abt",
            "BET",
            "AND",
            "FROM",
            "TO",
            "INT",
            "(",
            ")",
            "@#DJULIAN@",
            "@#DROMAN@",
            "JAN",
            "Sept.",
            "ADR",
            "HEBREW",
            "1871",
            "0",
            "31",
            "1648/49",
            "B.C.",
            "BCE",
            "_X",
            "на",
            "1903/4",
        ];
        let mut rng = Rng::new(0x01E4_1E47);
        for _ in 0..4_000 {
            let length = rng.range(0, 6);
            let text = (0..length)
                .map(|_| *rng.pick(&WORDS).unwrap())
                .collect::<Vec<_>>()
                .join(" ");
            let read = RecordedDate::parse_lenient(&text);
            if read.date.value.is_none() && !text.trim().is_empty() {
                assert!(
                    read.date.phrase.is_some(),
                    "{text:?} was neither read nor kept"
                );
            }
        }
    }
}
