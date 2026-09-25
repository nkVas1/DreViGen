//! Deterministic synthetic genealogy generation.
//!
//! The generator exists because the hard questions in this project — does the layout engine hold
//! 50 000 people at 60 fps, does the CRDT merge a hundred thousand operations in budget, does
//! record linkage find planted duplicates — can only be answered against trees of realistic
//! *shape*. A random graph is not a genealogy, and benchmarking against one would measure the
//! wrong thing.
//!
//! What "realistic shape" means here:
//!
//! - **Generational layering.** Everyone belongs to a generation, roughly 28 years apart.
//! - **Pedigree collapse.** Cousins marry, so an ancestor is reachable by more than one path and
//!   the graph is a DAG with diamonds rather than a tree. This is the single most important
//!   property: a generator that emits a clean tree would let a broken layout engine pass.
//! - **Married-in strangers.** Most spouses arrive from outside with no recorded parents, which
//!   is what real trees look like at their edges.
//! - **Remarriage.** A widowed partner forms a second family, so people belong to several.
//! - **Historical mortality.** High infant mortality early on, declining later, and a living
//!   final generation with no death date.
//!
//! Everything is seeded: the same [`TreeSpec`] yields a byte-identical tree on every platform,
//! so a benchmark from today is comparable with one from next year.

use crate::names::{
    FEMALE_GIVEN, MALE_GIVEN, MaleGiven, PLACES, SURNAMES, Sex, patronymic_for, surname_for,
};
use crate::rng::Rng;

/// Identifies a person within one [`SyntheticTree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersonId(pub u32);

/// Identifies a family within one [`SyntheticTree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FamilyId(pub u32);

/// A generated person.
#[derive(Debug, Clone)]
pub struct Person {
    /// Stable identifier, equal to the index in [`SyntheticTree::people`].
    pub id: PersonId,
    /// Given name in the nominative.
    pub given: String,
    /// Patronymic, absent for people who married in from outside the recorded tree.
    pub patronymic: Option<String>,
    /// Surname, already inflected for [`Person::sex`].
    pub surname: String,
    /// Recorded sex.
    pub sex: Sex,
    /// Year of birth.
    pub birth_year: i32,
    /// Year of death; `None` means still living.
    pub death_year: Option<i32>,
    /// Settlement of birth.
    pub birth_place: &'static str,
    /// Zero-based generation, counted from the founding couples.
    pub generation: u16,
    /// The family this person was born into, if their parents are in the tree.
    pub child_of: Option<FamilyId>,
    /// Families this person formed as a partner; more than one means remarriage.
    pub spouse_in: Vec<FamilyId>,
}

impl Person {
    /// Returns the full name in the Russian order: surname, given, patronymic.
    #[must_use]
    pub fn full_name(&self) -> String {
        match &self.patronymic {
            Some(p) => format!("{} {} {}", self.surname, self.given, p),
            None => format!("{} {}", self.surname, self.given),
        }
    }

    /// Returns `true` when no death year is recorded.
    #[must_use]
    pub fn is_living(&self) -> bool {
        self.death_year.is_none()
    }
}

/// A union of up to two partners and their children.
#[derive(Debug, Clone)]
pub struct Family {
    /// Stable identifier, equal to the index in [`SyntheticTree::families`].
    pub id: FamilyId,
    /// The male partner.
    pub husband: Option<PersonId>,
    /// The female partner.
    pub wife: Option<PersonId>,
    /// Children born to this union, in birth order.
    pub children: Vec<PersonId>,
    /// Year of marriage.
    pub marriage_year: i32,
}

/// Parameters controlling generation.
///
/// [`TreeSpec::default`] produces a tree that resembles a Russian provincial lineage traced from
/// the late 18th century to the present.
#[derive(Debug, Clone)]
pub struct TreeSpec {
    /// Seed. The same seed and parameters always produce the same tree.
    pub seed: u64,
    /// Number of unrelated couples in the founding generation.
    pub founding_couples: usize,
    /// Maximum number of generations to produce.
    pub generations: u16,
    /// Stop once this many people exist. `None` means run to `generations`.
    pub target_people: Option<usize>,
    /// Birth year of the founding generation.
    pub first_birth_year: i32,
    /// Years between one generation's births and the next.
    pub generation_span: i32,
    /// Probability that a marriage is contracted inside the tree, creating pedigree collapse.
    pub cousin_marriage_rate: f64,
    /// Probability that a surviving adult marries at all.
    pub marriage_rate: f64,
    /// Probability that a partner outlived by their spouse forms a second family.
    pub remarriage_rate: f64,
    /// Year from which nobody is recorded as dead, so the tree ends in living people.
    pub living_from_year: i32,
}

