//! The French Republican calendar.
//!
//! In force from 22 September 1792 to 31 December 1805 — years I to XIV — and during that
//! period it was the *only* legal calendar in France. French civil registers, military records
//! and notarial acts from those thirteen years carry Republican dates and nothing else, so a
//! tool that cannot read them cannot read French records from the Revolution.
//!
//! Twelve months of thirty days, each divided into three ten-day weeks, followed by five
//! complementary days — six in a sextile year — which belonged to no month. Those days are
//! held here as a thirteenth month, which is what GEDCOM 7 does with its `COMP` tag.
//!
//! # The leap rule, and why this module has a table in it
//!
//! The decree tied the new year to the **autumn equinox at Paris**, observed. That is not an
//! arithmetic rule, and it produced sextile years III, VII and XI. Charles-Gilbert Romme
//! proposed an arithmetic replacement whose leap years are the ones divisible by four; it was
//! never enacted, because the calendar was abolished first.
//!
//! So the two rules disagree, and for the years that actually happened only the equinox rule is
//! correct. Those years are therefore a table of observed facts rather than a formula. Beyond
//! year XIV the calendar was never in force, and a date there can only be a reconstruction;
//! those use Romme's rule and are documented as the reconstruction they are.
//!
//! Dates before year I are refused. A "Republican" date before the Republic is not a date.

use crate::day::Day;
use crate::gregorian;

/// The first day of each Republican year, as a Gregorian date.
///
/// Years I to XV. The fifteenth is here so that year XIV has an end; the calendar was abolished
/// on 31 December 1805, three months into it, and 23 September 1806 is where the equinox would
/// have put the next new year.
///
/// These are observations, not arithmetic: the drift between 22 and 24 September is the
/// equinox moving against the Gregorian calendar, which is precisely what the decree tied the
/// year to.
const NEW_YEARS: [(i32, u8, u8); 15] = [
    (1792, 9, 22),
    (1793, 9, 22),
    (1794, 9, 22),
    (1795, 9, 23),
    (1796, 9, 22),
    (1797, 9, 22),
    (1798, 9, 22),
    (1799, 9, 23),
    (1800, 9, 23),
    (1801, 9, 23),
    (1802, 9, 23),
    (1803, 9, 24),
    (1804, 9, 23),
    (1805, 9, 23),
    (1806, 9, 23),
];

/// The last Republican year the calendar was actually in force for.
pub const LAST_OFFICIAL_YEAR: i32 = 14;

/// Whether a Republican year has six complementary days rather than five.
///
/// For years I to XIV this is a fact, derived from when the equinox fell. Beyond them it is
/// Romme's never-enacted arithmetic rule, and the answer is a reconstruction.
#[must_use]
pub fn is_sextile_year(year: i32) -> bool {
    if (1..=LAST_OFFICIAL_YEAR).contains(&year) {
        return days_in_year(year) == 366;
    }
    // Romme: divisible by four, with the Gregorian century exceptions and a further one at
    // four thousand, which is what keeps it in step with the equinox over the long run.
    year % 4 == 0 && !matches!(year % 400, 100 | 200 | 300) && year % 4000 != 0
}

/// The day on which 1 Vendémiaire of a Republican year falls.
///
/// Returns `None` for a year before the Republic.
#[must_use]
pub fn new_year(year: i32) -> Option<Day> {
    if year < 1 {
        return None;
    }

    if let Some(&(g_year, month, day)) = NEW_YEARS.get(usize::try_from(year - 1).ok()?) {
        return Some(gregorian::to_day(g_year, month, day));
    }

    // Past the table: count Romme years forward from where the table ends.
    let mut fixed = {
        let (g_year, month, day) = NEW_YEARS[NEW_YEARS.len() - 1];
        gregorian::to_day(g_year, month, day).fixed()
    };
    let mut reconstructed = i32::try_from(NEW_YEARS.len()).ok()?;
    while reconstructed < year {
        fixed += if is_sextile_year(reconstructed) {
            366
        } else {
            365
        };
        reconstructed += 1;
    }
    Some(Day::new(fixed))
}

/// The length of a Republican year in days: 365 or 366.
#[must_use]
pub fn days_in_year(year: i32) -> i32 {
    match (new_year(year), new_year(year + 1)) {
        (Some(start), Some(end)) => start.days_until(end),
        _ => 365,
    }
}

/// The number of months in a Republican year.
///
/// Always thirteen, counting the complementary days as the last one. The thirteenth is five or
/// six days long, which is the only place in this calendar where anything varies.
#[must_use]
pub const fn months_in_year(_year: i32) -> u8 {
    13
}

/// Days in a Republican month.
///
/// Thirty, except the complementary days at the end of the year.
#[must_use]
pub fn days_in_month(year: i32, month: u8) -> Option<u8> {
    match month {
        1..=12 => Some(30),
        13 => Some(if is_sextile_year(year) { 6 } else { 5 }),
        _ => None,
    }
}

/// Converts a Republican date to a day number.
///
/// Returns `None` for a year before the Republic.
#[must_use]
pub fn to_day(year: i32, month: u8, day: u8) -> Option<Day> {
    let start = new_year(year)?;
    // Every month before the last is exactly thirty days, which is the calendar's whole idea.
    let offset = 30 * (i32::from(month) - 1) + i32::from(day) - 1;
    Some(start.offset(offset))
}

