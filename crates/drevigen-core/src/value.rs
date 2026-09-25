//! The values that can be asserted, and what agreement means for each.
//!
//! [`Claimable`] asks two questions of every value type — are two values compatible, and does one
//! say more than the other — and ADR 0010 is explicit that the answers are per type. This module
//! holds the answers for the simple ones. Names are complicated enough to have their own.

use drevigen_date::{RecordedDate, Trivalent};

use crate::claim::Claimable;

/// Two recorded dates agree when they could describe the same day.
///
/// Each keeps the tolerance its approximation earns, because this is exactly the question the
/// tolerance exists for: a register's "17 April 1871" and a census's "about 1873" could be the
/// same birth, and treating them as a conflict would fill the contradiction queue with census
/// arithmetic. "1871" against "1880" cannot, and is a conflict.
///
/// A date recorded only as words constrains nothing, so it contradicts nothing — and the date that
/// has a value always says more than one that does not.
impl Claimable for RecordedDate {
    fn compatible(&self, other: &Self) -> bool {
        self.could_coincide(other) != Trivalent::No
    }

    fn more_specific_than(&self, other: &Self) -> bool {
        match (self.stated_span().length(), other.stated_span().length()) {
            (Some(this), Some(that)) => this < that,
            // A bounded span says more than one open at an end, and anything says more than a
            // phrase.
            (Some(_), None) => true,
            _ => false,
        }
    }
}

/// Sex as a source records it.
///
/// GEDCOM 7's four values, kept distinct so that an import and an export carry the same thing.
/// The fourth matters for the model: a source that cannot determine sex has not contradicted one
/// that can.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sex {
    /// `F`.
    Female,
    /// `M`.
    Male,
    /// `X` — "does not fit the typical definition of only male or only female".
    Other,
    /// `U` — "cannot be determined from available sources".
    Undetermined,
}

impl Sex {
    /// The GEDCOM 7 enumeration value.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Female => "F",
            Self::Male => "M",
            Self::Other => "X",
            Self::Undetermined => "U",
        }
    }

    /// Reads a GEDCOM 7 enumeration value.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "F" => Some(Self::Female),
            "M" => Some(Self::Male),
            "X" => Some(Self::Other),
            "U" => Some(Self::Undetermined),
            _ => None,
        }
    }
}

impl Claimable for Sex {
    fn compatible(&self, other: &Self) -> bool {
        self == other || *self == Self::Undetermined || *other == Self::Undetermined
    }

    fn more_specific_than(&self, other: &Self) -> bool {
        *other == Self::Undetermined && *self != Self::Undetermined
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use drevigen_date::RecordedDate;

    use super::Sex;
    use crate::claim::{Assertion, Claimable, Claims, Provenance, Resolution, Timestamp};
    use crate::id::{AssertionId, ContributorId};

    fn date(text: &str) -> RecordedDate {
        RecordedDate::parse_gedcom7(text, None).unwrap()
    }

    fn claims_of(values: &[RecordedDate]) -> Claims<RecordedDate> {
        let mut claims = Claims::new();
        for (index, value) in values.iter().enumerate() {
            let n = u128::try_from(index).unwrap() + 1;
            claims
                .assert(Assertion::new(
                    AssertionId::from_raw(n),
                    value.clone(),
                    Provenance {
                        by: ContributorId::from_raw(1),
                        at: Timestamp(i64::try_from(index).unwrap()),
                    },
                ))
                .unwrap();
        }
        claims
    }

    #[test]
    fn a_register_and_a_census_describing_one_birth_agree() {
        // Year, then the day from a register, then a census age: one birth, three precisions.
        let claims = claims_of(&[date("1871"), date("17 APR 1871"), date("ABT 1873")]);
        let Resolution::Agreed { assertion, live } = claims.resolve() else {
            panic!("these could all be the same birth")
        };
        assert_eq!(
            assertion.value,
            date("17 APR 1871"),
            "the most specific wins"
        );
        assert_eq!(live, 3);
    }

    #[test]
    fn dates_nine_years_apart_are_a_question() {
        let claims = claims_of(&[date("1871"), date("1880")]);
        assert!(claims.resolve().is_open_question());
    }

    #[test]
    fn an_open_bound_agrees_with_what_it_contains_and_says_less() {
        let claims = claims_of(&[date("BEF 1875"), date("1871")]);
        assert_eq!(claims.resolve().value(), Some(&date("1871")));
    }

    #[test]
    fn a_phrase_contradicts_nothing_and_yields_to_any_date() {
        let phrase = RecordedDate::from_phrase("после пожара");
        assert!(phrase.compatible(&date("1871")));
        assert!(date("1871").more_specific_than(&phrase));
        assert!(!phrase.more_specific_than(&date("1871")));
    }

    #[test]
    fn a_source_that_cannot_tell_does_not_contradict_one_that_can() {
        assert!(Sex::Undetermined.compatible(&Sex::Female));
        assert!(Sex::Female.more_specific_than(&Sex::Undetermined));
        assert!(!Sex::Female.compatible(&Sex::Male));
        assert!(!Sex::Female.more_specific_than(&Sex::Female));
    }

    #[test]
    fn sex_round_trips_through_gedcom() {
        for sex in [Sex::Female, Sex::Male, Sex::Other, Sex::Undetermined] {
            assert_eq!(Sex::from_tag(sex.tag()), Some(sex));
        }
        assert_eq!(
            Sex::from_tag("f"),
            None,
            "GEDCOM enumerations are case-sensitive"
        );
    }
}
