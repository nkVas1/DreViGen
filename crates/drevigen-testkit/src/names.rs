//! Russian name pools and the grammar needed to combine them.
//!
//! Synthetic people need synthetic names, and for this project they need *Russian* synthetic
//! names — because the name problems DreViGen has to solve are Russian ones: patronymics derived
//! irregularly from the father's given name, and surnames that inflect for the bearer's sex.
//! Generating `Person 00412` would hide exactly the cases the product must get right.
//!
//! Patronymic forms are **stored, not derived**. The suffix looks regular until it is not:
//! Дмитрий yields Дмитриевич while Григорий yields Григорьевич, and Илья yields Ильич with the
//! feminine Ильинична. A table is the honest representation of an irregular paradigm, and the
//! production name module will need one too.

/// A masculine given name together with the patronymics formed from it.
#[derive(Debug, Clone, Copy)]
pub struct MaleGiven {
    /// Nominative form, e.g. `Иван`.
    pub name: &'static str,
    /// Patronymic borne by a son, e.g. `Иванович`.
    pub son: &'static str,
    /// Patronymic borne by a daughter, e.g. `Ивановна`.
    pub daughter: &'static str,
}

/// Masculine given names common in 19th-century Russian parish registers.
pub const MALE_GIVEN: &[MaleGiven] = &[
    MaleGiven {
        name: "Иван",
        son: "Иванович",
        daughter: "Ивановна",
    },
    MaleGiven {
        name: "Пётр",
        son: "Петрович",
        daughter: "Петровна",
    },
    MaleGiven {
        name: "Василий",
        son: "Васильевич",
        daughter: "Васильевна",
    },
    MaleGiven {
        name: "Николай",
        son: "Николаевич",
        daughter: "Николаевна",
    },
    MaleGiven {
        name: "Алексей",
        son: "Алексеевич",
        daughter: "Алексеевна",
    },
    MaleGiven {
        name: "Михаил",
        son: "Михайлович",
        daughter: "Михайловна",
    },
    MaleGiven {
        name: "Фёдор",
        son: "Фёдорович",
        daughter: "Фёдоровна",
    },
    MaleGiven {
        name: "Сергей",
        son: "Сергеевич",
        daughter: "Сергеевна",
    },
    MaleGiven {
        name: "Дмитрий",
        son: "Дмитриевич",
        daughter: "Дмитриевна",
    },
    MaleGiven {
        name: "Григорий",
        son: "Григорьевич",
        daughter: "Григорьевна",
    },
    MaleGiven {
        name: "Андрей",
        son: "Андреевич",
        daughter: "Андреевна",
    },
    MaleGiven {
        name: "Степан",
        son: "Степанович",
        daughter: "Степановна",
    },
    MaleGiven {
        name: "Яков",
        son: "Яковлевич",
        daughter: "Яковлевна",
    },
    MaleGiven {
        name: "Павел",
        son: "Павлович",
        daughter: "Павловна",
    },
    MaleGiven {
        name: "Семён",
        son: "Семёнович",
        daughter: "Семёновна",
    },
    MaleGiven {
        name: "Илья",
        son: "Ильич",
        daughter: "Ильинична",
    },
    MaleGiven {
        name: "Кузьма",
        son: "Кузьмич",
        daughter: "Кузьминична",
    },
    MaleGiven {
        name: "Лука",
        son: "Лукич",
        daughter: "Лукинична",
    },
    MaleGiven {
        name: "Фома",
        son: "Фомич",
        daughter: "Фоминична",
    },
    MaleGiven {
        name: "Никита",
        son: "Никитич",
        daughter: "Никитична",
    },
    MaleGiven {
        name: "Тимофей",
        son: "Тимофеевич",
        daughter: "Тимофеевна",
    },
    MaleGiven {
        name: "Максим",
        son: "Максимович",
        daughter: "Максимовна",
    },
    MaleGiven {
        name: "Гавриил",
        son: "Гаврилович",
        daughter: "Гавриловна",
    },
    MaleGiven {
        name: "Прохор",
        son: "Прохорович",
        daughter: "Прохоровна",
    },
    MaleGiven {
        name: "Захар",
        son: "Захарович",
        daughter: "Захаровна",
    },
    MaleGiven {
        name: "Трофим",
        son: "Трофимович",
        daughter: "Трофимовна",
    },
    MaleGiven {
        name: "Емельян",
        son: "Емельянович",
        daughter: "Емельяновна",
    },
    MaleGiven {
        name: "Афанасий",
        son: "Афанасьевич",
        daughter: "Афанасьевна",
    },
];

