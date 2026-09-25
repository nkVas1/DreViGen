//! The audit: what cannot be true.
//!
//! The archive accepts what sources say, including what cannot all be so, because refusing a
//! record teaches nothing and an import must never fail on its data. This module reads the whole
//! archive and reports the impossibilities: a child born before its mother, a burial before the
//! death, a person who is their own ancestor.
//!
//! # It must not cry wolf
//!
//! A finding is reported only when it is **certain** — when every reading of the dates, each
//! with the full tolerance its approximation earns, violates the rule. "Born about 1850" and a
//! child "born 1861" could be a mother of eleven or of fifteen, and the audit says nothing. An
//! audit that flags census arithmetic trains its reader to dismiss it, and then the real error in
//! the list goes unread. Every comparison here is three-valued, and only `Yes` is reported.
//!
//! # The thresholds are judgements
//!
//! Each is a constant below, with its reason. Errors are impossibilities; warnings are things a
//! researcher should look at twice because they are rare enough to suggest a mistake, not because
//! they cannot happen.

use std::collections::BTreeMap;

use drevigen_date::{Day, Span, Trivalent, gregorian};

use crate::archive::Archive;
use crate::id::{EventId, FamilyId, PersonId};
use crate::model::{EventKind, Family, Partnership, Role};
use crate::value::Sex;

/// The youngest a parent can plausibly be. Births to parents under thirteen are documented and
/// vanishingly rare; in a family tree they are almost always two people merged or a year misread.
pub const YOUNGEST_PARENT: i32 = 13;

/// The oldest a mother can plausibly be. Past fifty-five, natural conception is essentially
/// unrecorded before modern medicine, and the historical records this tool holds predate it.
pub const OLDEST_MOTHER: i32 = 55;

/// The oldest a father can plausibly be. Fathers in their seventies are recorded; past eighty, a
/// tree more often has a grandfather standing where a father should.
pub const OLDEST_FATHER: i32 = 80;

/// How long after a father's death a child can still be his. Ten months, with a margin: a child
/// born within it is posthumous, which is sad and ordinary; after it, the family is wrong.
pub const POSTHUMOUS_DAYS: i32 = 300;

/// The longest plausible life. The oldest verified lives are a little over 120; a recorded 110
/// is more often a father and son sharing a name.
pub const LONGEST_LIFE: i32 = 110;

/// The youngest a person can plausibly marry. Child marriage is recorded and was not rare in
/// some places and centuries; below thirteen it is more often a birth year misread.
pub const YOUNGEST_MARRIAGE: i32 = 13;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Rare enough to look at twice.
    Warning,
    /// Cannot be true.
    Error,
}

/// Which rule a finding breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rule {
    /// Someone is their own ancestor.
    OwnAncestor,
    /// A child was born before a parent.
    BornBeforeParent,
    /// A parent was younger than [`YOUNGEST_PARENT`] at a child's birth.
    ParentTooYoung,
    /// A mother was older than [`OLDEST_MOTHER`] at a child's birth.
    MotherTooOld,
    /// A father was older than [`OLDEST_FATHER`] at a child's birth.
    FatherTooOld,
    /// A child was born after its mother died.
    BornAfterMothersDeath,
    /// A child was born more than [`POSTHUMOUS_DAYS`] after its father died.
    BornAfterFathersDeath,
    /// A death is dated before the birth.
    DiedBeforeBorn,
    /// A baptism is dated before the birth.
    BaptisedBeforeBorn,
    /// A burial or cremation is dated before the death.
    BuriedBeforeDied,
    /// A life longer than [`LONGEST_LIFE`].
    ImplausibleLifespan,
    /// Someone took part in an event after they died, in a role that needs them alive.
    ActiveAfterDeath,
    /// Someone married younger than [`YOUNGEST_MARRIAGE`].
    MarriedTooYoung,
}

