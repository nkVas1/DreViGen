//! What a source says about when something happened.
//!
//! [`CalendarDate`] is a date. This is everything else a record actually contains: "about 1871",
//! "before the 1897 census", "between 1869 and 1873", "from 1902 to 1914", and the entries that
//! give a sentence instead of a date and still have to be stored as written.
//!
//! The forms are GEDCOM 7's `DATE_VALUE`, which is not arbitrary — it is what a century of
//! genealogical practice settled on, and matching it means an import loses nothing.
//!
//! # A range and a period are not the same thing
//!
//! GEDCOM distinguishes them and so does this, because the difference is real. *Between 1869 and
//! 1873* says an event happened once, at an unknown moment inside those years. *From 1902 to
//! 1914* says it went on for all of them. A birth is a range; a residence is a period. Treating
//! them alike produces a person who was born continuously for four years.

use crate::day::Day;
use crate::interval::{Span, Trivalent};
use crate::{Calendar, CalendarDate};

/// How approximate an approximate date is, and why.
///
/// The three GEDCOM qualifiers are not synonyms, and the difference decides how much room to
/// leave around the date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Approximation {
    /// `ABT` — the source itself hedged, or the researcher is reading an imprecise record.
    About,
    /// `CAL` — arithmetic from something else: a stated age at death, a marriage age, a
    /// gravestone. The date is only as good as its input, and its own arithmetic is exact.
    Calculated,
    /// `EST` — the researcher's inference from context, with no arithmetic under it.
    Estimated,
}

impl Approximation {
    /// The GEDCOM 7 tag.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::About => "ABT",
            Self::Calculated => "CAL",
            Self::Estimated => "EST",
        }
    }

    /// How many years of room to leave on each side when the date is used as an interval.
    ///
    /// GEDCOM defines the qualifiers and says nothing about how wide they are, so this is a
    /// judgement, stated here rather than buried at a call site:
    ///
    /// - **About: two years.** The commonest cause is a census age, and census ages are wrong
    ///   by one or two years constantly.
    /// - **Calculated: none.** The arithmetic is exact. Any error came in with the input, and
    ///   inventing slop here would double-count it.
    /// - **Estimated: five years.** An inference from context — "he must have been born before
    ///   his first child" — is worth no more than that.
    ///
    /// These widen a search; they never widen a stored date. [`DateValue::stated_span`] returns
    /// what the source wrote, with no tolerance at all.
    #[must_use]
    pub const fn tolerance_years(self) -> i32 {
        match self {
            Self::About => 2,
            Self::Calculated => 0,
            Self::Estimated => 5,
        }
    }
}

/// The longest a year can be in any calendar this crate knows, used to turn a tolerance in
/// years into one in days without pretending to know which years they are.
const LONGEST_YEAR: i32 = 385;

/// A date as a record gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateValue {
    /// One date, to whatever precision the source gave: `17 APR 1871`, `APR 1871`, `1871`.
    Exact(CalendarDate),

    /// A date the source or the researcher hedged: `ABT 1871`, `CAL 1871`, `EST 1871`.
    Approximate {
        /// The date as written.
        date: CalendarDate,
        /// Which kind of hedge, which decides how much room to leave.
        kind: Approximation,
    },

    /// `BET x AND y` — it happened once, somewhere in here.
    Between {
        /// The earlier bound.
        earliest: CalendarDate,
        /// The later bound.
        latest: CalendarDate,
    },

    /// `BEF x` — it happened at some point before this, with no stated beginning.
    Before(CalendarDate),

    /// `AFT x` — at some point after this, with no stated end.
    After(CalendarDate),

    /// `FROM x TO y` — it went on for this stretch. Either end may be missing, which is how a
    /// record says "from 1902" about a residence nobody recorded the end of.
    Period {
        /// When it began.
        from: Option<CalendarDate>,
        /// When it ended.
        to: Option<CalendarDate>,
    },

    /// Something the grammar cannot read, stored exactly as written.
    ///
    /// "In the third year after the fire", "during the war", "при Александре III". A tool that
    /// cannot hold these loses the only thing the record said, which is worse than holding
    /// something it cannot compute with.
    Phrase(String),
}

