//! Personal names.
//!
//! Names are not strings. `docs/01-research/data-standards.md` §5 lists the parts, and Russian
//! research makes each one matter:
//!
//! - **The patronymic is a part of its own.** *Иван Петрович Смирнов* has a given name, a
//!   patronymic and a surname, and folding the patronymic into the given name — as GEDCOM does,
//!   having no tag for it — makes "all Petroviches" an unanswerable question.
//! - **A person has several names**, each typed: a birth name, a married name, a baptismal or
//!   monastic name. Each is its own fact with its own sources; a second name is not a conflict
//!   with the first (ADR 0010).
//! - **Records are in pre-reform spelling.** A parish register from 1871 writes *Петръ*,
//!   *Ѳеодоръ*, *Матѳей*; a grandson types *Пётр*, *Фёдор*, *Матфей*. They are the same names, and
//!   comparing them letter by letter would make every nineteenth-century record contradict every
//!   modern one. So comparison goes through [`fold`], which undoes the 1918 orthographic reform,
//!   ignores *ё* against *е*, and drops stress marks.
//!
//! What `fold` deliberately does **not** do is match names that sound alike — *Иоанн* and *Иван*,
//! *Müller* and *Mueller*. That is phonetic matching, which is probabilistic and belongs to
//! `drevigen-match`. Agreement between two assertions must be certain, so it stops at spelling
//! conventions that are certainly the same name.

use core::fmt::Write as _;

use crate::claim::Claimable;

/// What kind of name this is.
///
/// GEDCOM 7's `NAME.TYPE` values, plus the one Orthodox and Catholic records need and GEDCOM files
/// under `OTHER`: the name given at baptism or taken in religious life, which is frequently not the
/// name a person was known by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NameKind {
    /// The name at birth.
    Birth,
    /// A woman's surname before marriage, where recorded as such.
    Maiden,
    /// A name taken on marriage.
    Married,
    /// A baptismal, monastic or other religious name.
    Religious,
    /// A name taken on emigration.
    Immigrant,
    /// A professional or stage name.
    Professional,
    /// A name someone was also known by.
    AlsoKnownAs,
    /// Something else, described in the source.
    Other,
}

impl NameKind {
    /// The GEDCOM 7 `TYPE` value. The religious name has none and is written as `OTHER`.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Birth => "BIRTH",
            Self::Maiden => "MAIDEN",
            Self::Married => "MARRIED",
            Self::Immigrant => "IMMIGRANT",
            Self::Professional => "PROFESSIONAL",
            Self::AlsoKnownAs => "AKA",
            Self::Religious | Self::Other => "OTHER",
        }
    }
}

/// The order a full name is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameOrder {
    /// *Иван Петрович Смирнов*, *Johann Schmidt*: how a name is spoken.
    GivenFirst,
    /// *Смирнов Иван Петрович*: how Russian lists, indexes and documents write it, and how a
    /// list of people is sorted.
    SurnameFirst,
}

/// A personal name, in parts.
///
/// Every part is optional, because records are partial: a revision list may give only a given name
/// and a patronymic, a gravestone only a surname.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonName {
    /// What kind of name, where known.
    pub kind: Option<NameKind>,
    /// A title before the name: *князь*, *Dr.*, *протоиерей*.
    pub prefix: Option<String>,
    /// The given name: *Иван*.
    pub given: Option<String>,
    /// The patronymic: *Петрович*, *Петровна*.
    pub patronymic: Option<String>,
    /// A particle belonging to the surname: *фон*, *van der*, *de*.
    pub surname_prefix: Option<String>,
    /// The surname: *Смирнов*, *Смирнова*.
    pub surname: Option<String>,
    /// After the name: *Jr.*, *III*, *младший*.
    pub suffix: Option<String>,
    /// What family called them: *Ваня*.
    pub nickname: Option<String>,
}

impl PersonName {
    /// A name from a given name, a patronymic and a surname — the commonest Russian record.
    #[must_use]
    pub fn russian(given: &str, patronymic: &str, surname: &str) -> Self {
        Self {
            given: non_empty(given),
            patronymic: non_empty(patronymic),
            surname: non_empty(surname),
            ..Self::default()
        }
    }

    /// Every part, in a fixed order, for comparing part against part.
    fn parts(&self) -> [&Option<String>; 7] {
        [
            &self.prefix,
            &self.given,
            &self.patronymic,
            &self.surname_prefix,
            &self.surname,
            &self.suffix,
            &self.nickname,
        ]
    }

    /// How many parts are present. A name with more parts says more.
    #[must_use]
    pub fn detail(&self) -> usize {
        self.parts().iter().filter(|part| part.is_some()).count()
    }

