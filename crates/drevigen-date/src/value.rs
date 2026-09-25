//! What a source says about when something happened.
//!
//! [`CalendarDate`] is a date. [`DateValue`] is everything else a record's date can be: "about
//! 1871", "before the 1897 census", "between 1869 and 1873", "from 1902 to 1914".
//! [`RecordedDate`] is what gets stored: the value that could be read, and the words the source
//! used, either of which may be missing.
//!
//! The forms are GEDCOM 7's `DateValue`, which is not arbitrary — it is what a century of
//! genealogical practice settled on, and matching it means an import loses nothing.
//!
//! # The phrase is beside the date, not inside it
//!
//! GEDCOM 5.5.1 let a date payload contain free text. 7.0 moved the text into a `PHRASE`
//! substructure next to the date, and this crate follows 7.0, because the older design had no
//! way to say "the record reads *30 January 1648/49*, which means 30 January 1649". That needs
//! both halves at once: the computable value and the words, kept apart so that neither
//! overwrites the other.
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
/// leave around the date. The definitions quoted are GEDCOM 7.0's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Approximation {
    /// `ABT` — "exact date unknown, but near *x*". The source hedged, or it is imprecise by
    /// nature: a census age, a recollection.
    About,
    /// `CAL` — "*x* is calculated from other data". A stated age at death, a marriage age, a
    /// gravestone. The date is only as good as its input, and its own arithmetic is exact.
    Calculated,
    /// `EST` — "exact date unknown, but near *x*; and *x* is calculated from other data". Both at
    /// once: arithmetic whose input was itself approximate, such as a parent's birth put a
    /// generation before the first child's.
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
    /// - **Estimated: five years.** The specification defines it as both near and calculated,
    ///   so it carries the imprecision of an approximation compounded by an inference: a
    ///   generation is twenty to forty years, and a parent placed "one generation before" the
    ///   first child is not worth more than this.
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

/// A date as a record gives it, in one of GEDCOM 7's forms.
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
    ///
    /// Kept in the order written. A source that says "between 1873 and 1869" is still a source
    /// that said it that way, and the span is computed correctly either way round.
    Between {
        /// The first bound as written.
        earliest: CalendarDate,
        /// The second bound as written.
        latest: CalendarDate,
    },

    /// `BEF x` — "exact date unknown, but no later than *x*". Under 7.0 that includes *x* itself:
    /// `BEF 1850` reaches to 31 December 1850.
    Before(CalendarDate),

    /// `AFT x` — "exact date unknown, but no earlier than *x*". `AFT 1850` begins on
    /// 1 January 1850, which is a change from 5.5.1; see [`crate::gedcom`].
    After(CalendarDate),

    /// `FROM x TO y`, `FROM x`, `TO y` — it went on for this stretch. Either end may be missing,
    /// which is how a record says "from 1902" about a residence nobody recorded the end of.
    ///
    /// At least one end is present in anything the parser produces; the grammar has no form for
    /// a period with neither, because that is the empty date. A value built by hand with both
    /// missing is treated as saying nothing, which is what it says.
    Period {
        /// When it began.
        from: Option<CalendarDate>,
        /// When it ended.
        to: Option<CalendarDate>,
    },
}

impl DateValue {
    /// The calendar this value is written in.
    ///
    /// A period with two ends in different calendars reports the first, which is the one a
    /// reader meets first. `None` only for a hand-built period with no ends.
    #[must_use]
    pub fn calendar(&self) -> Option<Calendar> {
        match self {
            Self::Exact(date) | Self::Approximate { date, .. } => Some(date.calendar()),
            Self::Between { earliest: date, .. } | Self::Before(date) | Self::After(date) => {
                Some(date.calendar())
            }
            Self::Period { from, to } => from.or(*to).map(CalendarDate::calendar),
        }
    }

