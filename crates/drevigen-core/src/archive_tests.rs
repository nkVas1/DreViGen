//! The archive's behaviour: relationships read through their claims, and the edits it refuses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use drevigen_date::RecordedDate;

use super::{Archive, Entity, Refusal};
use crate::claim::{Assertion, ClaimError, Conclusion, Provenance, Resolution, Timestamp};
use crate::id::{AssertionId, ConclusionId, ContributorId, EventId, FactId, FamilyId, PersonId};
use crate::model::{ChildRelation, Event, EventKind, Family, Partnership, Person, Role};
use crate::name::PersonName;

/// Deterministic identities for tests: every call returns the next number.
struct Mint(u128);

impl Mint {
    fn next(&mut self) -> u128 {
        self.0 += 1;
        self.0
    }
    fn person(&mut self) -> PersonId {
        PersonId::from_raw(self.next())
    }
    fn family(&mut self) -> FamilyId {
        FamilyId::from_raw(self.next())
    }
    fn fact(&mut self) -> FactId {
        FactId::from_raw(self.next())
    }
    fn assertion<V>(&mut self, value: V) -> Assertion<V> {
        Assertion::new(AssertionId::from_raw(self.next()), value, made(0))
    }
}

fn made(at: i64) -> Provenance {
    Provenance {
        by: ContributorId::from_raw(1),
        at: Timestamp(at),
    }
}

/// A small tree with the shapes that matter:
///
/// ```text
///   Пётр ═ Анна          (grandparents)
///        │
///      Иван ═ Мария      (parents)
///        ┌─┴─┐
///     Ольга  Николай     (children)
/// ```
struct Tree {
    archive: Archive,
    mint: Mint,
    petr: PersonId,
    anna: PersonId,
    ivan: PersonId,
    maria: PersonId,
    olga: PersonId,
    nikolai: PersonId,
    grandparents: FamilyId,
    parents: FamilyId,
}

impl Tree {
    fn new() -> Self {
        let mut mint = Mint(0);
        let mut archive = Archive::new();

        let mut person = |archive: &mut Archive, given: &str| {
            let id = mint.person();
            archive.add_person(Person::new(id)).unwrap();
            let fact = mint.fact();
            let assertion = mint.assertion(PersonName::russian(given, "", "Смирнов"));
            archive.assert_name(id, fact, assertion).unwrap();
            id
        };
        let petr = person(&mut archive, "Пётр");
        let anna = person(&mut archive, "Анна");
        let ivan = person(&mut archive, "Иван");
        let maria = person(&mut archive, "Мария");
        let olga = person(&mut archive, "Ольга");
        let nikolai = person(&mut archive, "Николай");

        let mut tree = Self {
            archive,
            mint,
            petr,
            anna,
            ivan,
            maria,
            olga,
            nikolai,
            grandparents: FamilyId::from_raw(0),
            parents: FamilyId::from_raw(0),
        };
        tree.grandparents = tree.union(petr, anna, &[ivan]);
        tree.parents = tree.union(ivan, maria, &[olga, nikolai]);
        tree
    }

    fn union(&mut self, husband: PersonId, wife: PersonId, children: &[PersonId]) -> FamilyId {
        let family = self.mint.family();
        self.archive.add_family(Family::new(family)).unwrap();
        self.partner(family, husband, Partnership::Husband).unwrap();
        self.partner(family, wife, Partnership::Wife).unwrap();
        for child in children {
            self.child(family, *child).unwrap();
        }
        family
    }

    fn partner(
        &mut self,
        family: FamilyId,
        person: PersonId,
        how: Partnership,
    ) -> Result<(), Refusal> {
        let fact = self.mint.fact();
        let assertion = self.mint.assertion(how);
        self.archive.assert_partner(family, person, fact, assertion)
    }

    fn child(&mut self, family: FamilyId, person: PersonId) -> Result<(), Refusal> {
        let fact = self.mint.fact();
        let assertion = self.mint.assertion(ChildRelation::Birth);
        self.archive.assert_child(family, person, fact, assertion)
    }
}

// ── relationships ─────────────────────────────────────────────────────────────────────────────