/// Feminine given names from the same period.
pub const FEMALE_GIVEN: &[&str] = &[
    "Анна",
    "Мария",
    "Евдокия",
    "Прасковья",
    "Екатерина",
    "Татьяна",
    "Ольга",
    "Дарья",
    "Пелагея",
    "Матрёна",
    "Ирина",
    "Наталья",
    "Ксения",
    "Агафья",
    "Варвара",
    "Аграфена",
    "Елизавета",
    "Феодосия",
    "Устинья",
    "Марфа",
    "Александра",
    "Любовь",
    "Вера",
    "Надежда",
    "Акулина",
    "Домна",
    "Соломония",
    "Гликерия",
];

/// Surnames in masculine nominative form; the feminine is derived by [`feminine_surname`].
pub const SURNAMES: &[&str] = &[
    "Васильев",
    "Петров",
    "Кузнецов",
    "Смирнов",
    "Попов",
    "Соколов",
    "Лебедев",
    "Козлов",
    "Новиков",
    "Морозов",
    "Волков",
    "Зайцев",
    "Павлов",
    "Семёнов",
    "Голубев",
    "Виноградов",
    "Богданов",
    "Воробьёв",
    "Фёдоров",
    "Михайлов",
    "Беляев",
    "Тарасов",
    "Белов",
    "Комаров",
    "Орлов",
    "Киселёв",
    "Макаров",
    "Андреев",
    "Ковалёв",
    "Ильин",
    "Гусев",
    "Титов",
    "Кудрявцев",
    "Баранов",
    "Куликов",
    "Алексеев",
    "Степанов",
    "Яковлев",
    "Сорокин",
    "Сергеев",
    "Романов",
    "Захаров",
    "Борисов",
    "Королёв",
    "Герасимов",
    "Пономарёв",
    "Григорьев",
    "Лазарев",
    "Медведев",
    "Ершов",
    "Никитин",
    "Соболев",
    "Рябов",
    "Поляков",
    "Цветков",
    "Данилов",
    "Жуков",
    "Фролов",
    "Журавлёв",
    "Николаев",
    "Крылов",
    "Максимов",
    "Сидоров",
    "Осипов",
    "Белозёров",
    "Троицкий",
    "Рождественский",
    "Покровский",
    "Успенский",
    "Преображенский",
];

/// Settlement names, used as places of birth and death.
pub const PLACES: &[&str] = &[
    "Подгорное",
    "Никольское",
    "Заречье",
    "Троицкое",
    "Ивановка",
    "Красный Бор",
    "Михайловка",
    "Успенское",
    "Покровское",
    "Спасское",
    "Дмитриевка",
    "Ольховка",
    "Берёзовка",
    "Липовка",
    "Сосновка",
    "Дубровка",
    "Каменка",
    "Песчаное",
    "Луговое",
    "Погорелово",
    "Рождествено",
    "Вознесенское",
    "Белогорье",
    "Черемшаны",
    "Гришино",
    "Лукино",
    "Фомино",
    "Селище",
];

/// Biological sex, as recorded in the registers the data imitates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sex {
    /// Male.
    Male,
    /// Female.
    Female,
}

/// Returns the feminine form of a masculine surname.
///
/// Covers the productive Russian patterns and leaves indeclinable surnames untouched:
///
/// | Masculine | Feminine |
/// |---|---|
/// | Васильев | Васильева |
/// | Никитин | Никитина |
/// | Троицкий | Троицкая |
/// | Толстой | Толстая |
/// | Шевченко | Шевченко |
#[must_use]
pub fn feminine_surname(masculine: &str) -> String {
    for (suffix, replacement) in [
        ("ский", "ская"),
        ("цкий", "цкая"),
        ("ной", "ная"),
        ("той", "тая"),
        ("ый", "ая"),
        ("ий", "яя"),
    ] {
        if let Some(stem) = masculine.strip_suffix(suffix) {
            return format!("{stem}{replacement}");
        }
    }

    // -ов / -ев / -ёв / -ин / -ын simply take -а.
    for suffix in ["ов", "ев", "ёв", "ин", "ын"] {
        if masculine.ends_with(suffix) {
            return format!("{masculine}а");
        }
    }

    // Anything else — Шевченко, Дурново, foreign names — does not inflect.
    masculine.to_owned()
}