    /// Every date the value mentions, in the order written.
    #[must_use]
    pub fn dates(&self) -> Vec<CalendarDate> {
        match self {
            Self::Exact(date)
            | Self::Approximate { date, .. }
            | Self::Before(date)
            | Self::After(date) => vec![*date],
            Self::Between { earliest, latest } => vec![*earliest, *latest],
            Self::Period { from, to } => from.iter().chain(to.iter()).copied().collect(),
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
                // Either may be the later one; `closed` orders them, and the outer bounds of
                // both are what "somewhere between" covers.
                let low = earliest.earliest().min(latest.earliest());
                let high = earliest.latest().max(latest.latest());
                Span::closed(low, high)
            }
            Self::Before(date) => Span::until(date.latest()),
            Self::After(date) => Span::from(date.earliest()),
            Self::Period { from, to } => match (from, to) {
                (Some(start), Some(end)) => Span::closed(start.earliest(), end.latest()),
                (Some(start), None) => Span::from(start.earliest()),
                (None, Some(end)) => Span::until(end.latest()),
                (None, None) => Span::unbounded(),
            },
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
    #[must_use]
    pub fn contains(&self, day: Day) -> Trivalent {
        if self.says_nothing() {
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
    /// A partial date cannot — "April 1871" has no counterpart in another calendar, because its
    /// month boundaries fall inside two of the other's — and is returned unchanged rather than
    /// mangled. So is a date before the target calendar begins.
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
        }
    }

    /// Whether this value constrains nothing at all.
    fn says_nothing(&self) -> bool {
        matches!(
            self,
            Self::Period {
                from: None,
                to: None
            }
        )
    }
}

/// A date as it is stored: the value that could be read, and the words the source used.
///
/// Either half may be missing, and the combinations mean different things:
///
/// | `value` | `phrase` | What the record held |
/// |---|---|---|
/// | some | none | A date, and nothing else to say about it |
/// | some | some | A date *and* the words it was read from — "30 January 1648/49", stored as 1649 |
/// | none | some | Words that are not a date: "in the third year after the fire" |
/// | none | none | Nothing; an event whose date is unknown |
///
/// The third row is the one tools usually lose. A record that gives a sentence instead of a date
/// still said something, and discarding it because it cannot be computed with discards the only
/// evidence there was.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecordedDate {
    /// What could be read as a date.
    pub value: Option<DateValue>,
    /// The words the source used, when they say more than the value does.
    pub phrase: Option<String>,
}

impl RecordedDate {
    /// A date with nothing further to say about it.
    #[must_use]
    pub const fn from_value(value: DateValue) -> Self {
        Self {
            value: Some(value),
            phrase: None,
        }
    }

    /// Words that are not a date, kept as written.
    #[must_use]
    pub fn from_phrase(phrase: impl Into<String>) -> Self {
        Self {
            value: None,
            phrase: Some(phrase.into()),
        }
    }

