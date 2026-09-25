//! Claims: what sources assert about a fact, and what a researcher concluded.
//!
//! This is ADR 0005 made into code, with the questions ADR 0010 settled. Every fact — a name, a
//! birth date, a place, a membership of a family — holds a [`Claims`]: the assertions sources
//! make about it, and the conclusions a researcher reached. Nothing is overwritten. A second
//! source that disagrees with the first adds an assertion beside it, and the disagreement is data.
//!
//! # Agreement is compatibility
//!
//! Two assertions agree when both could be true of the same fact. "1871" and "17 April 1871"
//! agree; the second says more. "1871" and "1873" do not. Each value type says what compatible
//! means for it by implementing [`Claimable`], and agreeing assertions resolve to the most
//! specific value among them. Only assertions that cannot all be true together make an open
//! question.

use crate::id::{AssertionId, CitationId, ConclusionId, ContributorId};

/// Milliseconds since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub i64);

/// Who recorded something, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provenance {
    /// The contributor.
    pub by: ContributorId,
    /// The moment.
    pub at: Timestamp,
}

/// How far a piece of evidence can be trusted.
///
/// The four levels of GEDCOM's `QUAY`, named for what they mean rather than numbered, so that an
/// import and an export of the same file carry the same judgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Confidence {
    /// `QUAY 0` — unreliable evidence or estimated data.
    Unreliable,
    /// `QUAY 1` — questionable reliability: an interview, a census, a biography.
    Questionable,
    /// `QUAY 2` — secondary evidence, recorded some time after the event.
    Secondary,
    /// `QUAY 3` — direct and primary evidence, recorded at or near the time.
    Primary,
}

impl Confidence {
    /// The GEDCOM `QUAY` value.
    #[must_use]
    pub const fn quay(self) -> u8 {
        match self {
            Self::Unreliable => 0,
            Self::Questionable => 1,
            Self::Secondary => 2,
            Self::Primary => 3,
        }
    }

    /// Reads a GEDCOM `QUAY` value.
    #[must_use]
    pub const fn from_quay(quay: u8) -> Option<Self> {
        match quay {
            0 => Some(Self::Unreliable),
            1 => Some(Self::Questionable),
            2 => Some(Self::Secondary),
            3 => Some(Self::Primary),
            _ => None,
        }
    }
}

/// Where an assessment sits when choosing between disagreeing assertions.
///
/// An assertion nobody has assessed ranks between secondary and questionable. It has not been
/// found wanting, so it outranks what has; it has not been found reliable either.
const fn rank(confidence: Option<Confidence>) -> u8 {
    match confidence {
        Some(Confidence::Primary) => 4,
        Some(Confidence::Secondary) => 3,
        None => 2,
        Some(Confidence::Questionable) => 1,
        Some(Confidence::Unreliable) => 0,
    }
}

/// A value that can be asserted about a fact.
///
/// The two questions ADR 0010 puts to every value type.
pub trait Claimable {
    /// Whether this and another value could both be true of the same fact.
    ///
    /// Must be symmetric. Erring towards `false` invents conflicts; erring towards `true` hides
    /// them — the second is worse, and a type that cannot decide should say `false`.
    fn compatible(&self, other: &Self) -> bool;

    /// Whether this value says strictly more than another compatible one.
    ///
    /// "17 April 1871" says more than "1871". Used to choose what an agreement resolves to.
    fn more_specific_than(&self, other: &Self) -> bool;
}

/// What one source says about one fact.
#[derive(Debug, Clone, PartialEq)]
pub struct Assertion<V> {
    /// This assertion.
    pub id: AssertionId,
    /// What it asserts.
    pub value: V,
    /// Where it says so. `None` for something a relative typed in from memory, which is an
    /// assertion too — an unsourced one, and shown as such.
    pub citation: Option<CitationId>,
    /// How far it can be trusted, if anyone has judged.
    pub confidence: Option<Confidence>,
    /// Who recorded it, and when.
    pub made: Provenance,
    /// Who withdrew it, and when. A withdrawn assertion stays in the history and stops counting.
    pub retracted: Option<Provenance>,
}

impl<V> Assertion<V> {
    /// An assertion as first recorded: not yet assessed, not withdrawn.
    pub const fn new(id: AssertionId, value: V, made: Provenance) -> Self {
        Self {
            id,
            value,
            citation: None,
            confidence: None,
            made,
            retracted: None,
        }
    }