/// Returns the surname a person of the given sex bears within a family line.
#[must_use]
pub fn surname_for(masculine: &str, sex: Sex) -> String {
    match sex {
        Sex::Male => masculine.to_owned(),
        Sex::Female => feminine_surname(masculine),
    }
}

/// Returns the patronymic a child of `father` bears.
#[must_use]
pub fn patronymic_for(father: MaleGiven, sex: Sex) -> &'static str {
    match sex {
        Sex::Male => father.son,
        Sex::Female => father.daughter,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

    use super::{
        FEMALE_GIVEN, MALE_GIVEN, PLACES, SURNAMES, Sex, feminine_surname, patronymic_for,
        surname_for,
    };

    #[test]
    fn feminine_surnames_follow_the_paradigm() {
        for (masculine, expected) in [
            ("Васильев", "Васильева"),
            ("Ковалёв", "Ковалёва"),
            ("Никитин", "Никитина"),
            ("Птицын", "Птицына"),
            ("Троицкий", "Троицкая"),
            ("Высоцкий", "Высоцкая"),
            ("Толстой", "Толстая"),
            ("Шевченко", "Шевченко"),
            ("Дурново", "Дурново"),
        ] {
            assert_eq!(feminine_surname(masculine), expected, "for {masculine}");
        }
    }

    #[test]
    fn every_listed_surname_has_a_feminine_form() {
        for &s in SURNAMES {
            let f = feminine_surname(s);
            assert!(!f.is_empty());
            // Every surname in our pool is a declinable Russian one, so it must change.
            assert_ne!(f, s, "{s} did not inflect");
        }
    }

    #[test]
    fn surname_for_male_is_unchanged() {
        assert_eq!(surname_for("Васильев", Sex::Male), "Васильев");
        assert_eq!(surname_for("Васильев", Sex::Female), "Васильева");
    }

    #[test]
    fn patronymics_are_irregular_and_stored_correctly() {
        let by_name = |n: &str| {
            *MALE_GIVEN
                .iter()
                .find(|m| m.name == n)
                .unwrap_or_else(|| panic!("{n} missing from the pool"))
        };

        assert_eq!(patronymic_for(by_name("Иван"), Sex::Male), "Иванович");
        assert_eq!(patronymic_for(by_name("Иван"), Sex::Female), "Ивановна");
        // The pair that defeats a naive suffix rule.
        assert_eq!(patronymic_for(by_name("Дмитрий"), Sex::Male), "Дмитриевич");
        assert_eq!(
            patronymic_for(by_name("Григорий"), Sex::Male),
            "Григорьевич"
        );
        // And the -ич / -инична paradigm.
        assert_eq!(patronymic_for(by_name("Илья"), Sex::Male), "Ильич");
        assert_eq!(patronymic_for(by_name("Илья"), Sex::Female), "Ильинична");
        assert_eq!(
            patronymic_for(by_name("Кузьма"), Sex::Female),
            "Кузьминична"
        );
    }

    #[test]
    fn pools_are_non_empty_and_free_of_duplicates() {
        let mut males: Vec<_> = MALE_GIVEN.iter().map(|m| m.name).collect();
        males.sort_unstable();
        let before = males.len();
        males.dedup();
        assert_eq!(before, males.len(), "duplicate masculine given name");

        let mut females = FEMALE_GIVEN.to_vec();
        females.sort_unstable();
        let before = females.len();
        females.dedup();
        assert_eq!(before, females.len(), "duplicate feminine given name");

        let mut surnames = SURNAMES.to_vec();
        surnames.sort_unstable();
        let before = surnames.len();
        surnames.dedup();
        assert_eq!(before, surnames.len(), "duplicate surname");

        assert!(!PLACES.is_empty());
    }
}
