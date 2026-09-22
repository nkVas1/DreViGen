//! The Gregorian calendar, extended backwards without limit.
//!
//! **Proleptic**, which is a decision rather than a convenience: a date written 1500-03-01 in
//! this calendar means the day that is 1500-03-01 by Gregorian rules, not the day the writer of
//! a 1500 document meant. A source from 1500 used the Julian calendar, and recording what it
//! says belongs in [`crate::julian`]. Storing the source's own calendar and converting for
//! display is the rule from `docs/01-research/data-standards.md` §3.
//!
//! Years are **astronomical**: year 0 exists and is 1 BCE, year -1 is 2 BCE. The historical
//! numbering that skips zero is a presentation concern, handled where dates are parsed and
//! formatted, so that arithmetic never has to step over a year that is not there.

use crate::day::{Day, div_floor};

/// Whether a Gregorian year is a leap year.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Days in a Gregorian month.
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

/// Converts a Gregorian date to a day number.
///
/// The date is not validated here; [`crate::CalendarDate`] does that before calling. Passing
/// 31 February yields the day that arithmetic says it is, which is 3 March.
#[must_use]
pub const fn to_day(year: i32, month: u8, day: u8) -> Day {
    let y = year - 1;
    let m = month as i32;

    // Days in the whole years before this one, with the leap rule applied at all three periods.
    let years = 365 * y + div_floor(y, 4) - div_floor(y, 100) + div_floor(y, 400);

    // Days in the whole months before this one, if every month were 30.6 days, then corrected
    // for the short February once the year is known to have started.
    let months = div_floor(367 * m - 362, 12);
    let february = if month <= 2 {
        0
    } else if is_leap_year(year) {
        -1
    } else {
        -2
    };

    Day::new(years + months + february + day as i32)
}

/// Converts a day number to a Gregorian date, as `(year, month, day)`.
#[must_use]
pub const fn from_day(day: Day) -> (i32, u8, u8) {
    let year = year_from_day(day);

    let january_first = to_day(year, 1, 1);
    let prior_days = january_first.days_until(day);

    // March is where the leap day has already happened or not; before it no correction is due.
    let correction = if day.fixed() < to_day(year, 3, 1).fixed() {
        0
    } else if is_leap_year(year) {
        1
    } else {
        2
    };
    let month = div_floor(12 * (prior_days + correction) + 373, 367);

    // Both fit: the formula yields 1..=12, and the day of month 1..=31.
    let month_u8 = month as u8;

    let day_of_month = to_day(year, month_u8, 1).days_until(day) + 1;

    let day_u8 = day_of_month as u8;

    (year, month_u8, day_u8)
}

/// The Gregorian year containing a day.
///
/// Counts whole 400-, 100-, 4- and 1-year cycles out of the elapsed days. The two `== 4` tests
/// are the leap-day boundary: on 31 December of a leap year the count has already rolled into
/// the next cycle, and adding one would name the wrong year.
#[must_use]
pub const fn year_from_day(day: Day) -> i32 {
    let d0 = day.fixed() - 1;

    let n400 = div_floor(d0, 146_097);
    let d1 = d0 - 146_097 * n400;

    let n100 = div_floor(d1, 36_524);
    let d2 = d1 - 36_524 * n100;

    let n4 = div_floor(d2, 1_461);
    let d3 = d2 - 1_461 * n4;

    let n1 = div_floor(d3, 365);

    let years = 400 * n400 + 100 * n100 + 4 * n4 + n1;
    if n100 == 4 || n1 == 4 {
        years
    } else {
        years + 1
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{days_in_month, from_day, is_leap_year, to_day};
    use crate::day::Day;

    #[test]
    fn the_century_rule_is_the_whole_point_of_the_calendar() {
        assert!(is_leap_year(2000), "divisible by 400");
        assert!(!is_leap_year(1900), "divisible by 100 and not 400");
        assert!(!is_leap_year(1800));
        assert!(!is_leap_year(1700));
        assert!(is_leap_year(1600));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn the_leap_rule_holds_before_the_common_era_too() {
        // Astronomical numbering: year 0 is 1 BCE, and it is divisible by 400.
        assert!(is_leap_year(0));
        assert!(is_leap_year(-4), "5 BCE");
        assert!(!is_leap_year(-1), "2 BCE");
        assert!(!is_leap_year(-100), "101 BCE, divisible by 100 and not 400");
    }

    #[test]
    fn published_anchors() {
        // 1 January 1 is day one; that is the definition this module is built on.
        assert_eq!(to_day(1, 1, 1), Day::new(1));
        // J2000.0.
        assert_eq!(to_day(2000, 1, 1), Day::new(730_120));
        assert_eq!(to_day(2000, 1, 1).julian_day_number(), 2_451_545);
        // Dershowitz and Reingold's worked sample.
        assert_eq!(to_day(1945, 11, 12), Day::new(710_347));
        // The day after day one is the second of January, not a new year.
        assert_eq!(from_day(Day::new(2)), (1, 1, 2));
    }

    #[test]
    fn the_conversion_is_a_bijection_over_five_centuries() {
        // Every day from 1600 to 2100, both directions. Cheap, and it catches the boundary
        // bugs that a handful of spot checks walk straight past.
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
    fn the_conversion_is_a_bijection_across_the_era_boundary() {
        // 5 BCE to 5 CE. The astronomical year zero and the negative-year division are the two
        // places this arithmetic goes wrong, and both are here.
        let start = to_day(-5, 1, 1).fixed();
        let end = to_day(5, 12, 31).fixed();
        for fixed in start..=end {
            let day = Day::new(fixed);
            let (y, m, d) = from_day(day);
            assert_eq!(to_day(y, m, d), day, "day {fixed} became {y}-{m}-{d}");
        }
        // 31 December 1 BCE is the day before 1 January 1 CE.
        assert_eq!(from_day(Day::new(0)), (0, 12, 31));
    }

    #[test]
    fn february_knows_which_year_it_is_in() {
        assert_eq!(days_in_month(2024, 2), Some(29));
        assert_eq!(days_in_month(2023, 2), Some(28));
        assert_eq!(days_in_month(1900, 2), Some(28));
        assert_eq!(days_in_month(2000, 2), Some(29));
        assert_eq!(days_in_month(2024, 13), None);
        assert_eq!(days_in_month(2024, 0), None);
    }
}
