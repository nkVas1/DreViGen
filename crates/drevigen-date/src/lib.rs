//! Genealogical dates.
//!
//! Not a general-purpose date library, and not built on one. The dates in a family archive are
//! not timestamps: they come from four calendars, they are frequently partial, often
//! approximate, sometimes only a phrase, and the useful questions about them have three answers
//! rather than two. `docs/01-research/data-standards.md` §3 sets out the obligation; this crate
//! is the discharge of it.
//!
//! # What a date is here
//!
//! - A [`CalendarDate`] is a date **as a source wrote it**: a calendar, a year, and as much of
//!   the month and day as the source gave. A parish register entry from 1871 Russia is a Julian
//!   date and is stored as one. Converting it to the Gregorian calendar is a display decision
//!   made later, against the reader's preference, and never a storage decision.
//! - Every calendar converts through [`Day`], a single count of days. Four calendars have
//!   twelve directed conversions between them; through a pivot there are eight, and each can be
//!   checked against published tables on its own.
//!
//! # Anchors
//!
//! ```
//! use drevigen_date::{Calendar, CalendarDate};
//!
//! // The October Revolution happened in November.
//! let old_style = CalendarDate::new(Calendar::Julian, 1917, Some(10), Some(25)).unwrap();
//! let new_style = old_style.to_calendar(Calendar::Gregorian).unwrap();
//! assert_eq!((new_style.year(), new_style.month(), new_style.day()), (1917, Some(11), Some(7)));
//! ```

mod day;
pub mod french;
pub mod gregorian;
pub mod hebrew;
pub mod interval;
pub mod julian;
pub mod value;

pub use day::{Day, Weekday};
pub use interval::{Span, Trivalent};
pub use value::{Approximation, DateValue};

/// The calendars a source may have been written in.
///
/// The list is GEDCOM 7's, minus the ones it marks as extensions. Four is not a round number
/// chosen for tidiness: these are the calendars that appear in the records this project exists
/// to hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Calendar {
    /// The calendar in civil use nearly everywhere today, extended backwards without limit.
    #[default]
    Gregorian,
    /// What almost every record before the twentieth century was written in, and every Russian
    /// civil record before February 1918.
    Julian,
    /// The Hebrew calendar, for Jewish records: a birth register entry may give only the Hebrew
    /// date, and converting it on import would discard what the source said.
    Hebrew,
    /// The French Republican calendar, in official use from 1792 to 1805. French civil registers
    /// from those years are dated in it and in nothing else.
    FrenchRepublican,
}

impl Calendar {
    /// The GEDCOM 7 calendar tag.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Gregorian => "GREGORIAN",
            Self::Julian => "JULIAN",
            Self::Hebrew => "HEBREW",
            Self::FrenchRepublican => "FRENCH_R",
        }
    }
}

/// Why a date could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateError {
    /// The month is outside the range this calendar has.
    ///
    /// Carries the month that was given and the highest this calendar's year reaches, because
    /// "month 13 is invalid" is unhelpful in a calendar that sometimes has thirteen.
    MonthOutOfRange {
        /// What was given.
        given: u8,
        /// The last month of that year in that calendar.
        last: u8,
    },
    /// The day is outside the range that month has.
    DayOutOfRange {
        /// What was given.
        given: u8,
        /// The last day of that month in that year.
        last: u8,
    },
    /// A day was given without a month.
    ///
    /// "The 12th of some month in 1871" is not a date anyone records, and allowing it would
    /// make every consumer handle a case that cannot come from a real source.
    DayWithoutMonth,
    /// The year is outside the range this implementation covers.
    YearOutOfRange {
        /// What was given.
        given: i32,
    },
    /// The calendar does not reach that far back.
    ///
    /// The Hebrew calendar in its present arithmetic form and the French Republican calendar
    /// both have a first year, and dates before it are not dates in them.
    BeforeCalendarBegins,
}

impl core::fmt::Display for DateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MonthOutOfRange { given, last } => {
                write!(
                    f,
                    "month {given} does not exist; that year ends at month {last}"
                )
            }
            Self::DayOutOfRange { given, last } => {
                write!(f, "day {given} does not exist; that month ends at {last}")
            }
            Self::DayWithoutMonth => f.write_str("a day was given without a month"),
            Self::YearOutOfRange { given } => {
                write!(f, "year {given} is outside the supported range")
            }
            Self::BeforeCalendarBegins => f.write_str("that date is before the calendar begins"),
        }
    }
}

impl core::error::Error for DateError {}