/// Converts a day number to a Republican date, as `(year, month, day)`.
///
/// Returns `None` for a day before the Republic began.
#[must_use]
pub fn from_day(day: Day) -> Option<(i32, u8, u8)> {
    let first = new_year(1)?;
    if day < first {
        return None;
    }

    // A Republican year is 365 or 366 days, so dividing gets within one and the loop settles
    // it. The table years are irregular enough that a closed form would be a lie.
    let mut year = 1 + first.days_until(day) / 366;
    while new_year(year + 1).is_some_and(|next| next <= day) {
        year += 1;
    }
    while year > 1 && new_year(year).is_some_and(|start| start > day) {
        year -= 1;
    }

    let elapsed = new_year(year)?.days_until(day);
    let month = (elapsed / 30 + 1).min(13);
    let day_of_month = elapsed - 30 * (month - 1) + 1;

    Some((year, month as u8, day_of_month as u8))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{days_in_month, days_in_year, from_day, is_sextile_year, new_year, to_day};
    use crate::gregorian;

    /// The Gregorian date naming the same day as a Republican one.
    fn as_gregorian(y: i32, m: u8, d: u8) -> (i32, u8, u8) {
        gregorian::from_day(to_day(y, m, d).unwrap())
    }

    #[test]
    fn the_republic_began_on_the_equinox() {
        assert_eq!(as_gregorian(1, 1, 1), (1792, 9, 22));
    }

    #[test]
    fn the_dates_that_named_events() {
        // 9 Thermidor Year II — the fall of Robespierre — was 27 July 1794.
        assert_eq!(as_gregorian(2, 11, 9), (1794, 7, 27));
        // 18 Brumaire Year VIII — Bonaparte's coup — was 9 November 1799.
        assert_eq!(as_gregorian(8, 2, 18), (1799, 11, 9));
        // 18 Fructidor Year V was 4 September 1797.
        assert_eq!(as_gregorian(5, 12, 18), (1797, 9, 4));
    }

    #[test]
    fn the_calendar_ended_in_the_middle_of_year_fourteen() {
        // The abolition took effect on 1 January 1806, which was 11 Nivôse Year XIV.
        assert_eq!(as_gregorian(14, 4, 10), (1805, 12, 31));
        assert_eq!(as_gregorian(14, 4, 11), (1806, 1, 1));
    }

    #[test]
    fn the_observed_sextile_years_were_three_seven_and_eleven() {
        // Not four, eight and twelve — which is what an arithmetic rule would have given, and
        // is the whole reason the years in force are a table.
        let sextiles: Vec<i32> = (1..=14).filter(|&y| is_sextile_year(y)).collect();
        assert_eq!(sextiles, vec![3, 7, 11]);
        assert_eq!(days_in_month(3, 13), Some(6));
        assert_eq!(days_in_month(4, 13), Some(5));
    }

    #[test]
    fn every_year_in_force_is_365_or_366_days() {
        for year in 1..=13 {
            let length = days_in_year(year);
            assert!(matches!(length, 365 | 366), "year {year} was {length} days");
            assert_eq!(length == 366, is_sextile_year(year), "year {year}");
        }
    }

    #[test]
    fn the_months_are_all_thirty_days_except_the_last() {
        for month in 1..=12 {
            assert_eq!(days_in_month(7, month), Some(30));
        }
        assert_eq!(days_in_month(7, 13), Some(6), "year VII was a sextile year");
        assert_eq!(days_in_month(7, 14), None);
    }

    #[test]
    fn the_complementary_days_are_the_last_five_or_six_of_the_year() {
        // 5 Sansculottides Year II is the last day of that year, and the next day is
        // 1 Vendémiaire Year III.
        let last = to_day(2, 13, 5).unwrap();
        assert_eq!(last.offset(1), new_year(3).unwrap());
        // Year III had six of them.
        let last_of_three = to_day(3, 13, 6).unwrap();
        assert_eq!(last_of_three.offset(1), new_year(4).unwrap());
    }

    #[test]
    fn the_conversion_is_a_bijection_over_every_year_in_force() {
        let start = new_year(1).unwrap().fixed();
        let end = new_year(15).unwrap().fixed();
        for fixed in start..end {
            let day = crate::Day::new(fixed);
            let (y, m, d) = from_day(day).unwrap();
            assert!((1..=13).contains(&m), "day {fixed} gave month {m}");
            assert!(
                d >= 1 && d <= days_in_month(y, m).unwrap(),
                "day {fixed} gave {y}-{m}-{d}"
            );
            assert_eq!(to_day(y, m, d).unwrap(), day, "year {y} month {m} day {d}");
        }
    }

    #[test]
    fn a_day_before_the_republic_is_not_a_republican_date() {
        let first = new_year(1).unwrap();
        assert_eq!(from_day(first.offset(-1)), None);
        assert_eq!(from_day(first), Some((1, 1, 1)));
        assert_eq!(new_year(0), None);
        assert_eq!(to_day(0, 1, 1), None);
    }

    #[test]
    fn the_reconstruction_past_year_fourteen_is_continuous() {
        // Never in force, so there is nothing to check it against except itself: the years must
        // still tile without gap or overlap.
        for year in 15..200 {
            let start = new_year(year).unwrap();
            let next = new_year(year + 1).unwrap();
            let length = start.days_until(next);
            assert!(matches!(length, 365 | 366), "year {year} was {length} days");
            let (y, m, d) = from_day(start).unwrap();
            assert_eq!((y, m, d), (year, 1, 1));
        }
    }
}