impl Default for TreeSpec {
    fn default() -> Self {
        Self {
            seed: 0x_D2E7_1BE4_5A90_1771,
            founding_couples: 4,
            generations: 9,
            target_people: None,
            first_birth_year: 1770,
            generation_span: 28,
            cousin_marriage_rate: 0.06,
            marriage_rate: 0.82,
            remarriage_rate: 0.12,
            living_from_year: 1945,
        }
    }
}

impl TreeSpec {
    /// Returns a spec tuned to reach approximately `people` individuals.
    ///
    /// Generation count is fixed and the founding population is scaled, which keeps the tree
    /// wide rather than implausibly deep — a 200 000-person tree spanning forty generations
    /// would not resemble anything a user will ever hold.
    #[must_use]
    pub fn sized_for(people: usize, seed: u64) -> Self {
        // With the default fertility and marriage rates a single founding couple yields roughly
        // 900 people over nine generations, so scale the founding population from there. The
        // floor of four is insurance against extinction: fertility is drawn from a distribution,
        // and a lone founding couple that happens to draw no children ends the whole tree at
        // generation one. Real lines do die out; a data generator should not.
        const PER_COUPLE: usize = 900;
        const MIN_COUPLES: usize = 4;
        let couples = people.div_ceil(PER_COUPLE).max(MIN_COUPLES);
        Self {
            seed,
            founding_couples: couples,
            target_people: Some(people),
            ..Self::default()
        }
    }
}

/// A generated genealogy.
#[derive(Debug, Clone)]
pub struct SyntheticTree {
    /// All people, indexed by [`PersonId`].
    pub people: Vec<Person>,
    /// All families, indexed by [`FamilyId`].
    pub families: Vec<Family>,
    /// The parameters that produced this tree.
    pub spec: TreeSpec,
}

impl SyntheticTree {
    /// Generates a tree from `spec`.
    #[must_use]
    pub fn generate(spec: TreeSpec) -> Self {
        Generator::new(spec).run()
    }

    /// Looks up a person.
    #[must_use]
    pub fn person(&self, id: PersonId) -> &Person {
        &self.people[id.0 as usize]
    }

    /// Looks up a family.
    #[must_use]
    pub fn family(&self, id: FamilyId) -> &Family {
        &self.families[id.0 as usize]
    }

    /// Computes summary statistics.
    #[must_use]
    pub fn stats(&self) -> TreeStats {
        let mut living = 0;
        let mut married_in = 0;
        let mut remarried = 0;
        let mut max_generation = 0;
        for p in &self.people {
            if p.is_living() {
                living += 1;
            }
            if p.child_of.is_none() {
                married_in += 1;
            }
            if p.spouse_in.len() > 1 {
                remarried += 1;
            }
            max_generation = max_generation.max(p.generation);
        }

        // Pedigree collapse: a family whose two partners both descend from inside the tree is a
        // marriage contracted within the recorded lineage, and every such union closes a diamond
        // in the ancestor graph.
        let endogamous = self
            .families
            .iter()
            .filter(|f| {
                let husband_known = f
                    .husband
                    .is_some_and(|id| self.person(id).child_of.is_some());
                let wife_known = f.wife.is_some_and(|id| self.person(id).child_of.is_some());
                husband_known && wife_known
            })
            .count();

        let children: usize = self.families.iter().map(|f| f.children.len()).sum();
        let largest_family = self
            .families
            .iter()
            .map(|f| f.children.len())
            .max()
            .unwrap_or(0);

        TreeStats {
            people: self.people.len(),
            families: self.families.len(),
            generations: usize::from(max_generation) + 1,
            living,
            married_in,
            remarried,
            endogamous_unions: endogamous,
            mean_children: if self.families.is_empty() {
                0.0
            } else {
                children as f64 / self.families.len() as f64
            },
            largest_family,
        }
    }
}

/// Summary statistics for a generated tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeStats {
    /// Total people.
    pub people: usize,
    /// Total families.
    pub families: usize,
    /// Number of generations present.
    pub generations: usize,
    /// People with no recorded death.
    pub living: usize,
    /// People who married in with no parents in the tree.
    pub married_in: usize,
    /// People belonging to more than one family as a partner.
    pub remarried: usize,
    /// Unions where both partners descend from inside the tree — the pedigree-collapse count.
    pub endogamous_unions: usize,
    /// Mean children per family.
    pub mean_children: f64,
    /// Children in the largest family.
    pub largest_family: usize,
}