    /// Whether no part is present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.detail() == 0
    }

    /// The name written out in full, in the order asked for.
    ///
    /// The nickname is not included: it is what the family called someone, not their name, and
    /// the interface shows it separately.
    #[must_use]
    pub fn full(&self, order: NameOrder) -> String {
        let surname = match (&self.surname_prefix, &self.surname) {
            (Some(prefix), Some(surname)) => Some(format!("{prefix} {surname}")),
            (None, Some(surname)) => Some(surname.clone()),
            (Some(prefix), None) => Some(prefix.clone()),
            (None, None) => None,
        };
        let spoken = [&self.given, &self.patronymic];

        let mut out = String::new();
        let mut push = |part: Option<&str>| {
            if let Some(part) = part {
                if !out.is_empty() {
                    out.push(' ');
                }
                let _ = write!(out, "{part}");
            }
        };

        push(self.prefix.as_deref());
        match order {
            NameOrder::GivenFirst => {
                for part in spoken {
                    push(part.as_deref());
                }
                push(surname.as_deref());
            }
            NameOrder::SurnameFirst => {
                push(surname.as_deref());
                for part in spoken {
                    push(part.as_deref());
                }
            }
        }
        push(self.suffix.as_deref());
        out
    }

    /// A key for sorting people alphabetically: surname, then given name, then patronymic,
    /// each folded, so that *Смирновъ* sorts with *Смирнов*.
    #[must_use]
    pub fn sort_key(&self) -> String {
        [&self.surname, &self.given, &self.patronymic]
            .iter()
            .map(|part| part.as_deref().map(fold).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\u{1F}")
    }
}

/// A part of a name, with nothing but whitespace treated as absent.
fn non_empty(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// Two names agree when every part present in both is the same after [`fold`]; a part present in
/// only one is a refinement, not a disagreement. Two names of different declared kinds do not
/// agree — a birth name and a married name are two facts, not two readings of one.
impl Claimable for PersonName {
    fn compatible(&self, other: &Self) -> bool {
        let kinds = match (self.kind, other.kind) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        };
        kinds
            && self
                .parts()
                .iter()
                .zip(other.parts())
                .all(|(a, b)| match (a, b) {
                    (Some(a), Some(b)) => fold(a) == fold(b),
                    _ => true,
                })
    }

    fn more_specific_than(&self, other: &Self) -> bool {
        let this = self.detail() + usize::from(self.kind.is_some());
        let that = other.detail() + usize::from(other.kind.is_some());
        this > that
    }
}

/// Reduces a name to the form in which spelling conventions no longer differ.
///
/// - Lower case.
/// - *ё* as *е* — Russian writes the diaeresis inconsistently, and a record and its transcription
///   routinely disagree about it.
/// - The letters the 1918 reform removed, as the reform replaced them: *і* and *ѵ* as *и*, *ѣ* as
///   *е*, *ѳ* as *ф*; and the hard sign at the end of a word, which the reform dropped, removed.
///   *Петръ* folds to *петр*, as does *Пётр*.
/// - Stress marks removed: dictionaries and some transcriptions write *Ива́н*.
/// - Whitespace collapsed, and spaces around a hyphen removed, so that *Римский - Корсаков* is
///   *римский-корсаков*.
///
/// Nothing phonetic: see the module documentation for why.
#[must_use]
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;

    for character in text.chars() {
        // U+0301 and U+0300: the acute and grave written over a vowel to mark stress. Always
        // combining in Cyrillic, which has no precomposed stressed letters.
        if matches!(character, '\u{0301}' | '\u{0300}') {
            continue;
        }
        if character.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        if character == '-' {
            // A hyphen absorbs the spaces around it.
            pending_space = false;
            out.push('-');
            continue;
        }
        if pending_space && !out.ends_with('-') {
            out.push(' ');
        }
        pending_space = false;

        for lower in character.to_lowercase() {
            out.push(match lower {
                'ё' | 'ѣ' => 'е',
                'і' | 'ѵ' => 'и',
                'ѳ' => 'ф',
                other => other,
            });
        }
    }

    drop_final_hard_signs(&out)
}

