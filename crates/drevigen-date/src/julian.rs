//! The Julian calendar — the one most of the sources were written in.
//!
//! For Russian genealogy this is not a historical curiosity, it is the default. Civil records
//! in Russia used the Julian calendar until February 1918; parish registers, revision lists and
//! metrical books are all Julian, and the difference against the Gregorian calendar is ten days
//! in the sixteenth century and thirteen in the twentieth. A birth "on 17 April 1871" is Julian
//! unless something says otherwise, and reading it as Gregorian moves the child by twelve days.
//!
//! The rule is simply every fourth year, with no century exception. That is why the two
//! calendars drift apart by about three days every four hundred years.
//!
//! # Years
//!
//! Stored **astronomically**, as in [`crate::gregorian`]: year 0 is 1 BCE. Historians writing
//! about the Julian calendar normally skip zero, so the conversion happens here, once, rather
//! than in every caller.

use crate::day::{Day, div_floor};

/// The day on which 1 January 1 CE (Julian) falls: two days before the Gregorian epoch.
const EPOCH: i32 = -1;

/// Whether a Julian year is a leap year.
///
/// Every fourth year, including the centuries that the Gregorian reform later excluded.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    // In astronomical numbering this is the same test as the Gregorian one without the
    // century exception. In historical numbering it would not be: the leap years before the
    // common era are 1, 5 and 9 BCE, and skipping a year zero shifts all of them. Storing
    // astronomical years is what keeps this line short.
    year % 4 == 0
}

