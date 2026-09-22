//! Intervals, and the three answers a question about them can have.
//!
//! Genealogical dates are rarely points, so comparing them is not comparing numbers. "Was this
//! person alive in 1900?" has three answers — yes, no, and *we cannot tell* — and a library that
//! can only return two will return the wrong one. `docs/01-research/data-standards.md` §3 states
//! the requirement; this module is it.
//!
//! The third answer is the whole point. A tool that quietly resolves "unknown" to "no" tells a
//! researcher that a marriage did not happen, when the truth is that the record does not say.

use crate::day::Day;

/// The answer to a question about dates that may not be answerable.
///
/// Kleene's strong three-valued logic: [`Unknown`](Trivalent::Unknown) means the evidence does
/// not decide, and it propagates through [`and`](Trivalent::and) and [`or`](Trivalent::or) only
/// where it actually matters. `false AND unknown` is `false`, because one false conjunct settles
/// it whatever the other turns out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trivalent {
    /// The evidence says so.
    Yes,
    /// The evidence rules it out.
    No,
    /// The evidence does not decide.
    Unknown,
}

impl Trivalent {
    /// Reads a certain answer.
    #[must_use]
    pub const fn known(value: bool) -> Self {
        if value { Self::Yes } else { Self::No }
    }

    /// Whether this is a definite answer either way.
    #[must_use]
    pub const fn is_known(self) -> bool {
        !matches!(self, Self::Unknown)
    }

    /// Negation. Not knowing something stays not knowing its opposite.
    #[must_use]
    pub const fn not(self) -> Self {
        match self {
            Self::Yes => Self::No,
            Self::No => Self::Yes,
            Self::Unknown => Self::Unknown,
        }
    }

    /// Conjunction. One `No` settles it.
    #[must_use]
    pub const fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::No, _) | (_, Self::No) => Self::No,
            (Self::Yes, Self::Yes) => Self::Yes,
            _ => Self::Unknown,
        }
    }

    /// Disjunction. One `Yes` settles it.
    #[must_use]
    pub const fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Yes, _) | (_, Self::Yes) => Self::Yes,
            (Self::No, Self::No) => Self::No,
            _ => Self::Unknown,
        }
    }

    /// Collapses to a definite answer, for a caller that must have one.
    ///
    /// Takes what `Unknown` should become, so the choice is visible at the call site instead of
    /// being a default nobody reads. A filter showing "people alive in 1900" wants `true` here;
    /// a rule refusing to record a death before a birth wants `false`.
    #[must_use]
    pub const fn or_when_unknown(self, fallback: bool) -> bool {
        match self {
            Self::Yes => true,
            Self::No => false,
            Self::Unknown => fallback,
        }
    }
}

/// A stretch of time a date could refer to, open at either end.
///
/// Both bounds are inclusive. `earliest` is `None` for "before 1875", which has no beginning;
/// `latest` is `None` for "after 1869". A span with neither bound says nothing and is exactly
/// what an unparseable phrase produces — which is the honest answer, and better than pretending
/// a phrase is a year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The first day this could be, if there is one.
    pub earliest: Option<Day>,
    /// The last day this could be, if there is one.
    pub latest: Option<Day>,
}

impl Span {
    /// A span covering exactly one day.
    #[must_use]
    pub const fn at(day: Day) -> Self {
        Self {
            earliest: Some(day),
            latest: Some(day),
        }
    }

    /// A closed span. The bounds are swapped if they arrive the wrong way round, because a
    /// source that writes "between 1873 and 1869" still means the four years between them.
    #[must_use]
    pub fn closed(first: Day, second: Day) -> Self {
        let (earliest, latest) = if first <= second {
            (first, second)
        } else {
            (second, first)
        };
        Self {
            earliest: Some(earliest),
            latest: Some(latest),
        }
    }

    /// Everything up to and including a day.
    #[must_use]
    pub const fn until(day: Day) -> Self {
        Self {
            earliest: None,
            latest: Some(day),
        }
    }

    /// Everything from a day onwards.
    #[must_use]
    pub const fn from(day: Day) -> Self {
        Self {
            earliest: Some(day),
            latest: None,
        }
    }