// ── generation ──────────────────────────────────────────────────────────────

struct Generator {
    spec: TreeSpec,
    rng: Rng,
    people: Vec<Person>,
    families: Vec<Family>,
    /// Masculine given name of each person's father, needed to build a grandchild's patronymic.
    male_given_of: Vec<Option<MaleGiven>>,
    /// The masculine surname of each person's line, kept separately because the person's own
    /// surname is already inflected and cannot be inflected back reliably.
    line_surname: Vec<&'static str>,
}

impl Generator {
    fn new(spec: TreeSpec) -> Self {
        let rng = Rng::new(spec.seed);
        Self {
            spec,
            rng,
            people: Vec::new(),
            families: Vec::new(),
            male_given_of: Vec::new(),
            line_surname: Vec::new(),
        }
    }

    fn full(&self) -> bool {
        self.spec
            .target_people
            .is_some_and(|t| self.people.len() >= t)
    }

    fn run(mut self) -> SyntheticTree {
        let mut current: Vec<FamilyId> = Vec::new();

        for i in 0..self.spec.founding_couples {
            if self.full() {
                break;
            }
            let surname = SURNAMES[i % SURNAMES.len()];
            let year = self.spec.first_birth_year;
            let family = self.found_couple(surname, year);
            current.push(family);
        }

        for generation in 0..self.spec.generations {
            if current.is_empty() || self.full() {
                break;
            }
            current = self.advance(&current, generation);
        }

        SyntheticTree {
            people: self.people,
            families: self.families,
            spec: self.spec,
        }
    }

    /// Creates a founding couple: two unrelated people with no parents, and their union.
    fn found_couple(&mut self, surname: &'static str, birth_year: i32) -> FamilyId {
        let husband = self.new_outsider(Sex::Male, 0, birth_year, Some(surname));
        let wife_born = birth_year + self.rng.range(-3, 3);
        let wife = self.new_outsider(Sex::Female, 0, wife_born, None);
        let marriage_year = birth_year + self.rng.range(20, 27);
        self.new_family(Some(husband), Some(wife), marriage_year)
    }

    /// Advances one generation: births, then marriages.
    ///
    /// Returns the families the new generation formed, which become the input to the next call.
    fn advance(&mut self, families: &[FamilyId], generation: u16) -> Vec<FamilyId> {
        let child_generation = generation + 1;
        let birth_year =
            self.spec.first_birth_year + i32::from(child_generation) * self.spec.generation_span;

        let mut cohort = self.bear_cohort(families, child_generation, birth_year);

        // Shuffle so that partner selection does not systematically pair neighbours, which
        // would make the resulting layout suspiciously tidy.
        self.rng.shuffle(&mut cohort);

        self.marry_cohort(&cohort)
    }

    /// Births the children of `families` into `child_generation`.
    fn bear_cohort(
        &mut self,
        families: &[FamilyId],
        child_generation: u16,
        birth_year: i32,
    ) -> Vec<PersonId> {
        let mut cohort: Vec<PersonId> = Vec::new();
        for &fid in families {
            if self.full() {
                break;
            }
            let (first_birth, last_birth) = self.birth_window(fid, birth_year);
            let count = i32::try_from(self.children_count(birth_year)).unwrap_or(i32::MAX);
            for i in 0..count {
                let year = first_birth + i * 2;
                if self.full() || last_birth.is_some_and(|last| year > last) {
                    break;
                }
                let child = self.new_child(fid, child_generation, year);
                cohort.push(child);
            }
        }
        cohort
    }

    /// Pairs off the members of `cohort` who survive to marriageable age and marry.
    fn marry_cohort(&mut self, cohort: &[PersonId]) -> Vec<FamilyId> {
        // Marrying people off creates new people — spouses who marry in from outside, and
        // second partners after a remarriage — so this set has to grow as we go. Sizing it once
        // from `people.len()` would index out of bounds the moment an outsider is created.
        let mut partnered = PartneredSet::default();
        let mut next: Vec<FamilyId> = Vec::new();

        for idx in 0..cohort.len() {
            let pid = cohort[idx];
            if partnered.contains(pid) {
                continue;
            }
            let (sex, generation, birth_year, death_year) = {
                let person = &self.people[pid.0 as usize];
                (
                    person.sex,
                    person.generation,
                    person.birth_year,
                    person.death_year,
                )
            };

            // The dead do not marry, and neither does everyone who lives.
            let marriage_year = birth_year + self.marriage_age(sex);
            if death_year.is_some_and(|d| d <= marriage_year) {
                continue;
            }
            if !self.rng.chance(self.spec.marriage_rate) {
                continue;
            }
            if self.full() {
                break;
            }

            let suitor = Suitor {
                id: pid,
                index: idx,
                sex,
                generation,
                birth_year,
            };
            let partner = self.choose_partner(cohort, suitor, &partnered);
            partnered.insert(pid);
            partnered.insert(partner);

            let (husband, wife) = if sex == Sex::Male {
                (pid, partner)
            } else {
                (partner, pid)
            };
            next.push(self.new_family(Some(husband), Some(wife), marriage_year));

            if let Some(second) =
                self.maybe_remarry(husband, wife, generation, birth_year, marriage_year)
            {
                next.push(second);
            }
        }

        next
    }