#[test]
fn relationships_read_both_ways() {
    let t = Tree::new();
    let a = &t.archive;

    assert_eq!(a.parents_of(t.olga), vec![t.ivan, t.maria]);
    assert_eq!(a.children_of(t.ivan), vec![t.olga, t.nikolai]);
    assert_eq!(a.partners_of(t.ivan), vec![t.maria]);
    assert_eq!(a.siblings_of(t.olga), vec![t.nikolai]);
    assert_eq!(a.parents_of(t.petr), Vec::<PersonId>::new());
}

#[test]
fn ancestry_reaches_every_generation_once() {
    let t = Tree::new();
    let ancestors = t.archive.ancestors_of(t.olga);
    assert_eq!(ancestors.len(), 4);
    for ancestor in [t.ivan, t.maria, t.petr, t.anna] {
        assert!(ancestors.contains(&ancestor));
    }
    assert!(t.archive.is_ancestor(t.petr, t.nikolai));
    assert!(!t.archive.is_ancestor(t.nikolai, t.petr));
    assert!(
        !t.archive.is_ancestor(t.maria, t.ivan),
        "a spouse is not an ancestor"
    );
}

#[test]
fn half_siblings_are_siblings_through_the_family_they_share() {
    let mut t = Tree::new();
    // Иван remarries after Мария; Сергей is Ольга's half-brother.
    let second_wife = t.mint.person();
    t.archive.add_person(Person::new(second_wife)).unwrap();
    let sergei = t.mint.person();
    t.archive.add_person(Person::new(sergei)).unwrap();
    t.union(t.ivan, second_wife, &[sergei]);

    assert_eq!(
        t.archive.children_of(t.ivan),
        vec![t.olga, t.nikolai, sergei]
    );
    assert_eq!(t.archive.siblings_of(t.olga), vec![t.nikolai]);
    assert_eq!(t.archive.partners_of(t.ivan), vec![t.maria, second_wife]);
}

#[test]
fn withdrawing_the_only_evidence_ends_a_relationship_and_keeps_its_history() {
    let mut t = Tree::new();
    let fact = t.mint.fact();
    let dmitri = t.mint.person();
    t.archive.add_person(Person::new(dmitri)).unwrap();

    let assertion = t.mint.assertion(ChildRelation::Birth);
    let assertion_id = assertion.id;
    t.archive
        .assert_child(t.parents, dmitri, fact, assertion)
        .unwrap();
    assert!(t.archive.children_of(t.ivan).contains(&dmitri));

    t.archive.retract(fact, assertion_id, made(1)).unwrap();
    assert!(!t.archive.children_of(t.ivan).contains(&dmitri));

    // Still there, as history.
    let family = t.archive.family(t.parents).unwrap();
    assert!(family.children.iter().any(|m| m.person == dmitri));
}

#[test]
fn a_removed_person_is_nobodys_parent() {
    let mut t = Tree::new();
    t.archive.remove_person(t.maria, made(1)).unwrap();
    assert_eq!(t.archive.parents_of(t.olga), vec![t.ivan]);
    assert_eq!(t.archive.population(), 5);
    assert!(
        t.archive.person(t.maria).unwrap().is_removed(),
        "the tombstone stays"
    );
    assert_eq!(
        t.archive.remove_person(t.maria, made(2)),
        Err(Refusal::Removed(Entity::Person(t.maria)))
    );
}

// ── refusals ──────────────────────────────────────────────────────────────────────────────────

#[test]
fn nobody_becomes_their_own_ancestor_by_hand() {
    let mut t = Tree::new();
    // Making Пётр a child of his grandchildren's family: he would be his own grandson's son.
    assert_eq!(
        t.child(t.parents, t.petr),
        Err(Refusal::Cycle {
            person: t.petr,
            family: t.parents
        })
    );
    // And making Ольга a partner in her grandparents' family: she would be her father's mother.
    let fact = t.mint.fact();
    let assertion = t.mint.assertion(Partnership::Partner);
    let grandparents_with_room = t.mint.family();
    t.archive
        .add_family(Family::new(grandparents_with_room))
        .unwrap();
    t.child(grandparents_with_room, t.ivan).unwrap();
    assert_eq!(
        t.archive
            .assert_partner(grandparents_with_room, t.olga, fact, assertion),
        Err(Refusal::Cycle {
            person: t.olga,
            family: grandparents_with_room
        })
    );
}