    /// The span that says nothing.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            earliest: None,
            latest: None,
        }
    }

    /// Whether this span fixes a date to a single day.
    #[must_use]
    pub fn is_certain(self) -> bool {
        matches!((self.earliest, self.latest), (Some(a), Some(b)) if a == b)
    }

    /// How many days wide, if both ends are known.
    #[must_use]
    pub fn length(self) -> Option<i32> {
        match (self.earliest, self.latest) {
            (Some(a), Some(b)) => Some(a.days_until(b) + 1),
            _ => None,
        }
    }

    /// Widens the span by a number of days at each end.
    ///
    /// Open ends stay open: there is nothing to widen past the beginning of time.
    #[must_use]
    pub fn widened(self, days: i32) -> Self {
        Self {
            earliest: self.earliest.map(|d| d.offset(-days)),
            latest: self.latest.map(|d| d.offset(days)),
        }
    }

    /// The smallest span containing both.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            earliest: match (self.earliest, other.earliest) {
                (Some(a), Some(b)) => Some(a.min(b)),
                // One side reaches back forever, so the union does too.
                _ => None,
            },
            latest: match (self.latest, other.latest) {
                (Some(a), Some(b)) => Some(a.max(b)),
                _ => None,
            },
        }
    }

    /// Whether a day falls inside.
    ///
    /// Never [`Unknown`](Trivalent::Unknown): a span either contains a day or it does not. The
    /// uncertainty in a genealogical date lives in how wide the span is, not in whether a
    /// number is inside it.
    #[must_use]
    pub fn contains(self, day: Day) -> bool {
        self.earliest.is_none_or(|first| first <= day) && self.latest.is_none_or(|last| day <= last)
    }

    /// Whether the date in this span is earlier than the date in another.
    ///
    /// `Yes` when every day this could be is earlier than every day that could be. `No` when
    /// that is impossible — every day here is on or after every day there. `Unknown` in
    /// between: if a birth is "1869 to 1873" and a marriage is "1871 to 1875", the record does
    /// not say which came first, and it is not this function's business to guess.
    #[must_use]
    pub fn before(self, other: Self) -> Trivalent {
        if entirely_before(self, other) {
            return Trivalent::Yes;
        }
        // Impossible when the earliest day here is on or after the latest day there: `a < b`
        // would need `a < b <= a`. The `<=` matters — one shared day is enough to rule it out.
        match (self.earliest, other.latest) {
            (Some(this_start), Some(other_end)) if this_start >= other_end => Trivalent::No,
            _ => Trivalent::Unknown,
        }
    }

    /// Whether the date in this span is later than the date in another.
    #[must_use]
    pub fn after(self, other: Self) -> Trivalent {
        other.before(self)
    }

    /// Whether the two spans share at least one day.
    ///
    /// A fact about the intervals rather than about the dates, and therefore two-valued: the
    /// spans either meet or they do not. Whether the two *dates* coincide is a different
    /// question with three answers — see [`same_day_as`](Span::same_day_as).
    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        !entirely_before(self, other) && !entirely_before(other, self)
    }

    /// Whether the two dates are the same day.
    ///
    /// `Yes` only when both are fixed to one day and it is the same day. `No` when the spans do
    /// not meet at all. `Unknown` whenever they could be the same and could be different, which
    /// is the usual case for two partial dates and the reason this is not a `==`.
    #[must_use]
    pub fn same_day_as(self, other: Self) -> Trivalent {
        if !self.intersects(other) {
            return Trivalent::No;
        }
        if self.is_certain() && self == other {
            return Trivalent::Yes;
        }
        Trivalent::Unknown
    }

    /// Whether this span is wholly inside another.
    ///
    /// Two-valued, like [`intersects`](Span::intersects): a question about the intervals.
    #[must_use]
    pub fn within(self, other: Self) -> bool {
        let start_ok = match (other.earliest, self.earliest) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(outer), Some(inner)) => outer <= inner,
        };
        let end_ok = match (other.latest, self.latest) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(outer), Some(inner)) => inner <= outer,
        };
        start_ok && end_ok
    }
}

/// Whether every day of `first` is strictly earlier than every day of `second`.
///
/// Free rather than a method, because both `before` and `intersects` need it and a method that
/// called another method that called it back was how the first version got this wrong.
fn entirely_before(first: Span, second: Span) -> bool {
    matches!((first.latest, second.earliest), (Some(end), Some(start)) if end < start)
}

#[cfg(test)]
mod tests {
    use super::{Span, Trivalent};
    use crate::day::Day;

    const fn day(n: i32) -> Day {
        Day::new(n)
    }