    /// Whether anything at all is recorded.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.value.is_none() && self.phrase.is_none()
    }

    /// The span the value states, or the span that says nothing when there is no value.
    ///
    /// A phrase constrains nothing. Saying so is the honest answer; guessing a year from prose is
    /// how a tool invents evidence.
    #[must_use]
    pub fn stated_span(&self) -> Span {
        self.value
            .as_ref()
            .map_or_else(Span::unbounded, DateValue::stated_span)
    }

    /// The span to search in; see [`DateValue::search_span`].
    #[must_use]
    pub fn search_span(&self) -> Span {
        self.value
            .as_ref()
            .map_or_else(Span::unbounded, DateValue::search_span)
    }

    /// Whether a day falls inside the recorded date. `Unknown` when only a phrase is recorded.
    #[must_use]
    pub fn contains(&self, day: Day) -> Trivalent {
        self.value
            .as_ref()
            .map_or(Trivalent::Unknown, |value| value.contains(day))
    }

    /// Whether this happened before another recorded date.
    #[must_use]
    pub fn before(&self, other: &Self) -> Trivalent {
        match (&self.value, &other.value) {
            (Some(this), Some(that)) => this.before(that),
            _ => Trivalent::Unknown,
        }
    }

    /// Whether this happened after another recorded date.
    #[must_use]
    pub fn after(&self, other: &Self) -> Trivalent {
        other.before(self)
    }

    /// Whether the two could describe the same moment; see [`DateValue::could_coincide`].
    #[must_use]
    pub fn could_coincide(&self, other: &Self) -> Trivalent {
        match (&self.value, &other.value) {
            (Some(this), Some(that)) => this.could_coincide(that),
            _ => Trivalent::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{Approximation, DateValue, RecordedDate};
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
    fn before_and_after_include_the_stated_year_as_seven_point_zero_defines_them() {
        // "BEF x: no later than x" and "AFT x: no earlier than x". The 5.5.1 reading excluded x,
        // and the change is the reason importing an older file only ever widens a date.
        let before = DateValue::Before(year(1850)).stated_span();
        assert_eq!(before.latest, Some(crate::gregorian::to_day(1850, 12, 31)));
        let after = DateValue::After(year(1850)).stated_span();
        assert_eq!(after.earliest, Some(crate::gregorian::to_day(1850, 1, 1)));
    }

    #[test]
    fn a_range_written_backwards_still_covers_the_years_between() {
        let backwards = DateValue::Between {
            earliest: year(1873),
            latest: year(1869),
        };
        let forwards = DateValue::Between {
            earliest: year(1869),
            latest: year(1873),
        };
        assert_eq!(backwards.stated_span(), forwards.stated_span());
        assert_ne!(
            backwards, forwards,
            "but it is stored as the source wrote it"
        );
    }

    #[test]
    fn a_phrase_constrains_nothing_and_says_so() {
        let phrase = RecordedDate::from_phrase("в третий год после пожара");
        assert_eq!(phrase.stated_span(), crate::Span::unbounded());
        assert_eq!(
            phrase.contains(crate::gregorian::to_day(1871, 1, 1)),
            Trivalent::Unknown
        );
        assert!(
            !phrase.is_empty(),
            "words are a record, even when they are not a date"
        );

        // And it does not silently order itself against anything.
        let known = RecordedDate::from_value(DateValue::Exact(year(1871)));
        assert_eq!(phrase.before(&known), Trivalent::Unknown);
        assert_eq!(known.before(&phrase), Trivalent::Unknown);
    }

    #[test]
    fn a_phrase_beside_a_value_does_not_change_what_it_computes() {
        // "30 January 1648/49" is stored as 1649 with the words kept. The words are for the
        // reader; the arithmetic uses the value alone.
        let dual = RecordedDate {
            value: Some(DateValue::Exact(date(1649, Some(1), Some(30)))),
            phrase: Some("30 January 1648/49".to_owned()),
        };
        let plain = RecordedDate::from_value(DateValue::Exact(date(1649, Some(1), Some(30))));
        assert_eq!(dual.stated_span(), plain.stated_span());
        assert_eq!(dual.could_coincide(&plain), Trivalent::Yes);
    }

    #[test]
    fn nothing_recorded_is_distinguishable_from_a_phrase() {
        assert!(RecordedDate::default().is_empty());
        assert_eq!(
            RecordedDate::default().stated_span(),
            crate::Span::unbounded()
        );
    }

    #[test]
    fn a_period_is_not_a_range() {
        // The distinction GEDCOM draws and the reason this enum has both: one event that lasted
        // twelve years, and one event that happened once somewhere inside four.
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

        // A partial date has no counterpart in another calendar.
        let partial =
            DateValue::Exact(CalendarDate::new(Calendar::Julian, 1871, Some(4), None).unwrap());
        assert_eq!(partial.to_calendar(Calendar::Gregorian), partial);
    }
}