impl DateValue {
    /// The calendar this value is written in, if it has one.
    ///
    /// A phrase has none. A period with two ends in different calendars reports the first,
    /// which is the one a reader will see.
    #[must_use]
    pub fn calendar(&self) -> Option<Calendar> {
        match self {
            Self::Exact(date) | Self::Approximate { date, .. } => Some(date.calendar()),
            Self::Between { earliest: date, .. } | Self::Before(date) | Self::After(date) => {
                Some(date.calendar())
            }
            Self::Period { from, to } => from.or(*to).map(CalendarDate::calendar),
            Self::Phrase(_) => None,
        }
    }

    /// Whether this fixes the date to one day.
    #[must_use]
    pub fn is_certain(&self) -> bool {
        matches!(self, Self::Exact(date) if date.is_exact())
    }

    /// Whether this is a stretch of time the event occupied, rather than a moment inside one.
    #[must_use]
    pub const fn is_period(&self) -> bool {
        matches!(self, Self::Period { .. })
    }

    /// The span the source stated, with no tolerance added.
    ///
    /// This is what was written. "About 1871" spans 1871 and nothing more, because that is the
    /// year on the page; the two years of room an approximation earns belong to
    /// [`search_span`](DateValue::search_span), where a caller has asked to be generous.
    #[must_use]
    pub fn stated_span(&self) -> Span {
        match self {
            Self::Exact(date) | Self::Approximate { date, .. } => {
                Span::closed(date.earliest(), date.latest())
            }
            Self::Between { earliest, latest } => {
                Span::closed(earliest.earliest(), latest.latest())
            }
            Self::Before(date) => Span::until(date.latest()),
            Self::After(date) => Span::from(date.earliest()),
            Self::Period { from, to } => match (from, to) {
                (Some(start), Some(end)) => Span::closed(start.earliest(), end.latest()),
                (Some(start), None) => Span::from(start.earliest()),
                (None, Some(end)) => Span::until(end.latest()),
                (None, None) => Span::unbounded(),
            },
            // A phrase constrains nothing. Saying so is the honest answer; guessing a year from
            // prose is how a tool invents evidence.
            Self::Phrase(_) => Span::unbounded(),
        }
    }

    /// The span to search in, with an approximation's tolerance included.
    ///
    /// Use this to answer "could this be the same event?" or "who was alive in 1900?", where
    /// missing a match costs more than considering one too many. Use
    /// [`stated_span`](DateValue::stated_span) to display what the record says.
    #[must_use]
    pub fn search_span(&self) -> Span {
        match self {
            Self::Approximate { kind, .. } => self
                .stated_span()
                .widened(kind.tolerance_years() * LONGEST_YEAR),
            _ => self.stated_span(),
        }
    }

    /// Whether a day falls inside the stated span.
    ///
    /// `Unknown` for a phrase, which constrains nothing and therefore excludes nothing.
    #[must_use]
    pub fn contains(&self, day: Day) -> Trivalent {
        if matches!(self, Self::Phrase(_)) {
            return Trivalent::Unknown;
        }
        Trivalent::known(self.stated_span().contains(day))
    }

    /// Whether this happened before another date.
    #[must_use]
    pub fn before(&self, other: &Self) -> Trivalent {
        if self.says_nothing() || other.says_nothing() {
            return Trivalent::Unknown;
        }
        self.stated_span().before(other.stated_span())
    }

    /// Whether this happened after another date.
    #[must_use]
    pub fn after(&self, other: &Self) -> Trivalent {
        other.before(self)
    }

    /// Whether the two could refer to the same moment, allowing for approximation.
    ///
    /// Uses [`search_span`](DateValue::search_span) on both sides, because deciding whether two
    /// records describe one event is exactly the case where being generous is right.
    #[must_use]
    pub fn could_coincide(&self, other: &Self) -> Trivalent {
        if self.says_nothing() || other.says_nothing() {
            return Trivalent::Unknown;
        }
        self.search_span().same_day_as(other.search_span())
    }

