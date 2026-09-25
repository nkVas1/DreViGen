//! The audit, from both directions: a sound tree produces no errors, and each fault put into one
//! is found — while approximations that only *might* be wrong produce nothing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use drevigen_testkit::{SyntheticTree, TreeSpec};

use super::{Finding, Rule, Severity, audit};
use crate::fixtures::{Scene, from_synthetic};
use crate::model::{EventKind, Role};
use crate::value::Sex;

fn rules(findings: &[Finding]) -> Vec<Rule> {
    findings.iter().map(|finding| finding.rule).collect()
}

// ── from the sound side ─────────────────────────────────────────────────────────────────────────

/// A generated lineage of three thousand people, built through the checked edits, audits clean.
///
/// Two things at once: the refusals never fire on a sound tree (every union and child went
/// through `assert_partner` and `assert_child`), and the audit does not report errors where there
/// are none. A rule that fires on this tree is a rule that will fire on every real one.
#[test]
fn a_sound_generated_tree_has_no_errors() {
    for seed in [7, 1871, 1918] {
        let tree = SyntheticTree::generate(TreeSpec::sized_for(3_000, seed));
        let scene = from_synthetic(&tree);
        let findings = audit(&scene.archive);
        let errors: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.severity() == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "seed {seed}: {} errors in a sound tree, first {:?}",
            errors.len(),
            errors.first()
        );
    }
}

// ── from the broken side ────────────────────────────────────────────────────────────────────────

#[test]
fn a_child_born_before_a_parent_is_named_once() {
    let mut s = Scene::new();
    let father = s.person("Иван", Sex::Male, Some("1880"), None);
    let mother = s.person("Мария", Sex::Female, Some("1850"), None);
    let child = s.person("Ольга", Sex::Female, Some("1871"), None);
    s.union(Some(father), Some(mother), &[child]);

    let findings = audit(&s.archive);
    assert_eq!(rules(&findings), vec![Rule::BornBeforeParent]);
    assert_eq!(findings[0].people, vec![child, father], "the child first");
}

#[test]
fn an_approximate_parent_is_given_the_benefit_of_the_doubt() {
    // "About 1858" could be 1856: a mother of fifteen. Not certain, so not reported.
    let mut s = Scene::new();
    let mother = s.person("Мария", Sex::Female, Some("ABT 1858"), None);
    let child = s.person("Ольга", Sex::Female, Some("1871"), None);
    s.union(None, Some(mother), &[child]);
    assert!(audit(&s.archive).is_empty());

    // An exact 1860 leaves no doubt: eleven or twelve, however the two years are read.
    let mut s = Scene::new();
    let mother = s.person("Мария", Sex::Female, Some("1860"), None);
    let child = s.person("Ольга", Sex::Female, Some("1872"), None);
    s.union(None, Some(mother), &[child]);
    assert_eq!(rules(&audit(&s.archive)), vec![Rule::ParentTooYoung]);
}

#[test]
fn a_posthumous_child_is_ordinary_and_a_late_one_is_not() {
    let mut s = Scene::new();
    let father = s.person("Иван", Sex::Male, Some("1840"), Some("1 MAR 1871"));
    let mother = s.person("Мария", Sex::Female, Some("1845"), None);
    let posthumous = s.person("Ольга", Sex::Female, Some("20 NOV 1871"), None);
    s.union(Some(father), Some(mother), &[posthumous]);
    assert!(
        audit(&s.archive).is_empty(),
        "within ten months of the death"
    );

    let mut s = Scene::new();
    let father = s.person("Иван", Sex::Male, Some("1840"), Some("1871"));
    let mother = s.person("Мария", Sex::Female, Some("1845"), None);
    let late = s.person("Ольга", Sex::Female, Some("1874"), None);
    s.union(Some(father), Some(mother), &[late]);
    assert_eq!(rules(&audit(&s.archive)), vec![Rule::BornAfterFathersDeath]);
}

#[test]
fn a_mother_cannot_give_birth_after_her_death() {
    let mut s = Scene::new();
    let mother = s.person("Мария", Sex::Female, Some("1845"), Some("1870"));
    let child = s.person("Ольга", Sex::Female, Some("1872"), None);
    s.union(None, Some(mother), &[child]);
    assert_eq!(rules(&audit(&s.archive)), vec![Rule::BornAfterMothersDeath]);
}

#[test]
fn a_mother_of_sixty_is_worth_a_second_look() {
    let mut s = Scene::new();
    let mother = s.person("Мария", Sex::Female, Some("1810"), None);
    let child = s.person("Ольга", Sex::Female, Some("1871"), None);
    s.union(None, Some(mother), &[child]);
    let findings = audit(&s.archive);
    assert_eq!(rules(&findings), vec![Rule::MotherTooOld]);
    assert_eq!(findings[0].severity(), Severity::Warning);
}

#[test]
fn a_life_is_born_then_baptised_then_dies_then_is_buried() {
    let mut s = Scene::new();
    let backwards = s.person("Иван", Sex::Male, Some("1880"), Some("1871"));
    let early_baptism = s.person("Пётр", Sex::Male, Some("10 APR 1871"), None);
    s.event(
        EventKind::Baptism,
        Some("2 APR 1871"),
        &[(early_baptism, Role::Principal)],
    );
    let early_burial = s.person("Анна", Sex::Female, Some("1850"), Some("10 MAR 1900"));
    s.event(
        EventKind::Burial,
        Some("1 MAR 1900"),
        &[(early_burial, Role::Principal)],
    );

    let found = rules(&audit(&s.archive));
    for rule in [
        Rule::DiedBeforeBorn,
        Rule::BaptisedBeforeBorn,
        Rule::BuriedBeforeDied,
    ] {
        assert!(found.contains(&rule), "{rule:?} missing from {found:?}");
    }
    let _ = backwards;
}