impl Rule {
    /// How serious a breach of this rule is.
    #[must_use]
    pub const fn severity(self) -> Severity {
        match self {
            Self::ParentTooYoung
            | Self::MotherTooOld
            | Self::FatherTooOld
            | Self::ImplausibleLifespan
            | Self::MarriedTooYoung => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

/// One thing the audit found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The rule broken.
    pub rule: Rule,
    /// Who it is about. For a family rule, the child first.
    pub people: Vec<PersonId>,
    /// The family it concerns, if any.
    pub family: Option<FamilyId>,
    /// The event it concerns, if any.
    pub event: Option<EventId>,
}

impl Finding {
    /// How serious it is.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.rule.severity()
    }
}

/// Reads the whole archive and reports what cannot be true, most serious first.
#[must_use]
pub fn audit(archive: &Archive) -> Vec<Finding> {
    let vitals: BTreeMap<PersonId, Vitals> = archive
        .people()
        .map(|person| (person.id, Vitals::of(archive, person.id)))
        .collect();

    let mut findings = Vec::new();
    findings.extend(own_ancestors(archive));
    for (person, life) in &vitals {
        life_order(*person, life, &mut findings);
    }
    for family in archive.families() {
        family_order(family, &vitals, &mut findings);
    }
    participation_order(archive, &vitals, &mut findings);

    findings.sort_by_key(|finding| core::cmp::Reverse(finding.severity()));
    findings
}

/// When a person's life began and ended, as far as the records fix it.
///
/// Built from the events a person is the principal of, each date taken with its full search
/// tolerance so that an approximation is never read as more certain than it is.
#[derive(Debug, Clone, Copy, Default)]
struct Vitals {
    birth: Option<Span>,
    baptism: Option<Span>,
    death: Option<Span>,
    burial: Option<Span>,
    /// Whether the person is recorded as female, where anything is recorded.
    female: Option<bool>,
}

impl Vitals {
    fn of(archive: &Archive, person: PersonId) -> Self {
        let mut vitals = Self {
            female: archive
                .person(person)
                .and_then(|p| p.sex.as_ref())
                .and_then(|fact| fact.resolve().value().copied())
                .and_then(|sex| match sex {
                    Sex::Female => Some(true),
                    Sex::Male => Some(false),
                    Sex::Other | Sex::Undetermined => None,
                }),
            ..Self::default()
        };

        for event in archive.events_of(person) {
            let principal = event.participants.iter().any(|participation| {
                participation.person == person
                    && participation.role.resolve().value() == Some(&Role::Principal)
            });
            let Some(span) = event
                .resolved_date()
                .map(drevigen_date::RecordedDate::search_span)
            else {
                continue;
            };
            if !principal || span == Span::unbounded() {
                continue;
            }
            let slot = match event.kind {
                EventKind::Birth => &mut vitals.birth,
                EventKind::Baptism | EventKind::Christening => &mut vitals.baptism,
                EventKind::Death => &mut vitals.death,
                EventKind::Burial | EventKind::Cremation => &mut vitals.burial,
                _ => continue,
            };
            slot.get_or_insert(span);
        }
        vitals
    }

    /// When the person was born: the birth itself, or else no later than the baptism.
    fn born(&self) -> Option<Span> {
        self.birth
            .or_else(|| self.baptism.and_then(|b| b.latest).map(Span::until))
    }

    /// When the person died: the death itself, or else no later than the burial.
    fn died(&self) -> Option<Span> {
        self.death
            .or_else(|| self.burial.and_then(|b| b.latest).map(Span::until))
    }
}

/// A span moved forward by whole calendar years.
///
/// Exact rather than averaged. Year-precision dates sit on 1 January and 31 December, which is
/// exactly where an average year length goes wrong: 1 January 1860 plus thirteen years of 365.25
/// days is 31 December 1872, a day short, and a mother born in 1860 with a child born in 1872 —
/// under thirteen however the two years are read — went unreported until this was calendar
/// arithmetic.
fn plus_years(span: Span, years: i32) -> Span {
    Span {
        earliest: span.earliest.map(|day| add_years(day, years)),
        latest: span.latest.map(|day| add_years(day, years)),
    }
}

/// A day moved forward by whole calendar years, with 29 February landing on 28 February in a
/// year that has none.
fn add_years(day: Day, years: i32) -> Day {
    let (year, month, date) = gregorian::from_day(day);
    let target = year + years;
    let date = date.min(gregorian::days_in_month(target, month).unwrap_or(28));
    gregorian::to_day(target, month, date)
}

/// Whether `a` is certainly before `b`, when both are known.
fn certainly_before(a: Option<Span>, b: Option<Span>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.before(b) == Trivalent::Yes,
        _ => false,
    }
}