    /// Chooses a spouse: occasionally another member of the cohort, otherwise a stranger.
    ///
    /// A marriage inside the tree closes a diamond in the ancestor graph. Everyone in a cohort
    /// is at least a cousin of everyone else, so any in-cohort partner from a different family
    /// produces genuine pedigree collapse.
    fn choose_partner(
        &mut self,
        cohort: &[PersonId],
        suitor: Suitor,
        partnered: &PartneredSet,
    ) -> PersonId {
        let inside = if self.rng.chance(self.spec.cousin_marriage_rate) {
            self.find_cohort_partner(cohort, suitor, partnered)
        } else {
            None
        };

        if let Some(other) = inside {
            return other;
        }

        let surname = if suitor.sex == Sex::Male {
            None
        } else {
            Some(*self.rng.pick(SURNAMES).unwrap_or(&SURNAMES[0]))
        };
        let partner_born = suitor.birth_year + self.rng.range(-4, 4);
        self.new_outsider(
            sex_opposite(suitor.sex),
            suitor.generation,
            partner_born,
            surname,
        )
    }

    /// Occasionally forms a second family for one of the partners.
    ///
    /// A widowed partner who remarries belongs to two families, which is exactly the case a
    /// layout engine assuming one union per person gets wrong.
    fn maybe_remarry(
        &mut self,
        husband: PersonId,
        wife: PersonId,
        generation: u16,
        birth_year: i32,
        marriage_year: i32,
    ) -> Option<FamilyId> {
        if !self.rng.chance(self.spec.remarriage_rate) || self.full() {
            return None;
        }

        let survivor_sex = if self.rng.chance(0.5) {
            Sex::Male
        } else {
            Sex::Female
        };
        let survivor = if survivor_sex == Sex::Male {
            husband
        } else {
            wife
        };
        let second_surname = if survivor_sex == Sex::Male {
            None
        } else {
            Some(*self.rng.pick(SURNAMES).unwrap_or(&SURNAMES[0]))
        };
        let partner_born = birth_year + self.rng.range(-6, 6);
        let new_partner = self.new_outsider(
            sex_opposite(survivor_sex),
            generation,
            partner_born,
            second_surname,
        );
        let (h2, w2) = if survivor_sex == Sex::Male {
            (survivor, new_partner)
        } else {
            (new_partner, survivor)
        };
        let second_marriage = marriage_year + self.rng.range(6, 16);
        Some(self.new_family(Some(h2), Some(w2), second_marriage))
    }

    /// Finds an unpartnered cohort member of the opposite sex who is not a sibling.
    fn find_cohort_partner(
        &self,
        cohort: &[PersonId],
        suitor: Suitor,
        partnered: &PartneredSet,
    ) -> Option<PersonId> {
        let want = sex_opposite(suitor.sex);
        let subject_family = self.people[suitor.id.0 as usize].child_of;

        cohort
            .iter()
            .skip(suitor.index + 1)
            .copied()
            .find(|&candidate| {
                let c = &self.people[candidate.0 as usize];
                c.sex == want
                    && !partnered.contains(candidate)
                    // Not a sibling: different parental family.
                    && c.child_of != subject_family
                    && c.child_of.is_some()
            })
    }