#[test]
fn a_partner_cannot_also_be_a_child_of_the_union() {
    let mut t = Tree::new();
    assert_eq!(
        t.child(t.parents, t.ivan),
        Err(Refusal::PartnerAndChild {
            person: t.ivan,
            family: t.parents
        })
    );
    assert_eq!(
        t.partner(t.parents, t.olga, Partnership::Partner),
        Err(Refusal::PartnerAndChild {
            person: t.olga,
            family: t.parents
        })
    );
}

#[test]
fn a_union_has_two_partners_and_restating_one_is_not_a_third() {
    let mut t = Tree::new();
    let stranger = t.mint.person();
    t.archive.add_person(Person::new(stranger)).unwrap();
    assert_eq!(
        t.partner(t.parents, stranger, Partnership::Partner),
        Err(Refusal::ThirdPartner { family: t.parents })
    );
    // A second source for Мария's membership is a second assertion, not a third partner.
    let existing = t
        .archive
        .family(t.parents)
        .unwrap()
        .partners
        .iter()
        .find(|m| m.person == t.maria)
        .unwrap()
        .fact
        .id;
    let assertion = t.mint.assertion(Partnership::Partner);
    t.archive
        .assert_partner(t.parents, t.maria, existing, assertion)
        .unwrap();
}

#[test]
fn an_import_is_not_refused_for_the_shape_of_its_tree() {
    // A cycle, built the way a GEDCOM import builds families: whole, in one call each. The rules
    // report it; the archive loads it.
    let mut archive = Archive::new();
    let mut mint = Mint(100);
    let a = mint.person();
    let b = mint.person();
    archive.add_person(Person::new(a)).unwrap();
    archive.add_person(Person::new(b)).unwrap();

    let mut membership = |person, relation: ChildRelation| {
        let mut fact = crate::model::Fact::new(mint.fact());
        fact.claims.assert(mint.assertion(relation)).unwrap();
        crate::model::Membership { person, fact }
    };
    let mut one = Family::new(FamilyId::from_raw(1));
    let mut partner_a = crate::model::Fact::new(FactId::from_raw(9001));
    partner_a
        .claims
        .assert(Assertion::new(
            AssertionId::from_raw(9101),
            Partnership::Partner,
            made(0),
        ))
        .unwrap();
    one.partners.push(crate::model::Membership {
        person: a,
        fact: partner_a,
    });
    one.children.push(membership(b, ChildRelation::Birth));

    let mut two = Family::new(FamilyId::from_raw(2));
    let mut partner_b = crate::model::Fact::new(FactId::from_raw(9002));
    partner_b
        .claims
        .assert(Assertion::new(
            AssertionId::from_raw(9102),
            Partnership::Partner,
            made(0),
        ))
        .unwrap();
    two.partners.push(crate::model::Membership {
        person: b,
        fact: partner_b,
    });
    two.children.push(membership(a, ChildRelation::Birth));

    archive.add_family(one).unwrap();
    archive.add_family(two).unwrap();
    assert!(archive.is_ancestor(a, b) && archive.is_ancestor(b, a));
    // And the walk terminates on it.
    assert_eq!(archive.ancestors_of(a).len(), 2);
}

#[test]
fn identities_and_references_are_checked() {
    let mut t = Tree::new();
    assert_eq!(
        t.archive.add_person(Person::new(t.olga)),
        Err(Refusal::Duplicate(Entity::Person(t.olga)))
    );

    let ghost = PersonId::from_raw(999_999);
    let mut family = Family::new(t.mint.family());
    let fact = crate::model::Fact::new(t.mint.fact());
    family.children.push(crate::model::Membership {
        person: ghost,
        fact,
    });
    assert_eq!(
        t.archive.add_family(family),
        Err(Refusal::Missing(Entity::Person(ghost)))
    );

    // A fact identity is one fact: reusing a child's membership fact for someone else would merge
    // two relationships into one history.
    let olga_fact = t
        .archive
        .family(t.parents)
        .unwrap()
        .children
        .iter()
        .find(|m| m.person == t.olga)
        .unwrap()
        .fact
        .id;
    let assertion = t.mint.assertion(ChildRelation::Birth);
    assert_eq!(
        t.archive
            .assert_child(t.parents, t.nikolai, olga_fact, assertion),
        Err(Refusal::WrongFact(olga_fact))
    );
}