    #[test]
    fn the_third_answer_propagates_the_way_kleene_says() {
        use Trivalent::{No, Unknown, Yes};

        // One false conjunct settles a conjunction whatever the other turns out to be.
        assert_eq!(No.and(Unknown), No);
        assert_eq!(Unknown.and(No), No);
        assert_eq!(Yes.and(Unknown), Unknown);
        assert_eq!(Yes.and(Yes), Yes);

        // And one true disjunct settles a disjunction.
        assert_eq!(Yes.or(Unknown), Yes);
        assert_eq!(No.or(Unknown), Unknown);
        assert_eq!(No.or(No), No);

        assert_eq!(Unknown.not(), Unknown);
        assert_eq!(Yes.not(), No);
    }

    #[test]
    fn collapsing_to_two_answers_makes_the_caller_say_which() {
        // The point of the signature: "alive in 1900" and "death precedes birth" want opposite
        // readings of the same Unknown, and neither should get it by default.
        assert!(Trivalent::Unknown.or_when_unknown(true));
        assert!(!Trivalent::Unknown.or_when_unknown(false));
        assert!(Trivalent::Yes.or_when_unknown(false));
        assert!(!Trivalent::No.or_when_unknown(true));
    }

    #[test]
    fn an_open_span_contains_everything_on_its_open_side() {
        let before_1875 = Span::until(day(1000));
        assert!(before_1875.contains(day(-500_000)));
        assert!(before_1875.contains(day(1000)));
        assert!(!before_1875.contains(day(1001)));

        let after = Span::from(day(1000));
        assert!(after.contains(day(500_000)));
        assert!(!after.contains(day(999)));

        assert!(Span::unbounded().contains(day(0)));
    }

    #[test]
    fn overlapping_spans_do_not_pretend_to_know_which_came_first() {
        // A birth between 1869 and 1873, a marriage between 1871 and 1875. The record does not
        // say which was first, and neither does this.
        let birth = Span::closed(day(100), day(500));
        let marriage = Span::closed(day(300), day(700));

        assert_eq!(birth.before(marriage), Trivalent::Unknown);
        assert_eq!(marriage.before(birth), Trivalent::Unknown);
        // The intervals meet, which is a fact; whether the two dates are the same day is not.
        assert!(birth.intersects(marriage));
        assert_eq!(birth.same_day_as(marriage), Trivalent::Unknown);
    }

    #[test]
    fn separated_spans_are_decided() {
        let first = Span::closed(day(100), day(200));
        let second = Span::closed(day(300), day(400));

        assert_eq!(first.before(second), Trivalent::Yes);
        assert_eq!(second.before(first), Trivalent::No);
        assert_eq!(first.after(second), Trivalent::No);
        assert_eq!(second.after(first), Trivalent::Yes);
        assert!(!first.intersects(second));
        assert_eq!(first.same_day_as(second), Trivalent::No);
    }

    #[test]
    fn touching_spans_overlap() {
        // Inclusive bounds: a span ending on the day another begins shares that day.
        let first = Span::closed(day(100), day(200));
        let second = Span::closed(day(200), day(300));
        assert!(first.intersects(second));
        assert_eq!(first.before(second), Trivalent::Unknown);
    }

    #[test]
    fn an_open_end_makes_ordering_undecidable() {
        // "After 1869" against "1871" cannot be ordered: the open end may reach past it.
        let after = Span::from(day(100));
        let point = Span::at(day(200));
        assert_eq!(after.before(point), Trivalent::Unknown);
        assert_eq!(point.before(after), Trivalent::Unknown);
        assert!(after.intersects(point));
    }

    #[test]
    fn reversed_bounds_are_read_as_the_range_between_them() {
        // A source that writes the years the wrong way round still means the years between.
        assert_eq!(
            Span::closed(day(400), day(100)),
            Span::closed(day(100), day(400))
        );
    }

    #[test]
    fn width_and_certainty() {
        assert!(Span::at(day(10)).is_certain());
        assert_eq!(Span::at(day(10)).length(), Some(1));
        assert_eq!(Span::closed(day(10), day(19)).length(), Some(10));
        assert!(!Span::from(day(10)).is_certain());
        assert_eq!(Span::from(day(10)).length(), None);
    }

    #[test]
    fn widening_leaves_open_ends_open() {
        let widened = Span::until(day(100)).widened(10);
        assert_eq!(
            widened.earliest, None,
            "there is nothing before the beginning"
        );
        assert_eq!(widened.latest, Some(day(110)));

        let both = Span::closed(day(100), day(200)).widened(10);
        assert_eq!(both, Span::closed(day(90), day(210)));
    }

    #[test]
    fn the_union_of_a_bounded_and_an_open_span_is_open() {
        let union = Span::closed(day(100), day(200)).union(Span::from(day(150)));
        assert_eq!(union.earliest, Some(day(100)));
        assert_eq!(union.latest, None);
    }
}