    /// Whether it still counts.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.retracted.is_none()
    }

    /// The key a disagreement is settled by, highest first: assessed reliability, then whether
    /// it cites anything, then how early it was recorded.
    fn precedence(&self, index: usize) -> (u8, bool, core::cmp::Reverse<(i64, usize)>) {
        (
            rank(self.confidence),
            self.citation.is_some(),
            core::cmp::Reverse((self.made.at.0, index)),
        )
    }
}

/// What a researcher decided about a fact, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Conclusion<V> {
    /// This conclusion.
    pub id: ConclusionId,
    /// The value decided on. Usually one of the asserted values or a refinement of one; nothing
    /// requires it, because a researcher may reason to a value no single source states.
    pub value: V,
    /// The assertions the reasoning relies on.
    pub supported_by: Vec<AssertionId>,
    /// The assertions the reasoning explains away.
    pub contradicted_by: Vec<AssertionId>,
    /// The argument. The Genealogical Proof Standard asks for one; the model does not refuse a
    /// conclusion without it, because a relative choosing "this one is right" is still deciding.
    pub reasoning: String,
    /// Who decided, and when.
    pub made: Provenance,
    /// Who withdrew it, and when, returning the fact to its assertions.
    pub withdrawn: Option<Provenance>,
}

/// Why a claim operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimError {
    /// No assertion with that identity belongs to this fact.
    UnknownAssertion(AssertionId),
    /// An assertion with that identity is already recorded.
    DuplicateAssertion(AssertionId),
    /// The assertion was already withdrawn.
    AlreadyRetracted(AssertionId),
    /// There is no standing conclusion to withdraw.
    NoConclusion,
}

impl core::fmt::Display for ClaimError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownAssertion(id) => write!(f, "{id:?} does not belong to this fact"),
            Self::DuplicateAssertion(id) => write!(f, "{id:?} is already recorded"),
            Self::AlreadyRetracted(id) => write!(f, "{id:?} was already withdrawn"),
            Self::NoConclusion => f.write_str("there is no standing conclusion to withdraw"),
        }
    }
}

impl core::error::Error for ClaimError {}

/// Everything asserted and concluded about one fact.
///
/// Append-only. Assertions are withdrawn, never removed; conclusions are revised by adding a new
/// one, never edited. The history of what was believed and why is part of the record.
#[derive(Debug, Clone, PartialEq)]
pub struct Claims<V> {
    assertions: Vec<Assertion<V>>,
    conclusions: Vec<Conclusion<V>>,
}

impl<V> Default for Claims<V> {
    fn default() -> Self {
        Self {
            assertions: Vec::new(),
            conclusions: Vec::new(),
        }
    }
}

/// What a fact comes to, once its claims are weighed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolution<'a, V> {
    /// Nothing is asserted.
    Unknown,
    /// A researcher decided.
    Concluded {
        /// The standing conclusion.
        conclusion: &'a Conclusion<V>,
        /// Live assertions the conclusion neither relies on nor explains away. Evidence found
        /// after a conclusion was reached, and the reason a conclusion is not forever.
        unconsidered: usize,
    },
    /// Every live assertion is compatible with every other.
    Agreed {
        /// The most specific of them.
        assertion: &'a Assertion<V>,
        /// How many live assertions there are.
        live: usize,
    },
    /// The sources disagree, and nobody has decided. An open question.
    Contested {
        /// What to show meanwhile — highest confidence, then cited, then earliest. Always shown
        /// as contested; never to be treated as the answer.
        provisional: &'a Assertion<V>,
        /// How many live assertions there are.
        live: usize,
    },
}

impl<'a, V> Resolution<'a, V> {
    /// The value to show, if any: concluded, agreed or provisional.
    #[must_use]
    pub const fn value(&self) -> Option<&'a V> {
        match self {
            Self::Unknown => None,
            Self::Concluded { conclusion, .. } => Some(&conclusion.value),
            Self::Agreed { assertion, .. }
            | Self::Contested {
                provisional: assertion,
                ..
            } => Some(&assertion.value),
        }
    }

    /// Whether this is a question waiting for a researcher.
    #[must_use]
    pub const fn is_open_question(&self) -> bool {
        matches!(self, Self::Contested { .. })
    }
}

impl<V: Claimable> Claims<V> {
    /// No claims yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every assertion ever made, withdrawn ones included, in the order recorded.
    #[must_use]
    pub fn assertions(&self) -> &[Assertion<V>] {
        &self.assertions
    }