// ── events and claims ─────────────────────────────────────────────────────────────────────────

#[test]
fn an_event_reads_its_roles_and_its_date_through_claims() {
    let mut t = Tree::new();
    let baptism = EventId::from_raw(t.mint.next());
    t.archive
        .add_event(Event::new(baptism, EventKind::Baptism))
        .unwrap();

    for (person, role) in [
        (t.olga, Role::Principal),
        (t.ivan, Role::Father),
        (t.anna, Role::Godparent),
    ] {
        let fact = t.mint.fact();
        let assertion = t.mint.assertion(role);
        t.archive
            .assert_participation(baptism, person, fact, assertion)
            .unwrap();
    }

    let date = t.mint.fact();
    let register = t
        .mint
        .assertion(RecordedDate::parse_gedcom7("JULIAN 17 APR 1871", None).unwrap());
    t.archive
        .assert_event_date(baptism, date, register)
        .unwrap();

    let event = t.archive.event(baptism).unwrap();
    assert_eq!(
        event.in_role(Role::Godparent).collect::<Vec<_>>(),
        vec![t.anna]
    );
    assert!(event.resolved_date().is_some());
    assert_eq!(t.archive.events_of(t.anna).count(), 1);
}

#[test]
fn a_contested_date_can_be_concluded_and_the_conclusion_withdrawn() {
    let mut t = Tree::new();
    let birth = EventId::from_raw(t.mint.next());
    t.archive
        .add_event(Event::new(birth, EventKind::Birth))
        .unwrap();

    let date = t.mint.fact();
    let register = t
        .mint
        .assertion(RecordedDate::parse_gedcom7("1871", None).unwrap());
    let register_id = register.id;
    t.archive.assert_event_date(birth, date, register).unwrap();
    let census = t
        .mint
        .assertion(RecordedDate::parse_gedcom7("1880", None).unwrap());
    t.archive.assert_event_date(birth, date, census).unwrap();

    let contested = |archive: &Archive| {
        archive
            .event(birth)
            .unwrap()
            .date
            .as_ref()
            .unwrap()
            .resolve()
            .is_open_question()
    };
    assert!(contested(&t.archive));

    t.archive
        .conclude_event_date(
            date,
            Conclusion {
                id: ConclusionId::from_raw(t.mint.next()),
                value: RecordedDate::parse_gedcom7("1871", None).unwrap(),
                supported_by: vec![register_id],
                contradicted_by: vec![],
                reasoning: "Recorded at the time.".to_owned(),
                made: made(5),
                withdrawn: None,
            },
        )
        .unwrap();
    assert!(matches!(
        t.archive
            .event(birth)
            .unwrap()
            .date
            .as_ref()
            .unwrap()
            .resolve(),
        Resolution::Concluded {
            unconsidered: 1,
            ..
        }
    ));

    t.archive.withdraw_conclusion(date, made(6)).unwrap();
    assert!(contested(&t.archive));
    assert_eq!(
        t.archive.withdraw_conclusion(date, made(7)),
        Err(Refusal::Claim(ClaimError::NoConclusion))
    );
}

#[test]
fn a_fact_is_found_by_its_identity_alone() {
    let mut t = Tree::new();
    let olga = t.archive.person(t.olga).unwrap();
    let name = olga.names.first().unwrap();
    let (fact, assertion) = (name.id, name.claims.assertions()[0].id);

    t.archive.retract(fact, assertion, made(1)).unwrap();
    assert!(t.archive.person(t.olga).unwrap().display_name().is_none());

    let nowhere = FactId::from_raw(424_242);
    assert_eq!(
        t.archive.retract(nowhere, assertion, made(2)),
        Err(Refusal::Missing(Entity::Fact(nowhere)))
    );
}