#[test]
fn a_baptism_bounds_a_birth_that_was_never_recorded() {
    // No birth event, only a baptism in 1871 — so the person was born by 1871, and a parent
    // born in 1880 cannot be theirs.
    let mut s = Scene::new();
    let father = s.person("Иван", Sex::Male, Some("1880"), None);
    let child = s.person("Ольга", Sex::Female, None, None);
    s.event(
        EventKind::Baptism,
        Some("1871"),
        &[(child, Role::Principal)],
    );
    s.union(Some(father), None, &[child]);
    assert_eq!(rules(&audit(&s.archive)), vec![Rule::BornBeforeParent]);
}

#[test]
fn a_witness_must_be_alive_and_a_named_parent_need_not_be() {
    let mut s = Scene::new();
    let dead_father = s.person("Иван", Sex::Male, Some("1820"), Some("1860"));
    let dead_witness = s.person("Пётр", Sex::Male, Some("1820"), Some("1860"));
    let groom = s.person("Николай", Sex::Male, Some("1848"), None);
    let bride = s.person("Анна", Sex::Female, Some("1850"), None);
    let wedding = s.event(
        EventKind::Marriage,
        Some("1871"),
        &[
            (groom, Role::Husband),
            (bride, Role::Wife),
            // Marriage records name the couple's parents whether or not they are living.
            (dead_father, Role::Father),
            (dead_witness, Role::Witness),
        ],
    );

    let findings = audit(&s.archive);
    assert_eq!(rules(&findings), vec![Rule::ActiveAfterDeath]);
    assert_eq!(findings[0].people, vec![dead_witness]);
    assert_eq!(findings[0].event, Some(wedding));
}

#[test]
fn a_child_bride_is_worth_a_second_look() {
    let mut s = Scene::new();
    let bride = s.person("Анна", Sex::Female, Some("1860"), None);
    s.event(EventKind::Marriage, Some("1871"), &[(bride, Role::Wife)]);
    assert_eq!(rules(&audit(&s.archive)), vec![Rule::MarriedTooYoung]);
}

#[test]
fn a_life_of_a_hundred_and_twenty_years_is_more_often_two_people() {
    let mut s = Scene::new();
    s.person("Иван", Sex::Male, Some("1750"), Some("1871"));
    assert_eq!(rules(&audit(&s.archive)), vec![Rule::ImplausibleLifespan]);
}

#[test]
fn a_loop_of_descent_is_one_finding_naming_everyone_in_it() {
    // Loaded the way an import loads it — whole families, no refusals — and then found.
    use crate::claim::{Assertion, Provenance, Timestamp};
    use crate::id::{AssertionId, ContributorId, FactId, FamilyId};
    use crate::model::{ChildRelation, Fact, Family, Membership, Partnership};

    let mut s = Scene::new();
    let a = s.person("Иван", Sex::Male, None, None);
    let b = s.person("Пётр", Sex::Male, None, None);
    let c = s.person("Николай", Sex::Male, None, None);
    let made = Provenance {
        by: ContributorId::from_raw(1),
        at: Timestamp(0),
    };
    let mut next = 10_000;
    let mut fact = |value_is_child: bool| {
        next += 2;
        let id = FactId::from_raw(next);
        let assertion = AssertionId::from_raw(next + 1);
        (id, assertion, value_is_child)
    };

    // a → b → c → a
    for (index, (parent, child)) in [(a, b), (b, c), (c, a)].into_iter().enumerate() {
        let mut family = Family::new(FamilyId::from_raw(20_000 + index as u128));
        let (id, assertion, _) = fact(false);
        let mut partner = Fact::new(id);
        partner
            .claims
            .assert(Assertion::new(assertion, Partnership::Husband, made))
            .unwrap();
        family.partners.push(Membership {
            person: parent,
            fact: partner,
        });
        let (id, assertion, _) = fact(true);
        let mut membership = Fact::new(id);
        membership
            .claims
            .assert(Assertion::new(assertion, ChildRelation::Birth, made))
            .unwrap();
        family.children.push(Membership {
            person: child,
            fact: membership,
        });
        s.archive.add_family(family).unwrap();
    }

    let findings = audit(&s.archive);
    assert_eq!(rules(&findings), vec![Rule::OwnAncestor]);
    let mut expected = vec![a, b, c];
    expected.sort_unstable();
    assert_eq!(findings[0].people, expected);
}

#[test]
fn errors_come_before_warnings() {
    let mut s = Scene::new();
    s.person("Иван", Sex::Male, Some("1750"), Some("1871"));
    s.person("Пётр", Sex::Male, Some("1880"), Some("1871"));
    let findings = audit(&s.archive);
    assert_eq!(
        findings.first().map(Finding::severity),
        Some(Severity::Error)
    );
    assert_eq!(
        findings.last().map(Finding::severity),
        Some(Severity::Warning)
    );
}