/// The widest year this crate accepts, in either direction.
///
/// Well beyond any record and comfortably inside the range where the day count cannot overflow
/// an `i32`. The limit exists so that a corrupt import cannot produce arithmetic nobody
/// checked, not because anyone needs the year 200 000.
const YEAR_LIMIT: i32 = 100_000;

/// A date as a source wrote it: a calendar, a year, and as much else as was written.
///
/// Partial by design. "1871" and "April 1871" are the two commonest forms in real records, and
/// a type that cannot hold them forces the importer to invent a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CalendarDate {
    calendar: Calendar,
    year: i32,
    month: Option<u8>,
    day: Option<u8>,
}

impl CalendarDate {
    /// Builds a date, checking it against the calendar it claims to be in.
    ///
    /// # Errors
    ///
    /// Returns [`DateError`] if the month or day does not exist in that calendar and year, if a
    /// day was given without a month, or if the year is outside the supported range.
    pub fn new(
        calendar: Calendar,
        year: i32,
        month: Option<u8>,
        day: Option<u8>,
    ) -> Result<Self, DateError> {
        if !(-YEAR_LIMIT..=YEAR_LIMIT).contains(&year) {
            return Err(DateError::YearOutOfRange { given: year });
        }

        let Some(month_number) = month else {
            return if day.is_some() {
                Err(DateError::DayWithoutMonth)
            } else {
                Ok(Self {
                    calendar,
                    year,
                    month,
                    day,
                })
            };
        };

        let last_month = last_month_of_year(calendar, year);
        if month_number < 1 || month_number > last_month {
            return Err(DateError::MonthOutOfRange {
                given: month_number,
                last: last_month,
            });
        }

        if let Some(day_number) = day {
            let last_day =
                days_in_month(calendar, year, month_number).ok_or(DateError::MonthOutOfRange {
                    given: month_number,
                    last: last_month,
                })?;
            if day_number < 1 || day_number > last_day {
                return Err(DateError::DayOutOfRange {
                    given: day_number,
                    last: last_day,
                });
            }
        }

        Ok(Self {
            calendar,
            year,
            month,
            day,
        })
    }

    /// The calendar this date is written in.
    #[must_use]
    pub const fn calendar(self) -> Calendar {
        self.calendar
    }

    /// The year, in astronomical numbering: 0 is 1 BCE.
    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    /// The month, if the source gave one.
    #[must_use]
    pub const fn month(self) -> Option<u8> {
        self.month
    }

    /// The day of the month, if the source gave one.
    #[must_use]
    pub const fn day(self) -> Option<u8> {
        self.day
    }

    /// Whether every field is present.
    #[must_use]
    pub const fn is_exact(self) -> bool {
        self.day.is_some()
    }

    /// The first day this date could mean.
    ///
    /// For a complete date that is the day itself. For "April 1871" it is 1 April; for "1871"
    /// it is 1 January. This is half of what makes a partial date comparable at all.
    #[must_use]
    pub fn earliest(self) -> Day {
        let month = self.month.unwrap_or(1);
        let day = self.day.unwrap_or(1);
        to_day(self.calendar, self.year, month, day)
    }

    /// The last day this date could mean.
    ///
    /// For "1871" that is 31 December 1871 — or the last day of whatever the year's final month
    /// is, which in the Hebrew calendar depends on the year.
    #[must_use]
    pub fn latest(self) -> Day {
        let month = self
            .month
            .unwrap_or_else(|| last_month_of_year(self.calendar, self.year));
        let day = self
            .day
            .unwrap_or_else(|| days_in_month(self.calendar, self.year, month).unwrap_or(30));
        to_day(self.calendar, self.year, month, day)
    }

    /// Restates this date in another calendar.
    ///
    /// Only a complete date converts. "April 1871" has no counterpart in another calendar
    /// because its month boundaries fall inside two of the other's, and returning something
    /// that looks like an answer would be worse than returning nothing.
    ///
    /// # Errors
    ///
    /// Returns [`DateError`] if the date is not complete, or if it falls before the target
    /// calendar begins.
    pub fn to_calendar(self, target: Calendar) -> Result<Self, DateError> {
        if self.calendar == target {
            return Ok(self);
        }
        if !self.is_exact() {
            return Err(DateError::DayWithoutMonth);
        }
        let (year, month, day) = from_day(target, self.earliest())?;
        Self::new(target, year, Some(month), Some(day))
    }

    /// The day of the week, for a complete date.
    #[must_use]
    pub fn weekday(self) -> Option<Weekday> {
        self.is_exact().then(|| self.earliest().weekday())
    }
}

