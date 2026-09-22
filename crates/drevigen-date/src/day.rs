//! The day number every calendar converts through.
//!
//! Calendars are converted by way of a single count of days, not by converting one calendar
//! directly into another. Four calendars have twelve directed pairs between them; with a pivot
//! there are eight functions, and each one can be checked against published tables on its own.
//!
//! The count used here is the **fixed day number** of Dershowitz and Reingold's
//! *Calendrical Calculations*: day 1 is 1 January 1 in the proleptic Gregorian calendar. Their
//! algorithms are stated in it, and restating them in another epoch is how an off-by-one enters
//! a calendar library.
//!
//! [`Day::julian_day_number`] converts to the count genealogy and astronomy quote, which is
//! larger by 1 721 425.

/// A day, counted from 1 January 1 (proleptic Gregorian) = 1.
///
/// Days before that are negative; there is no zero problem, because day 0 is 31 December 1 BCE
/// and means exactly that.
///
/// ```
/// use drevigen_date::Day;
///
/// // The two counts differ by a constant and nothing else.
/// assert_eq!(Day::new(1).julian_day_number(), 1_721_426);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(i32);

/// The Julian day number of fixed day 1.
///
/// Julian day numbers proper are counted from noon, so the astronomical JD of this instant is
/// 1 721 425.5. Genealogy quotes the integer, and the integer is what this returns.
const JDN_OF_DAY_ONE: i32 = 1_721_425;

impl Day {
    /// Wraps a fixed day number.
    #[must_use]
    pub const fn new(fixed: i32) -> Self {
        Self(fixed)
    }

    /// The fixed day number.
    #[must_use]
    pub const fn fixed(self) -> i32 {
        self.0
    }

    /// The Julian day number, as genealogy and astronomy quote it.
    #[must_use]
    pub const fn julian_day_number(self) -> i32 {
        self.0 + JDN_OF_DAY_ONE
    }

    /// Reads a Julian day number.
    #[must_use]
    pub const fn from_julian_day_number(jdn: i32) -> Self {
        Self(jdn - JDN_OF_DAY_ONE)
    }

    /// Days from `self` to `other`, positive when `other` is later.
    #[must_use]
    pub const fn days_until(self, other: Self) -> i32 {
        other.0 - self.0
    }

    /// The day `count` days after this one.
    #[must_use]
    pub const fn offset(self, count: i32) -> Self {
        Self(self.0 + count)
    }

    /// The day of the week.
    #[must_use]
    pub const fn weekday(self) -> Weekday {
        // Fixed day 1 was a Monday, so the residue counts from there.
        match self.0.rem_euclid(7) {
            0 => Weekday::Sunday,
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            _ => Weekday::Saturday,
        }
    }
}

/// A day of the week.
///
/// Present because genealogical sources state one — a parish register that says "Sunday" is
/// evidence, and a date that falls on a Tuesday contradicts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weekday {
    /// Sunday.
    Sunday,
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
}

/// Integer division that rounds towards negative infinity.
///
/// Every calendar formula here assumes it. Rust's `/` truncates towards zero, which is the same
/// thing for positive years and silently wrong for dates before the common era — the case a
/// genealogy library must not get wrong, because it is exactly where nobody checks.
pub(crate) const fn div_floor(a: i32, b: i32) -> i32 {
    a.div_euclid(b)
}

/// The non-negative remainder matching [`div_floor`].
pub(crate) const fn mod_floor(a: i32, b: i32) -> i32 {
    a.rem_euclid(b)
}

#[cfg(test)]
mod tests {
    use super::{Day, Weekday, div_floor, mod_floor};

    #[test]
    fn the_two_day_counts_differ_by_a_constant() {
        // 1 January 2000 is fixed day 730 120 and JDN 2 451 545 — the J2000.0 epoch, which is
        // quoted often enough to be a reliable anchor.
        let j2000 = Day::new(730_120);
        assert_eq!(j2000.julian_day_number(), 2_451_545);
        assert_eq!(Day::from_julian_day_number(2_451_545), j2000);
    }

    #[test]
    fn day_one_was_a_monday() {
        assert_eq!(Day::new(1).weekday(), Weekday::Monday);
        assert_eq!(Day::new(0).weekday(), Weekday::Sunday);
        assert_eq!(Day::new(-1).weekday(), Weekday::Saturday);
        // 1 January 2000 was a Saturday.
        assert_eq!(Day::new(730_120).weekday(), Weekday::Saturday);
    }

    #[test]
    fn arithmetic_is_ordinary() {
        let a = Day::new(100);
        let b = Day::new(130);
        assert_eq!(a.days_until(b), 30);
        assert_eq!(b.days_until(a), -30);
        assert_eq!(a.offset(30), b);
    }

    #[test]
    fn division_rounds_downwards_on_both_sides_of_zero() {
        // The whole reason these helpers exist. Rust's built-in `/` gives -2 and -1 here.
        assert_eq!(div_floor(-7, 4), -2);
        assert_eq!(div_floor(-8, 4), -2);
        assert_eq!(div_floor(7, 4), 1);
        assert_eq!(mod_floor(-7, 4), 1);
        assert_eq!(mod_floor(7, 4), 3);
    }
}