    /// Restates the date in another calendar, where every part of it can be restated.
    ///
    /// A phrase cannot. Neither can a partial date — "April 1871" has no counterpart in another
    /// calendar, because its month boundaries fall inside two of the other's. In both cases the
    /// value is returned unchanged rather than mangled.
    #[must_use]
    pub fn to_calendar(&self, target: Calendar) -> Self {
        let convert = |date: &CalendarDate| date.to_calendar(target).unwrap_or(*date);
        match self {
            Self::Exact(date) => Self::Exact(convert(date)),
            Self::Approximate { date, kind } => Self::Approximate {
                date: convert(date),
                kind: *kind,
            },
            Self::Between { earliest, latest } => Self::Between {
                earliest: convert(earliest),
                latest: convert(latest),
            },
            Self::Before(date) => Self::Before(convert(date)),
            Self::After(date) => Self::After(convert(date)),
            Self::Period { from, to } => Self::Period {
                from: from.as_ref().map(convert),
                to: to.as_ref().map(convert),
            },
            Self::Phrase(text) => Self::Phrase(text.clone()),
        }
    }

    /// Whether this value constrains nothing at all.
    fn says_nothing(&self) -> bool {
        matches!(
            self,
            Self::Phrase(_)
                | Self::Period {
                    from: None,
                    to: None
                }
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{Approximation, DateValue};
    use crate::interval::Trivalent;
    use crate::{Calendar, CalendarDate};

    fn date(year: i32, month: Option<u8>, day: Option<u8>) -> CalendarDate {
        CalendarDate::new(Calendar::Gregorian, year, month, day).unwrap()
    }

    fn year(y: i32) -> CalendarDate {
        date(y, None, None)
    }

    #[test]
    fn a_year_only_date_spans_the_whole_year() {
        let value = DateValue::Exact(year(1871));
        let span = value.stated_span();
        assert_eq!(span.earliest, Some(crate::gregorian::to_day(1871, 1, 1)));
        assert_eq!(span.latest, Some(crate::gregorian::to_day(1871, 12, 31)));
        assert!(!value.is_certain());

        let exact = DateValue::Exact(date(1871, Some(4), Some(17)));
        assert!(exact.is_certain());
        assert!(exact.stated_span().is_certain());
    }

    #[test]
    fn an_approximation_does_not_widen_what_the_record_says() {
        // The distinction this module exists to keep: the page says 1871, and only a search
        // gets to be generous about it.
        let about = DateValue::Approximate {
            date: year(1871),
            kind: Approximation::About,
        };
        assert_eq!(
            about.stated_span(),
            DateValue::Exact(year(1871)).stated_span()
        );

        let searched = about.search_span();
        assert!(searched.contains(crate::gregorian::to_day(1869, 6, 1)));
        assert!(searched.contains(crate::gregorian::to_day(1873, 6, 1)));
        assert!(!searched.contains(crate::gregorian::to_day(1866, 1, 1)));
    }

    #[test]
    fn a_calculated_date_earns_no_tolerance() {
        // Its arithmetic is exact; the error arrived with the input, and adding room here would
        // count that error twice.
        let calculated = DateValue::Approximate {
            date: year(1871),
            kind: Approximation::Calculated,
        };
        assert_eq!(calculated.search_span(), calculated.stated_span());

        let estimated = DateValue::Approximate {
            date: year(1871),
            kind: Approximation::Estimated,
        };
        assert!(
            estimated.search_span().length().unwrap() > calculated.search_span().length().unwrap()
        );
    }

    #[test]
    fn an_open_bound_stays_open() {
        let before = DateValue::Before(year(1875));
        assert_eq!(before.stated_span().earliest, None);
        assert!(
            before
                .stated_span()
                .contains(crate::gregorian::to_day(1500, 1, 1))
        );

        let after = DateValue::After(year(1869));
        assert_eq!(after.stated_span().latest, None);
    }

    #[test]
    fn a_phrase_constrains_nothing_and_says_so() {
        let phrase = DateValue::Phrase("в третий год после пожара".to_owned());
        assert_eq!(phrase.stated_span(), crate::Span::unbounded());
        assert_eq!(
            phrase.contains(crate::gregorian::to_day(1871, 1, 1)),
            Trivalent::Unknown
        );
        assert_eq!(phrase.calendar(), None);

        // And it does not silently order itself against anything.
        let known = DateValue::Exact(year(1871));
        assert_eq!(phrase.before(&known), Trivalent::Unknown);
        assert_eq!(known.before(&phrase), Trivalent::Unknown);
    }

    #[test]
    fn a_period_is_not_a_range() {
        // The distinction GEDCOM draws and the reason this enum has both: one event that lasted
        // fourteen years, and one event that happened once somewhere inside four.
        let residence = DateValue::Period {
            from: Some(year(1902)),
            to: Some(year(1914)),
        };
        let birth = DateValue::Between {
            earliest: year(1869),
            latest: year(1873),
        };
        assert!(residence.is_period());
        assert!(!birth.is_period());

        // They still measure the same way.
        assert_eq!(
            residence.stated_span().earliest,
            Some(crate::gregorian::to_day(1902, 1, 1))
        );
        assert_eq!(
            residence.stated_span().latest,
            Some(crate::gregorian::to_day(1914, 12, 31))
        );
    }

    #[test]
    fn a_period_with_one_end_is_half_open() {
        let ongoing = DateValue::Period {
            from: Some(year(1902)),
            to: None,
        };
        assert_eq!(ongoing.stated_span().latest, None);
        assert!(!ongoing.before(&DateValue::Exact(year(2000))).is_known());
    }

    #[test]
    fn overlapping_records_do_not_get_ordered() {
        let birth = DateValue::Between {
            earliest: year(1869),
            latest: year(1873),
        };
        let marriage = DateValue::Between {
            earliest: year(1871),
            latest: year(1875),
        };
        assert_eq!(birth.before(&marriage), Trivalent::Unknown);

        let later = DateValue::Exact(year(1890));
        assert_eq!(birth.before(&later), Trivalent::Yes);
        assert_eq!(later.before(&birth), Trivalent::No);
    }

    #[test]
    fn two_records_of_one_birth_can_be_recognised_as_one() {
        // A birth register says 17 April 1871. A census says about 1873. Being generous is
        // exactly right here, and `could_coincide` is where the generosity is allowed.
        let register = DateValue::Exact(date(1871, Some(4), Some(17)));
        let census = DateValue::Approximate {
            date: year(1873),
            kind: Approximation::About,
        };
        assert_eq!(register.could_coincide(&census), Trivalent::Unknown);

        // The stated spans, by contrast, do not meet at all.
        assert_eq!(
            register.stated_span().same_day_as(census.stated_span()),
            Trivalent::No
        );

        // Forty years apart is beyond any tolerance.
        let unrelated = DateValue::Exact(year(1930));
        assert_eq!(register.could_coincide(&unrelated), Trivalent::No);
    }

    #[test]
    fn conversion_leaves_alone_what_it_cannot_convert() {
        let julian = DateValue::Exact(
            CalendarDate::new(Calendar::Julian, 1917, Some(10), Some(25)).unwrap(),
        );
        let gregorian = julian.to_calendar(Calendar::Gregorian);
        assert_eq!(
            gregorian,
            DateValue::Exact(date(1917, Some(11), Some(7))),
            "the October Revolution, restated"
        );

        // A partial date has no counterpart, and a phrase has nothing to convert.
        let partial =
            DateValue::Exact(CalendarDate::new(Calendar::Julian, 1871, Some(4), None).unwrap());
        assert_eq!(partial.to_calendar(Calendar::Gregorian), partial);

        let phrase = DateValue::Phrase("during the war".to_owned());
        assert_eq!(phrase.to_calendar(Calendar::Gregorian), phrase);
    }
}