/// The last month number of a year in a calendar.
fn last_month_of_year(calendar: Calendar, year: i32) -> u8 {
    match calendar {
        Calendar::Gregorian | Calendar::Julian => 12,
        // Twelve or thirteen, depending on where the year falls in the Metonic cycle.
        Calendar::Hebrew => hebrew::months_in_year(year),
        Calendar::FrenchRepublican => french::months_in_year(year),
    }
}

/// Days in a month of a calendar, or `None` if the month does not exist.
fn days_in_month(calendar: Calendar, year: i32, month: u8) -> Option<u8> {
    match calendar {
        Calendar::Gregorian => gregorian::days_in_month(year, month),
        Calendar::Julian => julian::days_in_month(year, month),
        Calendar::Hebrew => hebrew::days_in_month(year, month),
        Calendar::FrenchRepublican => french::days_in_month(year, month),
    }
}

/// Converts a complete date in any calendar to a day number.
fn to_day(calendar: Calendar, year: i32, month: u8, day: u8) -> Day {
    match calendar {
        Calendar::Gregorian => gregorian::to_day(year, month, day),
        Calendar::Julian => julian::to_day(year, month, day),
        Calendar::Hebrew => hebrew::to_day(year, month, day),
        // A Republican year before the Republic cannot be constructed, so this cannot fail
        // for a date that exists; the fallback keeps the signature total.
        Calendar::FrenchRepublican => {
            french::to_day(year, month, day).unwrap_or_else(|| gregorian::to_day(year, month, day))
        }
    }
}

/// Converts a day number to a date in any calendar.
fn from_day(calendar: Calendar, day: Day) -> Result<(i32, u8, u8), DateError> {
    match calendar {
        Calendar::Gregorian => Ok(gregorian::from_day(day)),
        Calendar::Julian => Ok(julian::from_day(day)),
        Calendar::Hebrew => hebrew::from_day(day).ok_or(DateError::BeforeCalendarBegins),
        Calendar::FrenchRepublican => french::from_day(day).ok_or(DateError::BeforeCalendarBegins),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{Calendar, CalendarDate, DateError};

    #[test]
    fn a_partial_date_is_a_date() {
        let year_only = CalendarDate::new(Calendar::Gregorian, 1871, None, None).unwrap();
        assert!(!year_only.is_exact());
        assert_eq!(year_only.earliest(), super::gregorian::to_day(1871, 1, 1));
        assert_eq!(year_only.latest(), super::gregorian::to_day(1871, 12, 31));

        let month = CalendarDate::new(Calendar::Gregorian, 1871, Some(4), None).unwrap();
        assert_eq!(month.earliest(), super::gregorian::to_day(1871, 4, 1));
        assert_eq!(month.latest(), super::gregorian::to_day(1871, 4, 30));
    }

    #[test]
    fn a_day_without_a_month_is_not_a_date() {
        assert_eq!(
            CalendarDate::new(Calendar::Gregorian, 1871, None, Some(12)),
            Err(DateError::DayWithoutMonth)
        );
    }

    #[test]
    fn the_calendar_decides_whether_a_date_exists() {
        // 29 February 1900 is a real Julian date and not a real Gregorian one. A tool that
        // validates against the wrong calendar rejects a correctly transcribed record.
        assert!(CalendarDate::new(Calendar::Julian, 1900, Some(2), Some(29)).is_ok());
        assert_eq!(
            CalendarDate::new(Calendar::Gregorian, 1900, Some(2), Some(29)),
            Err(DateError::DayOutOfRange {
                given: 29,
                last: 28
            })
        );
    }

    #[test]
    fn an_error_says_what_would_have_been_valid() {
        let error = CalendarDate::new(Calendar::Gregorian, 1871, Some(13), None).unwrap_err();
        assert_eq!(
            error,
            DateError::MonthOutOfRange {
                given: 13,
                last: 12
            }
        );
        assert_eq!(
            error.to_string(),
            "month 13 does not exist; that year ends at month 12"
        );
    }

    #[test]
    fn converting_an_incomplete_date_is_refused_rather_than_guessed() {
        let month = CalendarDate::new(Calendar::Julian, 1871, Some(4), None).unwrap();
        assert_eq!(
            month.to_calendar(Calendar::Gregorian),
            Err(DateError::DayWithoutMonth)
        );
    }

    #[test]
    fn a_year_far_outside_any_record_is_refused() {
        assert_eq!(
            CalendarDate::new(Calendar::Gregorian, 900_000, None, None),
            Err(DateError::YearOutOfRange { given: 900_000 })
        );
    }
}
