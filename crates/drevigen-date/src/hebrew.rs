//! The Hebrew calendar, in the arithmetic form fixed by Hillel II.
//!
//! Jewish vital records give the Hebrew date, often only the Hebrew date, and converting on
//! import would throw away what the document says. So it is stored as written and converted
//! for display, like every other calendar here.
//!
//! # What makes this one hard
//!
//! A Hebrew year has twelve or thirteen months and 353, 354, 355, 383, 384 or 385 days. The
//! length is not a property of the year in isolation: two of the months are lengthened or
//! shortened to push the new year off certain weekdays, and which correction applies depends on
//! when the *next* year's new moon falls. So the length of Kislev in 5706 is a fact about 5707.
//!
//! The rules are the *dechiyot*, the postponements, and they exist for liturgical reasons: Yom
//! Kippur must not fall next to a Sabbath, and Hoshana Rabbah must not fall on one. Everything
//! below follows Dershowitz and Reingold, *Calendrical Calculations*, which states them as
//! arithmetic.
//!
//! # Month numbering
//!
//! Months are numbered **in the order the year runs**: Tishri is 1, Elul is 12 or 13. This is
//! the order GEDCOM 7 lists its tags in, and the order a month list has to be shown in. It is
//! *not* the numbering in the literature, where Nisan is 1 — that numbering is a religious
//! ordering, and the conversion between the two happens inside this module.
//!
//! In a leap year an extra month is inserted before Nisan: month 6 is Adar I and month 7 is
//! Adar II. In a common year month 6 is the only Adar and Nisan is month 7. That shift is why
//! a Hebrew month cannot be stored as a bare number without its year.

use crate::day::{Day, mod_floor};
use crate::julian;

/// Tishri 1 of Hebrew year 1, which is 7 October 3761 BCE in the Julian calendar.
const fn epoch() -> Day {
    // Astronomical year -3760 is the historical 3761 BCE.
    julian::to_day(-3760, 10, 7)
}

/// Whether a Hebrew year is a leap year: seven in every nineteen.
///
/// The Metonic cycle. Nineteen solar years are almost exactly 235 lunar months, so seven of the
/// nineteen carry a thirteenth month.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    mod_floor(7 * year + 1, 19) < 7
}

/// The number of months in a Hebrew year, which is also its last month number.
#[must_use]
pub const fn months_in_year(year: i32) -> u8 {
    if is_leap_year(year) { 13 } else { 12 }
}

/// Days from the epoch to the new year, before the postponement rules are applied.
///
/// The molad — the mean new moon — is tracked in *parts*, of which there are 1080 in an hour
/// and so 25 920 in a day. The constants are the classical ones: the first molad fell 12 084
/// parts into its day, and a lunation is 29 days 13 753 parts.
fn elapsed_days(year: i32) -> i32 {
    let months_elapsed = div_floor_i64(235 * i64::from(year) - 234, 19);
    let parts_elapsed = 12_084 + 13_753 * months_elapsed;
    let day = 29 * months_elapsed + div_floor_i64(parts_elapsed, 25_920);

    // The first postponement: the new year may not fall on a Sunday, Wednesday or Friday.
    // Expressed as a residue, that is what this test comes to.
    let day = if mod_floor_i64(3 * (day + 1), 7) < 3 {
        day + 1
    } else {
        day
    };

    // Safe: the year is bounded well inside the range where this fits.
    day as i32
}

/// The remaining two postponements, which lengthen the *previous* year rather than this one.
///
/// A year cannot be 356 days long, nor 382; when the arithmetic produces one, a day is moved.
fn year_length_correction(year: i32) -> i32 {
    let last = elapsed_days(year - 1);
    let this = elapsed_days(year);
    let next = elapsed_days(year + 1);

    if next - this == 356 {
        // The year would have been 356 days, which is not a permitted length, so it is
        // pushed two days.
        2
    } else {
        // 382 is likewise impossible; one day settles it. Neither branch is a boolean in
        // disguise, whatever the shape suggests.
        i32::from(this - last == 382)
    }
}

/// The day on which 1 Tishri of a Hebrew year falls.
#[must_use]
pub fn new_year(year: i32) -> Day {
    epoch().offset(elapsed_days(year) + year_length_correction(year))
}

/// The length of a Hebrew year in days: one of 353, 354, 355, 383, 384 or 385.
#[must_use]
pub fn days_in_year(year: i32) -> i32 {
    new_year(year).days_until(new_year(year + 1))
}

/// Whether Marheshvan is long this year. It is the month that absorbs a needed extra day.
fn is_marheshvan_long(year: i32) -> bool {
    matches!(days_in_year(year), 355 | 385)
}