    /// The assertions that still count.
    pub fn live(&self) -> impl Iterator<Item = &Assertion<V>> {
        self.assertions
            .iter()
            .filter(|assertion| assertion.is_live())
    }

    /// Every conclusion ever reached, in order. The last is standing unless it was withdrawn.
    #[must_use]
    pub fn conclusions(&self) -> &[Conclusion<V>] {
        &self.conclusions
    }

    /// The conclusion that currently stands.
    #[must_use]
    pub fn conclusion(&self) -> Option<&Conclusion<V>> {
        self.conclusions
            .last()
            .filter(|conclusion| conclusion.withdrawn.is_none())
    }

    /// Records an assertion.
    ///
    /// # Errors
    ///
    /// [`ClaimError::DuplicateAssertion`] if one with the same identity is already recorded — the
    /// case a replayed sync message produces, and one to refuse rather than double-count.
    pub fn assert(&mut self, assertion: Assertion<V>) -> Result<(), ClaimError> {
        if self.find(assertion.id).is_some() {
            return Err(ClaimError::DuplicateAssertion(assertion.id));
        }
        self.assertions.push(assertion);
        Ok(())
    }

    /// Withdraws an assertion. It stays in the history and stops counting.
    ///
    /// # Errors
    ///
    /// [`ClaimError::UnknownAssertion`] or [`ClaimError::AlreadyRetracted`].
    pub fn retract(&mut self, id: AssertionId, by: Provenance) -> Result<(), ClaimError> {
        let index = self.find(id).ok_or(ClaimError::UnknownAssertion(id))?;
        let assertion = &mut self.assertions[index];
        if assertion.retracted.is_some() {
            return Err(ClaimError::AlreadyRetracted(id));
        }
        assertion.retracted = Some(by);
        Ok(())
    }

    /// Records a conclusion, which then stands until revised or withdrawn.
    ///
    /// # Errors
    ///
    /// [`ClaimError::UnknownAssertion`] if the reasoning cites an assertion this fact does not
    /// have. A conclusion that relies on evidence that is not there is not a conclusion.
    pub fn conclude(&mut self, conclusion: Conclusion<V>) -> Result<(), ClaimError> {
        for id in conclusion
            .supported_by
            .iter()
            .chain(&conclusion.contradicted_by)
        {
            if self.find(*id).is_none() {
                return Err(ClaimError::UnknownAssertion(*id));
            }
        }
        self.conclusions.push(conclusion);
        Ok(())
    }

    /// Withdraws the standing conclusion, returning the fact to its assertions.
    ///
    /// # Errors
    ///
    /// [`ClaimError::NoConclusion`] if none stands.
    pub fn withdraw_conclusion(&mut self, by: Provenance) -> Result<(), ClaimError> {
        match self.conclusions.last_mut() {
            Some(conclusion) if conclusion.withdrawn.is_none() => {
                conclusion.withdrawn = Some(by);
                Ok(())
            }
            _ => Err(ClaimError::NoConclusion),
        }
    }

    /// Weighs the claims. See ADR 0010 for the order, and why it is that order.
    #[must_use]
    pub fn resolve(&self) -> Resolution<'_, V> {
        if let Some(conclusion) = self.conclusion() {
            let considered = |id: &AssertionId| {
                conclusion.supported_by.contains(id) || conclusion.contradicted_by.contains(id)
            };
            return Resolution::Concluded {
                conclusion,
                unconsidered: self.live().filter(|a| !considered(&a.id)).count(),
            };
        }

        let live: Vec<(usize, &Assertion<V>)> = self
            .assertions
            .iter()
            .enumerate()
            .filter(|(_, assertion)| assertion.is_live())
            .collect();

        let Some(&(first_index, first)) = live.first() else {
            return Resolution::Unknown;
        };

        let agreed = live.iter().enumerate().all(|(i, (_, a))| {
            live[i + 1..]
                .iter()
                .all(|(_, b)| a.value.compatible(&b.value))
        });

        if agreed {
            // The most specific value; between two that say the same amount, the one with the
            // stronger claim to be shown.
            let (_, chosen) = live.iter().skip(1).fold(
                (first_index, first),
                |(best_index, best), &(index, candidate)| {
                    let says_more = candidate.value.more_specific_than(&best.value);
                    let says_same = !best.value.more_specific_than(&candidate.value);
                    let stronger = candidate.precedence(index) > best.precedence(best_index);
                    if says_more || (says_same && stronger) {
                        (index, candidate)
                    } else {
                        (best_index, best)
                    }
                },
            );
            return Resolution::Agreed {
                assertion: chosen,
                live: live.len(),
            };
        }