/// The order of one life: born, baptised, died, buried.
fn life_order(person: PersonId, life: &Vitals, findings: &mut Vec<Finding>) {
    let mut report = |rule| {
        findings.push(Finding {
            rule,
            people: vec![person],
            family: None,
            event: None,
        });
    };
    if certainly_before(life.death, life.birth) {
        report(Rule::DiedBeforeBorn);
    }
    if certainly_before(life.baptism, life.birth) {
        report(Rule::BaptisedBeforeBorn);
    }
    if certainly_before(life.burial, life.death) {
        report(Rule::BuriedBeforeDied);
    }
    let longest = life.born().map(|born| plus_years(born, LONGEST_LIFE));
    if certainly_before(longest, life.died()) {
        report(Rule::ImplausibleLifespan);
    }
}

/// The order of a family: each child against each parent.
fn family_order(family: &Family, vitals: &BTreeMap<PersonId, Vitals>, findings: &mut Vec<Finding>) {
    let unknown = Vitals::default();
    let life = |person: &PersonId| vitals.get(person).unwrap_or(&unknown);

    for child in family.current_children() {
        let Some(born) = life(&child).born() else {
            continue;
        };
        for parent in family.current_partners() {
            if parent == child {
                // Imported as their own parent; the cycle finding already says so.
                continue;
            }
            let parents_life = life(&parent);
            let mut report = |rule| {
                findings.push(Finding {
                    rule,
                    people: vec![child, parent],
                    family: Some(family.id),
                    event: None,
                });
            };

            let parent_born = parents_life.born();
            if certainly_before(Some(born), parent_born) {
                // Every age rule would also fire. One finding, the one that names the mistake.
                report(Rule::BornBeforeParent);
                continue;
            }
            let came_of_age = parent_born.map(|b| plus_years(b, YOUNGEST_PARENT));
            if certainly_before(Some(born), came_of_age) {
                report(Rule::ParentTooYoung);
            }

            let female = partnership(family, parent)
                .map(|p| p == Partnership::Wife)
                .or(parents_life.female);
            let (oldest, after_death, rule) = match female {
                Some(true) => (OLDEST_MOTHER, 0, Rule::BornAfterMothersDeath),
                Some(false) => (OLDEST_FATHER, POSTHUMOUS_DAYS, Rule::BornAfterFathersDeath),
                None => continue,
            };
            let too_old = parent_born.map(|b| plus_years(b, oldest));
            if certainly_before(too_old, Some(born)) {
                report(if female == Some(true) {
                    Rule::MotherTooOld
                } else {
                    Rule::FatherTooOld
                });
            }
            let last_chance = parents_life.died().map(|d| d.shifted(after_death));
            if certainly_before(last_chance, Some(born)) {
                report(rule);
            }
        }
    }
}

/// How a partner is recorded in a family, as resolved.
fn partnership(family: &Family, person: PersonId) -> Option<Partnership> {
    family
        .partners
        .iter()
        .find(|membership| membership.person == person)
        .and_then(|membership| membership.fact.resolve().value().copied())
}

/// Events against the lives of the people in them.
fn participation_order(
    archive: &Archive,
    vitals: &BTreeMap<PersonId, Vitals>,
    findings: &mut Vec<Finding>,
) {
    for event in archive.events() {
        let Some(when) = event
            .resolved_date()
            .map(drevigen_date::RecordedDate::search_span)
        else {
            continue;
        };
        for participation in &event.participants {
            let Some(role) = participation.role.resolve().value().copied() else {
                continue;
            };
            let Some(life) = vitals.get(&participation.person) else {
                continue;
            };
            let mut report = |rule| {
                findings.push(Finding {
                    rule,
                    people: vec![participation.person],
                    family: None,
                    event: Some(event.id),
                });
            };

            if needs_the_living(role, event.kind) && certainly_before(life.died(), Some(when)) {
                report(Rule::ActiveAfterDeath);
            }
            let married = event.kind == EventKind::Marriage
                && matches!(role, Role::Husband | Role::Wife | Role::Spouse);
            let of_age = life.born().map(|b| plus_years(b, YOUNGEST_MARRIAGE));
            if married && certainly_before(Some(when), of_age) {
                report(Rule::MarriedTooYoung);
            }
        }
    }
}