/// Whether Kislev is short this year. It is the month that gives a day up.
fn is_kislev_short(year: i32) -> bool {
    matches!(days_in_year(year), 353 | 383)
}

/// Days in a Hebrew month, numbered in the order the year runs.
///
/// Returns `None` for a month the year does not have — which for month 13 depends on the year,
/// and is the reason this takes one.
#[must_use]
pub fn days_in_month(year: i32, month: u8) -> Option<u8> {
    if month < 1 || month > months_in_year(year) {
        return None;
    }
    // Every month alternates 30, 29, 30, 29 from Tishri, and the two variable months are the
    // exception that keeps the alternation in step with the moon. The leap month is inserted
    // as a 30-day Adar I, which is why the parity of the second half of the year flips.
    Some(match month {
        // Marheshvan and Kislev, the two that absorb the correction, and Tevet after them.
        2 if is_marheshvan_long(year) => 30,
        3 if !is_kislev_short(year) => 30,
        2..=4 => 29,
        m if is_leap_year(year) => {
            if m >= 7 && m % 2 == 1 {
                29
            } else {
                30
            }
        }
        m => {
            if m >= 6 && m % 2 == 0 {
                29
            } else {
                30
            }
        }
    })
}

/// Converts a Hebrew date to a day number.
///
/// The month is in year order; see the module documentation.
#[must_use]
pub fn to_day(year: i32, month: u8, day: u8) -> Day {
    let mut fixed = new_year(year).fixed();
    let mut earlier = 1;
    while earlier < month {
        fixed += i32::from(days_in_month(year, earlier).unwrap_or(30));
        earlier += 1;
    }
    Day::new(fixed + i32::from(day) - 1)
}

/// Converts a day number to a Hebrew date, as `(year, month, day)` with the month in year order.
///
/// Returns `None` for a day before the calendar's epoch, which is not a Hebrew date.
#[must_use]
pub fn from_day(day: Day) -> Option<(i32, u8, u8)> {
    let since_epoch = epoch().days_until(day);
    if since_epoch < 0 {
        return None;
    }

    // The mean Hebrew year is 35975351/98496 days, so this lands within one year of the answer
    // and the loop below closes the gap.
    let approx = div_floor_i64(i64::from(since_epoch) * 98_496, 35_975_351) as i32 + 1;

    let mut year = approx - 1;
    while new_year(year + 1).fixed() <= day.fixed() {
        year += 1;
    }

    let mut remaining = new_year(year).days_until(day);
    let mut month: u8 = 1;
    loop {
        let length = i32::from(days_in_month(year, month)?);
        if remaining < length {
            break;
        }
        remaining -= length;
        month += 1;
    }

    // Safe: remaining is now less than the month's length, which is at most 30.
    Some((year, month, (remaining + 1) as u8))
}

/// Floor division for the wider values the molad arithmetic needs.
const fn div_floor_i64(a: i64, b: i64) -> i64 {
    a.div_euclid(b)
}