/// Days in a Julian month.
///
/// Returns `None` for a month outside 1..=12.
#[must_use]
pub const fn days_in_month(year: i32, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if is_leap_year(year) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

/// Converts a Julian date to a day number.
#[must_use]
pub const fn to_day(year: i32, month: u8, day: u8) -> Day {
    let y = year - 1;
    let m = month as i32;

    let years = 365 * y + div_floor(y, 4);
    let months = div_floor(367 * m - 362, 12);
    let february = if month <= 2 {
        0
    } else if is_leap_year(year) {
        -1
    } else {
        -2
    };

    Day::new(EPOCH - 1 + years + months + february + day as i32)
}

/// Converts a day number to a Julian date, as `(year, month, day)` with an astronomical year.
#[must_use]
pub const fn from_day(day: Day) -> (i32, u8, u8) {
    // Four Julian years are exactly 1461 days, so the year is one division away — no search,
    // no cycle counting. The 1464 is the epoch offset folded into the numerator.
    let year = div_floor(4 * (day.fixed() - EPOCH) + 1464, 1461);

    let january_first = to_day(year, 1, 1);
    let prior_days = january_first.days_until(day);

    let correction = if day.fixed() < to_day(year, 3, 1).fixed() {
        0
    } else if is_leap_year(year) {
        1
    } else {
        2
    };
    let month = div_floor(12 * (prior_days + correction) + 373, 367);
    let month_u8 = month as u8;

    let day_of_month = to_day(year, month_u8, 1).days_until(day) + 1;
    let day_u8 = day_of_month as u8;

    (year, month_u8, day_u8)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{days_in_month, from_day, is_leap_year, to_day};
    use crate::day::Day;
    use crate::gregorian;

    /// Converts a Julian date to the Gregorian date naming the same day.
    fn as_gregorian(y: i32, m: u8, d: u8) -> (i32, u8, u8) {
        gregorian::from_day(to_day(y, m, d))
    }

    #[test]
    fn the_century_years_are_leap_years_here() {
        assert!(is_leap_year(1900), "no century rule in this calendar");
        assert!(is_leap_year(1800));
        assert!(is_leap_year(1700));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1901));
        assert_eq!(days_in_month(1900, 2), Some(29));
        assert_eq!(gregorian::days_in_month(1900, 2), Some(28));
    }

    #[test]
    fn the_october_revolution_happened_in_november() {
        // The single most-quoted Julian-to-Gregorian conversion there is: 25 October 1917 in
        // the old style is 7 November in the new. If this test fails, the calendar is wrong.
        assert_eq!(as_gregorian(1917, 10, 25), (1917, 11, 7));
    }

    #[test]
    fn the_gregorian_reform_lost_ten_days() {
        // Thursday 4 October 1582 was followed by Friday 15 October 1582: the reform skipped
        // ten days, so the Julian 4 October and the Gregorian 14 October are the same day.
        assert_eq!(as_gregorian(1582, 10, 4), (1582, 10, 14));
        // And the next day is the first Gregorian day.
        assert_eq!(as_gregorian(1582, 10, 5), (1582, 10, 15));
    }

    #[test]
    fn the_drift_grows_by_three_days_every_four_centuries() {
        // The difference is what a genealogist has to know per century, so it is pinned per
        // century rather than asserted in general.
        let difference = |y: i32| {
            let (_, _, julian_as_gregorian_day) = as_gregorian(y, 3, 1);
            i32::from(julian_as_gregorian_day) - 1
        };
        assert_eq!(difference(1500), 10);
        assert_eq!(difference(1600), 10);
        assert_eq!(difference(1700), 11);
        assert_eq!(difference(1800), 12);
        assert_eq!(difference(1900), 13);
        assert_eq!(difference(2100), 14);
    }

    #[test]
    fn russia_changed_calendar_by_deleting_thirteen_days() {
        // The decree: 31 January 1918 (Julian) was followed by 14 February 1918 (Gregorian).
        assert_eq!(as_gregorian(1918, 1, 31), (1918, 2, 13));
        assert_eq!(as_gregorian(1918, 2, 1), (1918, 2, 14));
    }

    #[test]
    fn britain_changed_calendar_by_deleting_eleven_days() {
        // 2 September 1752 was followed by 14 September 1752.
        assert_eq!(as_gregorian(1752, 9, 2), (1752, 9, 13));
        assert_eq!(as_gregorian(1752, 9, 3), (1752, 9, 14));
    }

    #[test]
    fn the_conversion_is_a_bijection_over_five_centuries() {
        let start = to_day(1600, 1, 1).fixed();
        let end = to_day(2100, 1, 1).fixed();
        for fixed in start..end {
            let day = Day::new(fixed);
            let (y, m, d) = from_day(day);
            assert!((1..=12).contains(&m), "day {fixed} gave month {m}");
            assert!(
                d >= 1 && d <= days_in_month(y, m).unwrap(),
                "day {fixed} gave {y}-{m}-{d}"
            );
            assert_eq!(to_day(y, m, d), day, "{y}-{m}-{d} did not round-trip");
        }
    }

    #[test]
    fn there_is_no_year_zero_to_fall_into() {
        // 1 BCE is the astronomical year 0 and the historical year -1, and the day before
        // 1 January 1 CE must be 31 December 1 BCE with no gap and no repetition.
        let first_ce = to_day(1, 1, 1);
        let last_bce = Day::new(first_ce.fixed() - 1);
        assert_eq!(from_day(last_bce), (0, 12, 31));

        let start = to_day(-5, 1, 1).fixed();
        let end = to_day(5, 12, 31).fixed();
        for fixed in start..=end {
            let day = Day::new(fixed);
            let (y, m, d) = from_day(day);
            assert_eq!(to_day(y, m, d), day, "day {fixed} became {y}-{m}-{d}");
        }
    }

    #[test]
    fn the_two_calendars_agree_in_the_third_century() {
        // They were two days apart at the start of the common era and coincided from
        // 1 March 200 to 28 February 300, which is a distinctive enough fact to test with.
        assert_eq!(as_gregorian(200, 3, 1), (200, 3, 1));
        assert_eq!(as_gregorian(300, 2, 28), (300, 2, 28));
        // 300 is a Julian leap year and not a Gregorian one, which is where they part again.
        assert_eq!(as_gregorian(300, 2, 29), (300, 3, 1));
        assert_eq!(as_gregorian(300, 3, 1), (300, 3, 2));
    }
}