/// Whether taking part in an event in this role requires being alive for it.
///
/// A marriage record names the couple's parents, dead or alive, and a baptism names a father who
/// died before the birth; neither is an error. A godparent, a witness or a bride must have been
/// there. As principal, only what is done to the dead — a burial, a cremation, a probate — can
/// follow the death.
const fn needs_the_living(role: Role, kind: EventKind) -> bool {
    match role {
        Role::Principal => !matches!(
            kind,
            EventKind::Burial | EventKind::Cremation | EventKind::Probate | EventKind::Death
        ),
        Role::Husband
        | Role::Wife
        | Role::Spouse
        | Role::Godparent
        | Role::Witness
        | Role::Clergy
        | Role::Officiator
        | Role::Informant => true,
        Role::Father
        | Role::Mother
        | Role::Parent
        | Role::Child
        | Role::Friend
        | Role::Neighbour
        | Role::Other => false,
    }
}

/// People who are their own ancestors, one finding per knot.
///
/// Tarjan's strongly-connected components over the parent-to-child graph, iteratively, so a tree
/// of any depth cannot overflow the stack. A component of more than one person is a loop of
/// descent; a component of one with an edge to itself is a person recorded as their own child.
fn own_ancestors(archive: &Archive) -> Vec<Finding> {
    let people: Vec<PersonId> = archive.people().map(|person| person.id).collect();
    let index_of: BTreeMap<PersonId, usize> = people
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();

    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); people.len()];
    for family in archive.families() {
        let children: Vec<usize> = family
            .current_children()
            .filter_map(|c| index_of.get(&c).copied())
            .collect();
        for parent in family
            .current_partners()
            .filter_map(|p| index_of.get(&p).copied())
        {
            edges[parent].extend(&children);
        }
    }

    strongly_connected(&edges)
        .into_iter()
        .filter(|component| {
            component.len() > 1 || component.first().is_some_and(|&v| edges[v].contains(&v))
        })
        .map(|component| {
            let mut members: Vec<PersonId> = component.iter().map(|&i| people[i]).collect();
            members.sort_unstable();
            Finding {
                rule: Rule::OwnAncestor,
                people: members,
                family: None,
                event: None,
            }
        })
        .collect()
}

/// Tarjan's algorithm without recursion.
fn strongly_connected(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    const UNVISITED: usize = usize::MAX;
    let count = edges.len();
    let mut index = vec![UNVISITED; count];
    let mut low = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut stack = Vec::new();
    let mut next = 0;
    let mut components = Vec::new();

    for start in 0..count {
        if index[start] != UNVISITED {
            continue;
        }
        let mut calls: Vec<(usize, usize)> = vec![(start, 0)];
        index[start] = next;
        low[start] = next;
        next += 1;
        stack.push(start);
        on_stack[start] = true;

        while let Some(&(node, edge)) = calls.last() {
            if let Some(&target) = edges[node].get(edge) {
                if let Some(top) = calls.last_mut() {
                    top.1 += 1;
                }
                if index[target] == UNVISITED {
                    index[target] = next;
                    low[target] = next;
                    next += 1;
                    stack.push(target);
                    on_stack[target] = true;
                    calls.push((target, 0));
                } else if on_stack[target] {
                    low[node] = low[node].min(index[target]);
                }
                continue;
            }

            calls.pop();
            if let Some(&(caller, _)) = calls.last() {
                low[caller] = low[caller].min(low[node]);
            }
            if low[node] == index[node] {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                components.push(component);
            }
        }
    }
    components
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