        let (_, provisional) = live
            .iter()
            .max_by_key(|(index, assertion)| assertion.precedence(*index))
            .copied()
            .unwrap_or((first_index, first));
        Resolution::Contested {
            provisional,
            live: live.len(),
        }
    }

    fn find(&self, id: AssertionId) -> Option<usize> {
        self.assertions
            .iter()
            .position(|assertion| assertion.id == id)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{
        Assertion, ClaimError, Claimable, Claims, Conclusion, Confidence, Provenance, Resolution,
        Timestamp,
    };
    use crate::id::{AssertionId, CitationId, ConclusionId, ContributorId};

    /// A year range, as the simplest value with a real notion of compatibility and specificity.
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Years(i32, i32);

    impl Claimable for Years {
        fn compatible(&self, other: &Self) -> bool {
            self.0 <= other.1 && other.0 <= self.1
        }
        fn more_specific_than(&self, other: &Self) -> bool {
            self.1 - self.0 < other.1 - other.0
        }
    }

    fn at(millis: i64) -> Provenance {
        Provenance {
            by: ContributorId::from_raw(1),
            at: Timestamp(millis),
        }
    }

    fn assertion(n: u128, value: Years, made: i64) -> Assertion<Years> {
        Assertion::new(AssertionId::from_raw(n), value, at(made))
    }

    #[test]
    fn nothing_asserted_is_unknown() {
        assert_eq!(Claims::<Years>::new().resolve(), Resolution::Unknown);
    }

    #[test]
    fn a_refinement_is_agreement_and_resolves_to_what_says_more() {
        // "1871" then "17 April 1871": the second is narrower and compatible, so it wins and
        // nothing is contested. The whole point of ADR 0010.
        let mut claims = Claims::new();
        claims
            .assert(assertion(1, Years(1871, 1871 + 1), 10))
            .unwrap();
        claims.assert(assertion(2, Years(1871, 1871), 20)).unwrap();

        let Resolution::Agreed { assertion, live } = claims.resolve() else {
            panic!("a refinement is not a conflict")
        };
        assert_eq!(assertion.value, Years(1871, 1871));
        assert_eq!(live, 2);
    }

    #[test]
    fn incompatible_sources_are_an_open_question() {
        let mut claims = Claims::new();
        claims.assert(assertion(1, Years(1871, 1871), 10)).unwrap();
        claims.assert(assertion(2, Years(1873, 1873), 20)).unwrap();

        let resolution = claims.resolve();
        assert!(resolution.is_open_question());
        // The provisional value is the earlier one when nothing else separates them.
        assert_eq!(resolution.value(), Some(&Years(1871, 1871)));
    }

    #[test]
    fn a_contest_is_provisionally_led_by_the_strongest_evidence() {
        let mut claims = Claims::new();
        claims.assert(assertion(1, Years(1871, 1871), 10)).unwrap();

        let mut register = assertion(2, Years(1873, 1873), 20);
        register.confidence = Some(Confidence::Primary);
        register.citation = Some(CitationId::from_raw(9));
        claims.assert(register).unwrap();

        let mut guess = assertion(3, Years(1875, 1875), 5);
        guess.confidence = Some(Confidence::Unreliable);
        claims.assert(guess).unwrap();

        let Resolution::Contested { provisional, live } = claims.resolve() else {
            panic!()
        };
        assert_eq!(
            provisional.value,
            Years(1873, 1873),
            "primary evidence leads"
        );
        assert_eq!(live, 3);
    }

    #[test]
    fn an_unassessed_assertion_outranks_a_doubted_one() {
        let mut claims = Claims::new();
        let mut doubted = assertion(1, Years(1871, 1871), 10);
        doubted.confidence = Some(Confidence::Questionable);
        claims.assert(doubted).unwrap();
        claims.assert(assertion(2, Years(1873, 1873), 20)).unwrap();
        assert_eq!(claims.resolve().value(), Some(&Years(1873, 1873)));
    }

    #[test]
    fn a_cited_assertion_outranks_an_uncited_one_of_equal_confidence() {
        let mut claims = Claims::new();
        claims.assert(assertion(1, Years(1871, 1871), 10)).unwrap();
        let mut cited = assertion(2, Years(1873, 1873), 20);
        cited.citation = Some(CitationId::from_raw(4));
        claims.assert(cited).unwrap();
        assert_eq!(claims.resolve().value(), Some(&Years(1873, 1873)));
    }

    #[test]
    fn withdrawing_the_dissenting_source_ends_the_contest() {
        let mut claims = Claims::new();
        claims.assert(assertion(1, Years(1871, 1871), 10)).unwrap();
        claims.assert(assertion(2, Years(1873, 1873), 20)).unwrap();
        assert!(claims.resolve().is_open_question());

        claims.retract(AssertionId::from_raw(2), at(30)).unwrap();
        assert!(matches!(
            claims.resolve(),
            Resolution::Agreed { live: 1, .. }
        ));

        // Still in the history.
        assert_eq!(claims.assertions().len(), 2);
        assert_eq!(
            claims.retract(AssertionId::from_raw(2), at(40)),
            Err(ClaimError::AlreadyRetracted(AssertionId::from_raw(2)))
        );
    }

    #[test]
    fn a_conclusion_settles_it_and_reports_what_it_did_not_consider() {
        let mut claims = Claims::new();
        claims.assert(assertion(1, Years(1871, 1871), 10)).unwrap();
        claims.assert(assertion(2, Years(1873, 1873), 20)).unwrap();

        claims
            .conclude(Conclusion {
                id: ConclusionId::from_raw(1),
                value: Years(1871, 1871),
                supported_by: vec![AssertionId::from_raw(1)],
                contradicted_by: vec![AssertionId::from_raw(2)],
                reasoning: "The register was written at the baptism; the census from memory."
                    .to_owned(),
                made: at(30),
                withdrawn: None,
            })
            .unwrap();

        let Resolution::Concluded { unconsidered, .. } = claims.resolve() else {
            panic!()
        };
        assert_eq!(unconsidered, 0);

        // New evidence arrives after the conclusion. It stands — and says it has not seen this.
        claims.assert(assertion(3, Years(1872, 1872), 40)).unwrap();
        let Resolution::Concluded {
            conclusion,
            unconsidered,
        } = claims.resolve()
        else {
            panic!()
        };
        assert_eq!(conclusion.value, Years(1871, 1871));
        assert_eq!(unconsidered, 1);
    }

    #[test]
    fn a_conclusion_cannot_rely_on_evidence_that_is_not_there() {
        let mut claims = Claims::<Years>::new();
        let refused = claims.conclude(Conclusion {
            id: ConclusionId::from_raw(1),
            value: Years(1871, 1871),
            supported_by: vec![AssertionId::from_raw(99)],
            contradicted_by: vec![],
            reasoning: String::new(),
            made: at(1),
            withdrawn: None,
        });
        assert_eq!(
            refused,
            Err(ClaimError::UnknownAssertion(AssertionId::from_raw(99)))
        );
    }

    #[test]
    fn withdrawing_a_conclusion_returns_the_fact_to_its_sources() {
        let mut claims = Claims::new();
        claims.assert(assertion(1, Years(1871, 1871), 10)).unwrap();
        claims.assert(assertion(2, Years(1873, 1873), 20)).unwrap();
        claims
            .conclude(Conclusion {
                id: ConclusionId::from_raw(1),
                value: Years(1873, 1873),
                supported_by: vec![],
                contradicted_by: vec![],
                reasoning: String::new(),
                made: at(30),
                withdrawn: None,
            })
            .unwrap();
        claims.withdraw_conclusion(at(40)).unwrap();

        assert!(claims.resolve().is_open_question());
        assert_eq!(claims.conclusions().len(), 1, "the history keeps it");
        assert_eq!(
            claims.withdraw_conclusion(at(50)),
            Err(ClaimError::NoConclusion)
        );
    }

    #[test]
    fn a_replayed_assertion_is_refused_rather_than_counted_twice() {
        let mut claims = Claims::new();
        claims.assert(assertion(1, Years(1871, 1871), 10)).unwrap();
        assert_eq!(
            claims.assert(assertion(1, Years(1871, 1871), 10)),
            Err(ClaimError::DuplicateAssertion(AssertionId::from_raw(1)))
        );
    }

    #[test]
    fn the_quality_scale_round_trips_through_gedcom() {
        for quay in 0..=3 {
            assert_eq!(Confidence::from_quay(quay).unwrap().quay(), quay);
        }
        assert_eq!(Confidence::from_quay(4), None);
    }
}