    /// Draws a plausible number of children for a marriage contracted around `year`.
    ///
    /// Russian peasant families of the early 19th century recorded six or more births; the
    /// figure falls through the 20th century. Modelling the decline matters because it shapes
    /// the tree: a constant rate produces an implausibly uniform pyramid.
    /// The years a family's children can be born in, as `(first, last)`.
    ///
    /// Births used to be scheduled from the generation's year alone, which knew nothing of the
    /// parents: a partner born later than the rest of their generation could have children older
    /// than themselves, and a family went on having children after a parent died. The domain
    /// model's audit found both on its first run over these trees. The window closes both:
    ///
    /// - **First**: no earlier than the generation's year, nor before the younger parent turns
    ///   sixteen.
    /// - **Last**: the mother's death, the year after the father's — a child can be born within
    ///   ten months of its father's death — and the mother's forty-fifth year, whichever comes
    ///   first. `None` when nothing closes it.
    fn birth_window(&self, family: FamilyId, generation_year: i32) -> (i32, Option<i32>) {
        let union = &self.families[family.0 as usize];
        let person = |id: Option<PersonId>| id.map(|p| &self.people[p.0 as usize]);
        let (father, mother) = (person(union.husband), person(union.wife));

        let youngest_parent = father
            .iter()
            .chain(mother.iter())
            .map(|parent| parent.birth_year)
            .max();
        let first = youngest_parent.map_or(generation_year, |born| generation_year.max(born + 16));

        let limits = [
            mother.and_then(|m| m.death_year),
            father.and_then(|f| f.death_year).map(|year| year + 1),
            mother.map(|m| m.birth_year + 45),
        ];
        let last = limits.into_iter().flatten().min();
        (first, last)
    }

    fn children_count(&mut self, year: i32) -> usize {
        let mean = if year < 1870 {
            6.2
        } else if year < 1920 {
            5.0
        } else if year < 1960 {
            3.0
        } else {
            1.9
        };
        self.rng.normal(mean, mean * 0.45, 0.0, mean * 2.6).round() as usize
    }

    /// Typical age at first marriage.
    fn marriage_age(&mut self, sex: Sex) -> i32 {
        match sex {
            Sex::Male => self.rng.normal(24.0, 3.5, 17.0, 45.0) as i32,
            Sex::Female => self.rng.normal(20.5, 3.0, 16.0, 40.0) as i32,
        }
    }

    /// Draws a death year, or `None` for someone still living.
    ///
    /// Infant mortality was brutal and falling: roughly a quarter of children died before five
    /// in the early 19th century. A generator that ignores this produces trees in which every
    /// birth becomes an adult, and understates how much of a real register is short lives.
    ///
    /// `exposed_to_infancy` is `false` for founders and for spouses who married in: we meet
    /// those people as adults, so drawing them a death in infancy would produce a founder who
    /// died in the year of their own birth and still had eight children.
    fn death_year(&mut self, birth_year: i32, exposed_to_infancy: bool) -> Option<i32> {
        let infant_mortality = if !exposed_to_infancy {
            0.0
        } else if birth_year < 1870 {
            0.26
        } else if birth_year < 1920 {
            0.19
        } else if birth_year < 1950 {
            0.08
        } else {
            0.01
        };

        if infant_mortality > 0.0 && self.rng.chance(infant_mortality) {
            return Some(birth_year + self.rng.range(0, 5));
        }

        let mean_age = if birth_year < 1900 { 58.0 } else { 71.0 };
        let floor = if exposed_to_infancy { 6.0 } else { 18.0 };
        let age = self.rng.normal(mean_age, 16.0, floor, 101.0) as i32;
        let death = birth_year + age;

        // Anyone whose death would fall after the cutoff is simply still alive.
        if death >= self.spec.living_from_year && birth_year >= self.spec.living_from_year - 100 {
            None
        } else {
            Some(death)
        }
    }

    /// Creates a person with no parents in the tree — a founder or someone who married in.
    fn new_outsider(
        &mut self,
        sex: Sex,
        generation: u16,
        birth_year: i32,
        surname: Option<&'static str>,
    ) -> PersonId {
        let line = surname.unwrap_or_else(|| *self.rng.pick(SURNAMES).unwrap_or(&SURNAMES[0]));
        let (given, male_given) = self.draw_given(sex);
        let death = self.death_year(birth_year, false);
        let place = *self.rng.pick(PLACES).unwrap_or(&PLACES[0]);

        self.push_person(
            Person {
                id: PersonId(0), // replaced by push_person
                given,
                patronymic: None,
                surname: surname_for(line, sex),
                sex,
                birth_year,
                death_year: death,
                birth_place: place,
                generation,
                child_of: None,
                spouse_in: Vec::new(),
            },
            male_given,
            line,
        )
    }