/// The non-negative remainder, likewise.
const fn mod_floor_i64(a: i64, b: i64) -> i64 {
    a.rem_euclid(b)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{
        days_in_month, days_in_year, epoch, from_day, is_leap_year, months_in_year, new_year,
        to_day,
    };
    use crate::gregorian;

    /// The Gregorian date naming the same day as a Hebrew one.
    fn as_gregorian(y: i32, m: u8, d: u8) -> (i32, u8, u8) {
        gregorian::from_day(to_day(y, m, d))
    }

    #[test]
    fn the_epoch_is_where_the_literature_puts_it() {
        // 1 Tishri 1 = 7 October 3761 BCE (Julian) = fixed day -1 373 427.
        assert_eq!(epoch().fixed(), -1_373_427);
        assert_eq!(new_year(1), epoch());
    }

    #[test]
    fn seven_years_in_nineteen_are_leap_years() {
        let leaps = (1..=19).filter(|&y| is_leap_year(y)).count();
        assert_eq!(leaps, 7);
        // The classical positions in the cycle: 3, 6, 8, 11, 14, 17, 19.
        let positions: Vec<i32> = (1..=19).filter(|&y| is_leap_year(y)).collect();
        assert_eq!(positions, vec![3, 6, 8, 11, 14, 17, 19]);
        // 5784 was a leap year and 5785 was not.
        assert!(is_leap_year(5784));
        assert!(!is_leap_year(5785));
    }

    #[test]
    fn rosh_hashanah_falls_where_the_calendars_say() {
        // 1 Tishri 5784 was 16 September 2023; 1 Tishri 5785 was 3 October 2024.
        assert_eq!(as_gregorian(5784, 1, 1), (2023, 9, 16));
        assert_eq!(as_gregorian(5785, 1, 1), (2024, 10, 3));
        assert_eq!(as_gregorian(5786, 1, 1), (2025, 9, 23));
    }

    #[test]
    fn the_new_year_never_falls_on_a_forbidden_weekday() {
        // The first postponement, checked rather than assumed: Sunday, Wednesday and Friday are
        // excluded so that Yom Kippur does not abut a Sabbath and Hoshana Rabbah does not fall
        // on one.
        use crate::day::Weekday::{Friday, Sunday, Wednesday};
        for year in 5000..5400 {
            let weekday = new_year(year).weekday();
            assert!(
                !matches!(weekday, Sunday | Wednesday | Friday),
                "Hebrew {year} began on a {weekday:?}"
            );
        }
    }

    #[test]
    fn a_year_is_one_of_six_lengths() {
        for year in 4000..6000 {
            let length = days_in_year(year);
            assert!(
                matches!(length, 353 | 354 | 355 | 383 | 384 | 385),
                "Hebrew {year} was {length} days long"
            );
            let expected_leap = length > 360;
            assert_eq!(expected_leap, is_leap_year(year), "Hebrew {year}");
        }
    }

    #[test]
    fn the_months_add_up_to_the_year() {
        for year in 5700..5820 {
            let summed: i32 = (1..=months_in_year(year))
                .map(|m| i32::from(days_in_month(year, m).unwrap()))
                .sum();
            assert_eq!(summed, days_in_year(year), "Hebrew {year}");
        }
    }

    #[test]
    fn only_two_months_ever_change_length() {
        // Marheshvan and Kislev absorb the correction; everything else is fixed. If a third
        // month starts varying, the postponement logic has leaked.
        for year in 5700..5900 {
            let leap = is_leap_year(year);
            for month in 1..=months_in_year(year) {
                let length = days_in_month(year, month).unwrap();
                if month == 2 || month == 3 {
                    continue;
                }
                // Spelled out by name rather than by the formula the code uses, so that the
                // test would survive the formula being wrong.
                let expected = if leap {
                    match month {
                        1 | 5 | 6 | 8 | 10 | 12 => 30, // Tishri Shevat AdarI Nisan Sivan Av
                        4 | 7 | 9 | 11 | 13 => 29,     // Tevet AdarII Iyar Tammuz Elul
                        _ => unreachable!("month {month} is Marheshvan or Kislev"),
                    }
                } else {
                    match month {
                        1 | 5 | 7 | 9 | 11 => 30,  // Tishri Shevat Nisan Sivan Av
                        4 | 6 | 8 | 10 | 12 => 29, // Tevet Adar Iyar Tammuz Elul
                        _ => unreachable!("month {month} is Marheshvan or Kislev"),
                    }
                };
                assert_eq!(length, expected, "Hebrew {year} month {month}");
            }
        }
    }

    #[test]
    fn the_leap_month_pushes_nisan_along() {
        // In a common year Nisan is the seventh month of the year; in a leap year it is the
        // eighth, because Adar I is inserted before it. Passover is on 15 Nisan, so getting
        // this wrong moves a festival by a month.
        let common = 5785;
        let leap = 5784;
        assert!(!is_leap_year(common));
        assert!(is_leap_year(leap));

        // 15 Nisan 5785 was 13 April 2025; 15 Nisan 5784 was 23 April 2024.
        assert_eq!(as_gregorian(common, 7, 15), (2025, 4, 13));
        assert_eq!(as_gregorian(leap, 8, 15), (2024, 4, 23));
    }

    #[test]
    fn the_conversion_is_a_bijection_over_three_centuries() {
        let start = to_day(5600, 1, 1).fixed();
        let end = to_day(5900, 1, 1).fixed();
        for fixed in start..end {
            let day = crate::Day::new(fixed);
            let (y, m, d) = from_day(day).unwrap();
            assert!(
                m >= 1 && m <= months_in_year(y),
                "day {fixed} gave month {m} of a {}-month year",
                months_in_year(y)
            );
            assert!(d >= 1 && d <= days_in_month(y, m).unwrap());
            assert_eq!(
                to_day(y, m, d),
                day,
                "Hebrew {y}-{m}-{d} did not round-trip"
            );
        }
    }

    #[test]
    fn a_day_before_the_epoch_is_not_a_hebrew_date() {
        assert_eq!(from_day(epoch().offset(-1)), None);
        assert!(from_day(epoch()).is_some());
    }
}