/// Removes a hard sign at the end of each word, where the pre-reform spelling wrote one after
/// every final consonant. A hard sign inside a word — *подъезд* — is a letter and stays.
fn drop_final_hard_signs(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    characters
        .iter()
        .enumerate()
        .filter(|(index, character)| {
            let at_word_end = characters
                .get(index + 1)
                .is_none_or(|next| !next.is_alphabetic());
            !(**character == 'ъ' && at_word_end)
        })
        .map(|(_, character)| *character)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{NameKind, NameOrder, PersonName, fold};
    use crate::claim::Claimable;

    #[test]
    fn pre_reform_spelling_is_the_same_name() {
        // A register from 1871 against a grandson's typing.
        assert_eq!(fold("Петръ"), fold("Пётр"));
        assert_eq!(fold("Ѳеодоръ"), fold("Феодор"));
        assert_eq!(fold("Матѳей"), fold("Матфей"));
        assert_eq!(fold("Алексѣй"), fold("Алексей"));
        assert_eq!(fold("Іоаннъ"), fold("Иоанн"));
        assert_eq!(fold("Смирновъ"), fold("Смирнов"));
    }

    #[test]
    fn a_hard_sign_inside_a_word_is_a_letter() {
        assert_eq!(fold("Подъяпольский"), "подъяпольский");
        assert_eq!(fold("Съедин"), "съедин");
    }

    #[test]
    fn stress_marks_and_spacing_are_not_spelling() {
        assert_eq!(fold("Ива\u{301}н"), "иван");
        assert_eq!(fold("  Римский - Корсаков "), "римский-корсаков");
        assert_eq!(fold("Anna  Maria"), "anna maria");
    }

    #[test]
    fn phonetic_variants_are_not_folded_together() {
        // Probabilistic matching is drevigen-match's job. Agreement has to be certain.
        assert_ne!(fold("Иоанн"), fold("Иван"));
        assert_ne!(fold("Müller"), fold("Mueller"));
        // The church form and the vernacular are two forms of one name, not two spellings of
        // one form: Ѳеодоръ is Феодор after the reform, and Фёдор is another word.
        assert_ne!(fold("Ѳеодоръ"), fold("Фёдор"));
    }

    #[test]
    fn a_name_with_more_parts_refines_one_with_fewer() {
        let register = PersonName {
            given: Some("Иоаннъ".to_owned()),
            ..PersonName::default()
        };
        let census = PersonName::russian("Иоанн", "Петров", "Смирнов");
        assert!(register.compatible(&census));
        assert!(census.more_specific_than(&register));
    }

    #[test]
    fn a_part_that_differs_is_a_disagreement() {
        let one = PersonName::russian("Иван", "Петрович", "Смирнов");
        let other = PersonName::russian("Иван", "Павлович", "Смирнов");
        assert!(!one.compatible(&other));
    }

    #[test]
    fn a_birth_name_and_a_married_name_are_two_facts_not_two_readings() {
        let birth = PersonName {
            kind: Some(NameKind::Birth),
            ..PersonName::russian("Мария", "Ивановна", "Смирнова")
        };
        let married = PersonName {
            kind: Some(NameKind::Married),
            ..PersonName::russian("Мария", "Ивановна", "Смирнова")
        };
        assert!(!birth.compatible(&married));

        // An undeclared kind is compatible with either — and declaring one says more.
        let undeclared = PersonName::russian("Мария", "Ивановна", "Смирнова");
        assert!(undeclared.compatible(&birth));
        assert!(birth.more_specific_than(&undeclared));
    }

    #[test]
    fn the_full_name_in_both_orders() {
        let name = PersonName {
            prefix: Some("протоиерей".to_owned()),
            ..PersonName::russian("Иоанн", "Сергиевич", "Ильичёв")
        };
        assert_eq!(
            name.full(NameOrder::GivenFirst),
            "протоиерей Иоанн Сергиевич Ильичёв"
        );
        assert_eq!(
            name.full(NameOrder::SurnameFirst),
            "протоиерей Ильичёв Иоанн Сергиевич"
        );

        let particle = PersonName {
            given: Some("Ludwig".to_owned()),
            surname_prefix: Some("van".to_owned()),
            surname: Some("Beethoven".to_owned()),
            ..PersonName::default()
        };
        assert_eq!(particle.full(NameOrder::GivenFirst), "Ludwig van Beethoven");
        assert_eq!(
            particle.full(NameOrder::SurnameFirst),
            "van Beethoven Ludwig"
        );
    }

    #[test]
    fn people_sort_by_surname_with_old_spellings_among_the_new() {
        let old = PersonName::russian("Петръ", "", "Смирновъ");
        let new = PersonName::russian("Пётр", "", "Смирнов");
        assert_eq!(old.sort_key(), new.sort_key());

        let mut names = [
            PersonName::russian("Иван", "", "Яковлев"),
            PersonName::russian("Анна", "", "Абрамова"),
            PersonName::russian("Борис", "", "Абрамов"),
        ];
        names.sort_by_key(PersonName::sort_key);
        let surnames: Vec<_> = names.iter().filter_map(|n| n.surname.as_deref()).collect();
        assert_eq!(surnames, ["Абрамов", "Абрамова", "Яковлев"]);
    }

    #[test]
    fn empty_parts_are_absent() {
        let name = PersonName::russian("Иван", "  ", "");
        assert_eq!(name.detail(), 1);
        assert!(PersonName::default().is_empty());
    }

    #[test]
    fn name_kinds_map_to_gedcom() {
        assert_eq!(NameKind::AlsoKnownAs.tag(), "AKA");
        assert_eq!(NameKind::Religious.tag(), "OTHER");
    }
}