    /// Creates a child of an existing family, inheriting line surname and patronymic.
    fn new_child(&mut self, family: FamilyId, generation: u16, birth_year: i32) -> PersonId {
        let sex = if self.rng.chance(0.512) {
            Sex::Male
        } else {
            Sex::Female
        };
        let (given, male_given) = self.draw_given(sex);

        let father = self.families[family.0 as usize].husband;
        let patronymic = father
            .and_then(|f| self.male_given_of[f.0 as usize])
            .map(|f| patronymic_for(f, sex).to_owned());
        let line = father.map_or_else(
            || *SURNAMES.first().unwrap_or(&"Неизвестный"),
            |f| self.line_surname[f.0 as usize],
        );

        let death = self.death_year(birth_year, true);
        let place = *self.rng.pick(PLACES).unwrap_or(&PLACES[0]);

        let id = self.push_person(
            Person {
                id: PersonId(0),
                given,
                patronymic,
                surname: surname_for(line, sex),
                sex,
                birth_year,
                death_year: death,
                birth_place: place,
                generation,
                child_of: Some(family),
                spouse_in: Vec::new(),
            },
            male_given,
            line,
        );

        self.families[family.0 as usize].children.push(id);
        id
    }

    /// Draws a given name, returning the [`MaleGiven`] entry when the person is male so that
    /// their children's patronymics can be formed later.
    fn draw_given(&mut self, sex: Sex) -> (String, Option<MaleGiven>) {
        match sex {
            Sex::Male => {
                let entry = *self.rng.pick(MALE_GIVEN).unwrap_or(&MALE_GIVEN[0]);
                (entry.name.to_owned(), Some(entry))
            }
            Sex::Female => {
                let name = *self.rng.pick(FEMALE_GIVEN).unwrap_or(&FEMALE_GIVEN[0]);
                (name.to_owned(), None)
            }
        }
    }

    fn push_person(
        &mut self,
        mut person: Person,
        male_given: Option<MaleGiven>,
        line: &'static str,
    ) -> PersonId {
        let id = PersonId(u32::try_from(self.people.len()).unwrap_or(u32::MAX));
        person.id = id;
        self.people.push(person);
        self.male_given_of.push(male_given);
        self.line_surname.push(line);
        id
    }

    fn new_family(
        &mut self,
        husband: Option<PersonId>,
        wife: Option<PersonId>,
        marriage_year: i32,
    ) -> FamilyId {
        // Nobody marries posthumously. A partner can be drawn a short life that happens to end
        // before the wedding; rejecting and redrawing would make generation non-deterministic
        // in its consumption of the stream, so the death is pushed past the marriage instead,
        // by a plausible and varied margin.
        let bumps = [self.rng.range(1, 34), self.rng.range(1, 34)];

        let id = FamilyId(u32::try_from(self.families.len()).unwrap_or(u32::MAX));
        self.families.push(Family {
            id,
            husband,
            wife,
            children: Vec::new(),
            marriage_year,
        });

        for (slot, partner) in [husband, wife].into_iter().enumerate() {
            let Some(partner) = partner else { continue };
            let person = &mut self.people[partner.0 as usize];
            person.spouse_in.push(id);
            if person.death_year.is_some_and(|d| d < marriage_year) {
                person.death_year = Some(marriage_year + bumps[slot]);
            }
        }
        id
    }
}

/// The facts about a cohort member that partner selection needs.
///
/// Bundled rather than passed loose: the alternative is an eight-argument call in which the
/// order of three integers is the only thing standing between correct and silently wrong.
#[derive(Debug, Clone, Copy)]
struct Suitor {
    id: PersonId,
    index: usize,
    sex: Sex,
    generation: u16,
    birth_year: i32,
}

/// Tracks which people have already been paired off during one generation's marriage pass.
///
/// A plain `Vec<bool>` sized up front will not do: the pass itself mints new people, so the
/// set has to grow underneath it.
#[derive(Debug, Default)]
struct PartneredSet {
    flags: Vec<bool>,
}

impl PartneredSet {
    fn contains(&self, id: PersonId) -> bool {
        self.flags.get(id.0 as usize).copied().unwrap_or(false)
    }

    fn insert(&mut self, id: PersonId) {
        let i = id.0 as usize;
        if i >= self.flags.len() {
            self.flags.resize(i + 1, false);
        }
        self.flags[i] = true;
    }
}

const fn sex_opposite(sex: Sex) -> Sex {
    match sex {
        Sex::Male => Sex::Female,
        Sex::Female => Sex::Male,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

    use super::{Person, PersonId, Sex, SyntheticTree, TreeSpec};
    use std::collections::HashSet;

    /// Folds ё to е. Russian stress shifts turn ё into е when it leaves the stressed
    /// syllable, so Пётр becomes Петрович while Фёдор keeps its ё in Фёдорович.
    fn fold_yo(s: &str) -> String {
        s.replace('ё', "е").replace('Ё', "Е")
    }

    fn small() -> SyntheticTree {
        SyntheticTree::generate(TreeSpec {
            seed: 99,
            founding_couples: 3,
            generations: 6,
            ..TreeSpec::default()
        })
    }

    #[test]
    fn generation_is_deterministic() {
        let a = SyntheticTree::generate(TreeSpec::default());
        let b = SyntheticTree::generate(TreeSpec::default());
        assert_eq!(a.people.len(), b.people.len());
        assert_eq!(a.families.len(), b.families.len());
        for (p, q) in a.people.iter().zip(&b.people) {
            assert_eq!(p.full_name(), q.full_name());
            assert_eq!(p.birth_year, q.birth_year);
            assert_eq!(p.death_year, q.death_year);
            assert_eq!(p.child_of, q.child_of);
        }
    }

    #[test]
    fn different_seeds_produce_different_trees() {
        let a = SyntheticTree::generate(TreeSpec {
            seed: 1,
            ..TreeSpec::default()
        });
        let b = SyntheticTree::generate(TreeSpec {
            seed: 2,
            ..TreeSpec::default()
        });
        let names_a: Vec<_> = a.people.iter().take(50).map(Person::full_name).collect();
        let names_b: Vec<_> = b.people.iter().take(50).map(Person::full_name).collect();
        assert_ne!(names_a, names_b);
    }

    #[test]
    fn ids_match_their_index() {
        let tree = small();
        for (i, p) in tree.people.iter().enumerate() {
            assert_eq!(p.id, PersonId(i as u32));
        }
        for (i, f) in tree.families.iter().enumerate() {
            assert_eq!(f.id.0 as usize, i);
        }
    }

    #[test]
    fn no_child_is_born_before_a_parent() {
        let tree = small();
        for family in &tree.families {
            for &child in &family.children {
                let c = tree.person(child);
                for parent in [family.husband, family.wife].into_iter().flatten() {
                    let p = tree.person(parent);
                    assert!(
                        p.birth_year < c.birth_year,
                        "{} (b. {}) is not older than child {} (b. {})",
                        p.full_name(),
                        p.birth_year,
                        c.full_name(),
                        c.birth_year
                    );
                }
            }
        }
    }

    #[test]
    fn generations_increase_from_parent_to_child() {
        let tree = small();
        for family in &tree.families {
            for &child in &family.children {
                let c = tree.person(child);
                for parent in [family.husband, family.wife].into_iter().flatten() {
                    assert!(tree.person(parent).generation < c.generation);
                }
            }
        }
    }

    #[test]
    fn partners_have_opposite_sexes_and_membership_is_symmetric() {
        let tree = small();
        for family in &tree.families {
            if let Some(h) = family.husband {
                assert_eq!(tree.person(h).sex, Sex::Male);
                assert!(tree.person(h).spouse_in.contains(&family.id));
            }
            if let Some(w) = family.wife {
                assert_eq!(tree.person(w).sex, Sex::Female);
                assert!(tree.person(w).spouse_in.contains(&family.id));
            }
        }
        for person in &tree.people {
            for fid in &person.spouse_in {
                let f = tree.family(*fid);
                assert!(f.husband == Some(person.id) || f.wife == Some(person.id));
            }
            if let Some(fid) = person.child_of {
                assert!(tree.family(fid).children.contains(&person.id));
            }
        }
    }

    #[test]
    fn the_ancestor_graph_is_layered_and_terminates() {
        let tree = small();
        // Reaching the same ancestor twice is a *diamond*, not a cycle — that is pedigree
        // collapse, and it is the property we are deliberately generating. What must hold is
        // that generation strictly decreases along every parent edge, which makes a cycle
        // impossible and bounds the walk.
        for person in &tree.people {
            let mut seen = HashSet::new();
            let mut frontier = vec![person.id];
            let mut visits = 0_usize;
            while let Some(id) = frontier.pop() {
                visits += 1;
                assert!(
                    visits < 1_000_000,
                    "ancestor walk from {:?} did not terminate",
                    person.id
                );
                if !seen.insert(id) {
                    continue;
                }
                let here = tree.person(id).generation;
                if let Some(fid) = tree.person(id).child_of {
                    let f = tree.family(fid);
                    for parent in [f.husband, f.wife].into_iter().flatten() {
                        assert!(
                            tree.person(parent).generation < here,
                            "parent {parent:?} is not in an earlier generation than child {id:?}"
                        );
                        frontier.push(parent);
                    }
                }
            }
        }
    }

    #[test]
    fn some_ancestor_is_reachable_by_two_distinct_paths() {
        // The concrete consequence of pedigree collapse, asserted directly: without it the
        // graph is a plain tree and a layout engine that cannot handle a DAG would pass.
        let tree = SyntheticTree::generate(TreeSpec {
            seed: 4,
            founding_couples: 6,
            generations: 8,
            cousin_marriage_rate: 0.10,
            ..TreeSpec::default()
        });

        let diamond_found = tree.people.iter().any(|person| {
            let mut arrivals: std::collections::HashMap<PersonId, usize> =
                std::collections::HashMap::new();
            let mut frontier = vec![person.id];
            while let Some(id) = frontier.pop() {
                let count = arrivals.entry(id).or_default();
                *count += 1;
                if *count > 1 {
                    continue; // already expanded once
                }
                if let Some(fid) = tree.person(id).child_of {
                    let f = tree.family(fid);
                    frontier.extend([f.husband, f.wife].into_iter().flatten());
                }
            }
            arrivals.values().any(|&n| n > 1)
        });

        assert!(diamond_found, "no ancestor was reachable by two paths");
    }

    #[test]
    fn nobody_marries_posthumously() {
        let tree = SyntheticTree::generate(TreeSpec {
            seed: 31,
            founding_couples: 5,
            generations: 8,
            ..TreeSpec::default()
        });
        for family in &tree.families {
            for partner in [family.husband, family.wife].into_iter().flatten() {
                let p = tree.person(partner);
                if let Some(d) = p.death_year {
                    assert!(
                        d >= family.marriage_year,
                        "{} died in {d} but married in {}",
                        p.full_name(),
                        family.marriage_year
                    );
                }
            }
        }
    }

    #[test]
    fn people_who_married_in_reached_adulthood() {
        let tree = small();
        for p in &tree.people {
            if p.child_of.is_some() {
                continue; // born into the tree; infant mortality applies to them
            }
            if let Some(d) = p.death_year {
                assert!(
                    d - p.birth_year >= 18,
                    "{} married in but died at {}",
                    p.full_name(),
                    d - p.birth_year
                );
            }
        }
    }

    #[test]
    fn nobody_dies_before_they_are_born() {
        let tree = small();
        for p in &tree.people {
            if let Some(d) = p.death_year {
                assert!(d >= p.birth_year, "{} died before birth", p.full_name());
            }
        }
    }

    #[test]
    fn patronymics_follow_the_father() {
        let tree = small();
        let mut checked = 0;
        for family in &tree.families {
            let Some(father) = family.husband else {
                continue;
            };
            let father_given = tree.person(father).given.clone();
            for &child in &family.children {
                let c = tree.person(child);
                let Some(patronymic) = &c.patronymic else {
                    continue;
                };
                // The patronymic must be built from the father's given name, so it shares a
                // meaningful prefix with it — after normalising ё, which alternates with е
                // under the stress shift: Пётр yields Петрович, Фёдор yields Фёдорович.
                let stem: String = fold_yo(&father_given).chars().take(3).collect();
                assert!(
                    fold_yo(patronymic).starts_with(&stem),
                    "{patronymic} is not a patronymic of {father_given}"
                );
                checked += 1;
            }
        }
        assert!(checked > 20, "too few patronymics exercised ({checked})");
    }

    #[test]
    fn the_tree_exhibits_pedigree_collapse_and_remarriage() {
        let tree = SyntheticTree::generate(TreeSpec {
            seed: 4,
            founding_couples: 6,
            generations: 8,
            cousin_marriage_rate: 0.10,
            ..TreeSpec::default()
        });
        let stats = tree.stats();
        assert!(
            stats.endogamous_unions > 0,
            "no in-tree marriages: the graph is a plain tree and would not exercise a DAG layout"
        );
        assert!(stats.remarried > 0, "nobody remarried");
        assert!(stats.married_in > 0, "nobody married in from outside");
    }

    #[test]
    fn sized_for_reaches_its_target() {
        for target in [500_usize, 5_000, 20_000] {
            let tree = SyntheticTree::generate(TreeSpec::sized_for(target, 7));
            let n = tree.people.len();
            assert!(n >= target, "asked for {target}, produced {n}");
            // Overshoot is bounded: generation stops at the first check past the target, and a
            // single family or remarriage can carry it a little further.
            assert!(
                n < target + target / 4 + 200,
                "asked for {target}, overshot to {n}"
            );
        }
    }

    #[test]
    fn statistics_are_self_consistent() {
        let tree = small();
        let s = tree.stats();
        assert_eq!(s.people, tree.people.len());
        assert_eq!(s.families, tree.families.len());
        assert!(s.generations <= usize::from(tree.spec.generations) + 1);
        assert!(s.living <= s.people);
        assert!(s.mean_children > 0.0);
    }
}
