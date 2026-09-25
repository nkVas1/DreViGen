//! Dates in words, in Russian, English and German.
//!
//! This lives beside the calendars rather than in the interface's message catalogues, because
//! the thing that makes it hard is not the words but the arithmetic under them. Russian practice
//! writes a date before February 1918 in both styles at once — *25 октября (7 ноября) 1917 г.* —
//! and producing that needs the Julian-to-Gregorian conversion, the knowledge of when the two
//! fall in different years, and in the Hebrew calendar the knowledge of whether a year has an
//! Adar I. A message format has none of that; this crate has all of it.
//!
//! # What "made properly" means here
//!
//! - **Russian months take the case their preposition demands.** *апрель 1871 г.* on its own,
//!   *около апреля 1871 г.*, *между апрелем 1869 г. и маем 1873 г.* A formatter that writes
//!   *между апрель* is readable and wrong, and a Russian reader notices at once. With a day the
//!   month is always genitive — *17 апреля* — whatever surrounds it, because it is the number
//!   that takes the case, and numbers are written as digits.
//! - **Two bare years share their abbreviation.** *между 1869 и 1873 гг.*, not *между 1869 г. и
//!   1873 г.*
//! - **Republican years are Roman numerals**, as the calendar wrote them: *9 термидора II года*,
//!   *9 Thermidor Year II*.
//! - **A Julian date that cannot be converted says it is Julian.** "April 1871" in the old style
//!   has no single new-style equivalent, so it is shown as written and marked *(ст. ст.)*,
//!   *(O.S.)*, *(a. St.)* rather than left to be misread.
//! - **Quotation marks are the locale's own**: «ёлочки», “English”, „deutsche“.

use crate::gedcom::month_tag;
use crate::value::{Approximation, DateValue, RecordedDate};
use crate::{Calendar, CalendarDate};

/// A language the formatter writes.
///
/// The three launch locales. Adding one is a table and a set of prepositions, and the Russian
/// case system is the hardest any of them will need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    /// Русский.
    Russian,
    /// English.
    English,
    /// Deutsch.
    German,
}

impl Locale {
    /// The BCP 47 language subtag.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Russian => "ru",
            Self::English => "en",
            Self::German => "de",
        }
    }

    /// Reads a BCP 47 tag by its language subtag: `ru`, `ru-RU`, `de-AT`, `en-US` and so on.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        let language = tag.split(['-', '_']).next()?;
        match language.to_ascii_lowercase().as_str() {
            "ru" => Some(Self::Russian),
            "en" => Some(Self::English),
            "de" => Some(Self::German),
            _ => None,
        }
    }
}

/// How dates in other calendars are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CalendarDisplay {
    /// As the source wrote it, marked if it is Julian.
    AsRecorded,
    /// Converted to the Gregorian calendar wherever the date is complete enough to convert.
    Gregorian,
    /// As written, with the Gregorian date beside it. The Russian historiographical convention,
    /// and the default in every locale because it hides nothing: the reader sees what the record
    /// says and what it means.
    #[default]
    Paired,
}

/// How to write dates: in which language, and how to treat other calendars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DateFormat {
    /// The language.
    pub locale: Locale,
    /// What to do with dates that are not Gregorian.
    pub calendar: CalendarDisplay,
}

/// The grammatical case a Russian month takes after the word before it. The other locales do
/// not inflect months and ignore it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    Nominative,
    Genitive,
    Instrumental,
}

impl DateFormat {
    /// The format for a locale, with dates in other calendars paired with their Gregorian
    /// equivalent.
    #[must_use]
    pub const fn new(locale: Locale) -> Self {
        Self {
            locale,
            calendar: CalendarDisplay::Paired,
        }
    }

    /// Writes one date.
    ///
    /// ```
    /// use drevigen_date::{Calendar, CalendarDate, DateFormat, Locale};
    ///
    /// let revolution = CalendarDate::new(Calendar::Julian, 1917, Some(10), Some(25)).unwrap();
    /// assert_eq!(
    ///     DateFormat::new(Locale::Russian).date(&revolution),
    ///     "25 октября (7 ноября) 1917 г."
    /// );
    /// ```
    #[must_use]
    pub fn date(self, date: &CalendarDate) -> String {
        self.date_in(date, Case::Nominative)
    }

    /// Writes a date value, with the words its form calls for.
    #[must_use]
    pub fn value(self, value: &DateValue) -> String {
        let words = self.words();
        match value {
            DateValue::Exact(date) => self.date(date),
            DateValue::Approximate { date, kind } => {
                let (word, case) = match kind {
                    Approximation::About => (words.about, Case::Genitive),
                    Approximation::Calculated => (words.calculated, Case::Nominative),
                    Approximation::Estimated => (words.estimated, Case::Nominative),
                };
                format!("{word} {}", self.date_in(date, case))
            }
            DateValue::Before(date) => {
                format!("{} {}", words.before, self.date_in(date, Case::Genitive))
            }
            DateValue::After(date) => {
                format!("{} {}", words.after, self.date_in(date, Case::Genitive))
            }
            DateValue::Between { earliest, latest } => {
                if let Some((first, second)) = self.shared_years(earliest, latest) {
                    return format!(
                        "{} {first} {} {second}{}",
                        words.between, words.and, words.years
                    );
                }
                format!(
                    "{} {} {} {}",
                    words.between,
                    self.date_in(earliest, Case::Instrumental),
                    words.and,
                    self.date_in(latest, Case::Instrumental)
                )
            }
            DateValue::Period { from, to } => match (from, to) {
                (Some(start), Some(end)) => {
                    if let Some((first, second)) = self.shared_years(start, end) {
                        return format!(
                            "{} {first} {} {second}{}",
                            words.from, words.to, words.years
                        );
                    }
                    format!(
                        "{} {} {} {}",
                        words.from,
                        self.date_in(start, Case::Genitive),
                        words.to,
                        // "по апрель" — the accusative, which for these months is the
                        // nominative form.
                        self.date_in(end, Case::Nominative)
                    )
                }
                (Some(start), None) => {
                    format!("{} {}", words.since, self.date_in(start, Case::Genitive))
                }
                (None, Some(end)) => {
                    format!("{} {}", words.until, self.date_in(end, Case::Nominative))
                }
                (None, None) => words.unknown.to_owned(),
            },
        }
    }

    /// Writes a recorded date: its value if it has one, otherwise its words in the locale's
    /// quotation marks, otherwise that the date is unknown.
    ///
    /// A phrase beside a value is not repeated here. The value is what a list or a chart shows;
    /// the words belong to the dossier, where there is room to say where they came from.
    #[must_use]
    pub fn recorded(self, recorded: &RecordedDate) -> String {
        if let Some(value) = &recorded.value {
            return self.value(value);
        }
        let words = self.words();
        match &recorded.phrase {
            Some(phrase) => format!("{}{phrase}{}", words.open_quote, words.close_quote),
            None => words.unknown.to_owned(),
        }
    }

    /// A date in the calendar display this format asks for.
    fn date_in(self, date: &CalendarDate, case: Case) -> String {
        let gregorian = || date.to_calendar(Calendar::Gregorian).ok();

        match (date.calendar(), self.calendar) {
            (Calendar::Gregorian, _) => self.render(date, case),

            (Calendar::Julian, CalendarDisplay::Paired) if date.is_exact() => gregorian()
                .map_or_else(
                    || self.marked(date, case),
                    |modern| self.julian_pair(date, &modern),
                ),
            (Calendar::Julian, CalendarDisplay::Gregorian) if date.is_exact() => gregorian()
                .map_or_else(
                    || self.marked(date, case),
                    |modern| self.render(&modern, case),
                ),
            (Calendar::Julian, _) => self.marked(date, case),

            (_, CalendarDisplay::Paired) if date.is_exact() => match gregorian() {
                Some(modern) => format!(
                    "{} ({})",
                    self.render(date, case),
                    self.render(&modern, Case::Nominative)
                ),
                None => self.render(date, case),
            },
            (_, CalendarDisplay::Gregorian) if date.is_exact() => gregorian().map_or_else(
                || self.render(date, case),
                |modern| self.render(&modern, case),
            ),
            _ => self.render(date, case),
        }
    }

    /// A Julian date with its Gregorian twin: *25 октября (7 ноября) 1917 г.*, or both whole
    /// when the two fall in different years.
    fn julian_pair(self, old: &CalendarDate, new: &CalendarDate) -> String {
        if old.year() != new.year() {
            return format!(
                "{} ({})",
                self.render(old, Case::Nominative),
                self.render(new, Case::Nominative)
            );
        }
        format!(
            "{} ({}) {}",
            self.day_and_month(old),
            self.day_and_month(new),
            self.year(old, Case::Genitive)
        )
    }

    /// A Julian date shown as written, with the old-style mark.
    fn marked(self, date: &CalendarDate, case: Case) -> String {
        format!("{} {}", self.render(date, case), self.words().old_style)
    }

    /// A date in its own calendar, without conversion or marks.
    fn render(self, date: &CalendarDate, case: Case) -> String {
        match (date.month(), date.day()) {
            (Some(_), Some(_)) => {
                format!(
                    "{} {}",
                    self.day_and_month(date),
                    self.year(date, Case::Genitive)
                )
            }
            (Some(month), None) => format!(
                "{} {}",
                self.month_name(date, month, case),
                self.year(date, Case::Genitive)
            ),
            _ => self.year(date, case),
        }
    }

    /// The day and month of a complete date: *17 апреля*, *17 April*, *17. April*.
    fn day_and_month(self, date: &CalendarDate) -> String {
        let day = date.day().unwrap_or(1);
        let month = date.month().unwrap_or(1);

        // The Republican complementary days are not a month, and are written as numbered days.
        if date.calendar() == Calendar::FrenchRepublican && month == 13 {
            return match self.locale {
                Locale::Russian => format!("{day}-й дополнительный день"),
                Locale::English => format!("{} complementary day,", ordinal(day)),
                Locale::German => format!("{day}. Ergänzungstag"),
            };
        }

        // With a day, the Russian month is genitive whatever the preposition: the case falls on
        // the number, and the number is written in digits.
        let name = self.month_name(date, month, Case::Genitive);
        match self.locale {
            Locale::Russian | Locale::English => format!("{day} {name}"),
            Locale::German => format!("{day}. {name}"),
        }
    }

    /// The year of a date, in the form its calendar and the locale call for.
    fn year(self, date: &CalendarDate, case: Case) -> String {
        let year = date.year();

        if date.calendar() == Calendar::FrenchRepublican {
            let numeral = roman(year);
            return match self.locale {
                Locale::Russian => {
                    let noun = match case {
                        Case::Nominative => "год",
                        Case::Genitive => "года",
                        Case::Instrumental => "годом",
                    };
                    format!("{numeral} {noun}")
                }
                Locale::English => format!("Year {numeral}"),
                Locale::German => {
                    if date.month().is_some() {
                        numeral
                    } else {
                        format!("Jahr {numeral}")
                    }
                }
            };
        }

        // Astronomical year 0 is 1 BCE, -4 is 5 BCE.
        let (number, before_era) = if year <= 0 {
            (1 - year, true)
        } else {
            (year, false)
        };
        match (self.locale, before_era) {
            (Locale::Russian, false) => format!("{number} г."),
            (Locale::Russian, true) => format!("{number} г. до н. э."),
            (Locale::English | Locale::German, false) => number.to_string(),
            (Locale::English, true) => format!("{number} BCE"),
            (Locale::German, true) => format!("{number} v. Chr."),
        }
    }

    /// When both ends of a range or period are bare Gregorian years, the two numbers, so that
    /// they can share one abbreviation: *между 1869 и 1873 гг.*
    ///
    /// Only Russian writes the abbreviation at all, and only for years of the common era; every
    /// other case writes each date whole.
    fn shared_years(self, first: &CalendarDate, second: &CalendarDate) -> Option<(i32, i32)> {
        let bare = |date: &CalendarDate| {
            date.calendar() == Calendar::Gregorian && date.month().is_none() && date.year() > 0
        };
        (self.locale == Locale::Russian && bare(first) && bare(second))
            .then(|| (first.year(), second.year()))
    }

    /// The name of a month, in the case asked for.
    fn month_name(self, date: &CalendarDate, month: u8, case: Case) -> String {
        let index = usize::from(month.saturating_sub(1));
        match date.calendar() {
            Calendar::Gregorian | Calendar::Julian => {
                let table = match (self.locale, case) {
                    (Locale::Russian, Case::Nominative) => &RU_MONTHS[0],
                    (Locale::Russian, Case::Genitive) => &RU_MONTHS[1],
                    (Locale::Russian, Case::Instrumental) => &RU_MONTHS[2],
                    (Locale::English, _) => &EN_MONTHS,
                    (Locale::German, _) => &DE_MONTHS,
                };
                table.get(index).copied().unwrap_or("?").to_owned()
            }
            Calendar::FrenchRepublican => {
                if month == 13 {
                    return match (self.locale, case) {
                        (Locale::Russian, Case::Nominative) => "дополнительные дни",
                        (Locale::Russian, Case::Genitive) => "дополнительных дней",
                        (Locale::Russian, Case::Instrumental) => "дополнительными днями",
                        (Locale::English, _) => "complementary days,",
                        (Locale::German, _) => "Ergänzungstage",
                    }
                    .to_owned();
                }
                let table = match (self.locale, case) {
                    (Locale::Russian, Case::Nominative) => &RU_REPUBLICAN[0],
                    (Locale::Russian, Case::Genitive) => &RU_REPUBLICAN[1],
                    (Locale::Russian, Case::Instrumental) => &RU_REPUBLICAN[2],
                    (Locale::English | Locale::German, _) => &REPUBLICAN,
                };
                table.get(index).copied().unwrap_or("?").to_owned()
            }
            Calendar::Hebrew => hebrew_month_name(self.locale, date.year(), month, case),
        }
    }

    fn words(self) -> &'static Words {
        match self.locale {
            Locale::Russian => &RU_WORDS,
            Locale::English => &EN_WORDS,
            Locale::German => &DE_WORDS,
        }
    }
}

/// The words around a date, per locale.
struct Words {
    about: &'static str,
    calculated: &'static str,
    estimated: &'static str,
    before: &'static str,
    after: &'static str,
    between: &'static str,
    and: &'static str,
    from: &'static str,
    to: &'static str,
    since: &'static str,
    until: &'static str,
    /// The shared abbreviation after two bare years, with its leading space.
    years: &'static str,
    old_style: &'static str,
    unknown: &'static str,
    open_quote: &'static str,
    close_quote: &'static str,
}

const RU_WORDS: Words = Words {
    about: "около",
    // "по расчёту" and "предположительно" are adverbial, so the month after them stays in the
    // nominative: "по расчёту апрель 1871 г."
    calculated: "по расчёту",
    estimated: "предположительно",
    before: "до",
    after: "после",
    between: "между",
    and: "и",
    from: "с",
    to: "по",
    since: "с",
    until: "по",
    years: " гг.",
    old_style: "(ст. ст.)",
    unknown: "дата неизвестна",
    open_quote: "«",
    close_quote: "»",
};

const EN_WORDS: Words = Words {
    about: "about",
    calculated: "calculated",
    estimated: "estimated",
    before: "before",
    after: "after",
    between: "between",
    and: "and",
    from: "from",
    to: "to",
    since: "from",
    until: "until",
    years: "",
    old_style: "(O.S.)",
    unknown: "date unknown",
    open_quote: "\u{201C}",
    close_quote: "\u{201D}",
};

const DE_WORDS: Words = Words {
    // "um 1850" is the established form in German genealogical writing.
    about: "um",
    calculated: "berechnet",
    estimated: "geschätzt",
    before: "vor",
    after: "nach",
    between: "zwischen",
    and: "und",
    from: "von",
    to: "bis",
    since: "ab",
    until: "bis",
    years: "",
    old_style: "(a. St.)",
    unknown: "Datum unbekannt",
    open_quote: "\u{201E}",
    close_quote: "\u{201C}",
};

/// Russian month names: nominative, genitive, instrumental.
const RU_MONTHS: [[&str; 12]; 3] = [
    [
        "январь",
        "февраль",
        "март",
        "апрель",
        "май",
        "июнь",
        "июль",
        "август",
        "сентябрь",
        "октябрь",
        "ноябрь",
        "декабрь",
    ],
    [
        "января",
        "февраля",
        "марта",
        "апреля",
        "мая",
        "июня",
        "июля",
        "августа",
        "сентября",
        "октября",
        "ноября",
        "декабря",
    ],
    [
        "январём",
        "февралём",
        "мартом",
        "апрелем",
        "маем",
        "июнем",
        "июлем",
        "августом",
        "сентябрём",
        "октябрём",
        "ноябрём",
        "декабрём",
    ],
];

const EN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const DE_MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

/// The Republican months as English and German write them — in French, as the decree named them.
const REPUBLICAN: [&str; 12] = [
    "Vendémiaire",
    "Brumaire",
    "Frimaire",
    "Nivôse",
    "Pluviôse",
    "Ventôse",
    "Germinal",
    "Floréal",
    "Prairial",
    "Messidor",
    "Thermidor",
    "Fructidor",
];

/// The Republican months in Russian: nominative, genitive, instrumental.
const RU_REPUBLICAN: [[&str; 12]; 3] = [
    [
        "вандемьер",
        "брюмер",
        "фример",
        "нивоз",
        "плювиоз",
        "вантоз",
        "жерминаль",
        "флореаль",
        "прериаль",
        "мессидор",
        "термидор",
        "фрюктидор",
    ],
    [
        "вандемьера",
        "брюмера",
        "фримера",
        "нивоза",
        "плювиоза",
        "вантоза",
        "жерминаля",
        "флореаля",
        "прериаля",
        "мессидора",
        "термидора",
        "фрюктидора",
    ],
    [
        "вандемьером",
        "брюмером",
        "фримером",
        "нивозом",
        "плювиозом",
        "вантозом",
        "жерминалем",
        "флореалем",
        "прериалем",
        "мессидором",
        "термидором",
        "фрюктидором",
    ],
];

/// A Hebrew month's name, which for Adar depends on whether the year has two.
fn hebrew_month_name(locale: Locale, year: i32, month: u8, case: Case) -> String {
    let leap = crate::hebrew::is_leap_year(year);
    let Some(tag) = month_tag(Calendar::Hebrew, year, month) else {
        return "?".to_owned();
    };

    // ADR is Adar I. ADS is Adar II in a leap year and plain Adar otherwise.
    let (russian, english, german) = match tag {
        "TSH" => (["тишрей", "тишрея", "тишреем"], "Tishrei", "Tischri"),
        "CSH" => (["хешван", "хешвана", "хешваном"], "Cheshvan", "Cheschwan"),
        "KSL" => (["кислев", "кислева", "кислевом"], "Kislev", "Kislew"),
        "TVT" => (["тевет", "тевета", "теветом"], "Tevet", "Tewet"),
        "SHV" => (["шват", "швата", "шватом"], "Shevat", "Schewat"),
        "ADR" => (["адар I", "адара I", "адаром I"], "Adar I", "Adar I"),
        "ADS" if leap => (["адар II", "адара II", "адаром II"], "Adar II", "Adar II"),
        "ADS" => (["адар", "адара", "адаром"], "Adar", "Adar"),
        "NSN" => (["нисан", "нисана", "нисаном"], "Nisan", "Nisan"),
        "IYR" => (["ияр", "ияра", "ияром"], "Iyar", "Ijar"),
        "SVN" => (["сиван", "сивана", "сиваном"], "Sivan", "Siwan"),
        "TMZ" => (["таммуз", "таммуза", "таммузом"], "Tammuz", "Tammus"),
        "AAV" => (["ав", "ава", "авом"], "Av", "Aw"),
        _ => (["элул", "элула", "элулом"], "Elul", "Elul"),
    };

    match locale {
        Locale::Russian => match case {
            Case::Nominative => russian[0],
            Case::Genitive => russian[1],
            Case::Instrumental => russian[2],
        }
        .to_owned(),
        Locale::English => english.to_owned(),
        Locale::German => german.to_owned(),
    }
}

/// A Roman numeral, for the Republican years.
fn roman(number: i32) -> String {
    const NUMERALS: [(i32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    if number < 1 {
        return number.to_string();
    }
    let mut remaining = number;
    let mut out = String::new();
    for (value, numeral) in NUMERALS {
        while remaining >= value {
            out.push_str(numeral);
            remaining -= value;
        }
    }
    out
}

/// An English ordinal: 1st, 2nd, 3rd, 4th, 11th, 21st.
fn ordinal(number: u8) -> String {
    let suffix = match (number % 10, number % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{number}{suffix}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{CalendarDisplay, DateFormat, Locale, ordinal, roman};
    use crate::value::{DateValue, RecordedDate};
    use crate::{Calendar, CalendarDate};

    const RU: DateFormat = DateFormat::new(Locale::Russian);
    const EN: DateFormat = DateFormat::new(Locale::English);
    const DE: DateFormat = DateFormat::new(Locale::German);

    fn value(payload: &str) -> DateValue {
        DateValue::parse_gedcom7(payload).unwrap().unwrap()
    }

    fn date(calendar: Calendar, year: i32, month: Option<u8>, day: Option<u8>) -> CalendarDate {
        CalendarDate::new(calendar, year, month, day).unwrap()
    }

    #[test]
    fn the_three_precisions_in_three_languages() {
        let full = value("17 APR 1871");
        assert_eq!(RU.value(&full), "17 апреля 1871 г.");
        assert_eq!(EN.value(&full), "17 April 1871");
        assert_eq!(DE.value(&full), "17. April 1871");

        let month = value("APR 1871");
        assert_eq!(RU.value(&month), "апрель 1871 г.");
        assert_eq!(EN.value(&month), "April 1871");
        assert_eq!(DE.value(&month), "April 1871");

        let year = value("1871");
        assert_eq!(RU.value(&year), "1871 г.");
        assert_eq!(EN.value(&year), "1871");
    }

    #[test]
    fn the_october_revolution_in_both_styles() {
        let revolution = value("JULIAN 25 OCT 1917");
        assert_eq!(RU.value(&revolution), "25 октября (7 ноября) 1917 г.");
        assert_eq!(EN.value(&revolution), "25 October (7 November) 1917");
        assert_eq!(DE.value(&revolution), "25. Oktober (7. November) 1917");
    }

    #[test]
    fn a_pair_across_new_year_is_written_whole() {
        // Julian 20 December 1917 is Gregorian 2 January 1918: the years differ, so each side
        // carries its own.
        let christmas = value("JULIAN 20 DEC 1917");
        assert_eq!(
            RU.value(&christmas),
            "20 декабря 1917 г. (2 января 1918 г.)"
        );
        assert_eq!(EN.value(&christmas), "20 December 1917 (2 January 1918)");
    }

    #[test]
    fn a_julian_date_that_cannot_be_converted_says_it_is_julian() {
        let partial = value("JULIAN APR 1871");
        assert_eq!(RU.value(&partial), "апрель 1871 г. (ст. ст.)");
        assert_eq!(EN.value(&partial), "April 1871 (O.S.)");
        assert_eq!(DE.value(&partial), "April 1871 (a. St.)");
    }

    #[test]
    fn the_other_display_choices() {
        let revolution = value("JULIAN 25 OCT 1917");
        let converted = DateFormat {
            locale: Locale::Russian,
            calendar: CalendarDisplay::Gregorian,
        };
        assert_eq!(converted.value(&revolution), "7 ноября 1917 г.");
        let recorded = DateFormat {
            locale: Locale::Russian,
            calendar: CalendarDisplay::AsRecorded,
        };
        assert_eq!(recorded.value(&revolution), "25 октября 1917 г. (ст. ст.)");
    }

    #[test]
    fn russian_months_take_the_case_their_preposition_demands() {
        assert_eq!(RU.value(&value("ABT APR 1871")), "около апреля 1871 г.");
        assert_eq!(RU.value(&value("BEF APR 1875")), "до апреля 1875 г.");
        assert_eq!(RU.value(&value("AFT APR 1869")), "после апреля 1869 г.");
        assert_eq!(
            RU.value(&value("BET APR 1869 AND MAY 1873")),
            "между апрелем 1869 г. и маем 1873 г."
        );
        assert_eq!(
            RU.value(&value("FROM APR 1902 TO MAY 1914")),
            "с апреля 1902 г. по май 1914 г."
        );
        // The adverbial hedges leave the month alone.
        assert_eq!(
            RU.value(&value("CAL APR 1871")),
            "по расчёту апрель 1871 г."
        );
        // With a day the month is genitive whatever surrounds it.
        assert_eq!(
            RU.value(&value("BET 17 APR 1869 AND 3 MAY 1873")),
            "между 17 апреля 1869 г. и 3 мая 1873 г."
        );
    }

    #[test]
    fn two_bare_years_share_one_abbreviation() {
        assert_eq!(
            RU.value(&value("BET 1869 AND 1873")),
            "между 1869 и 1873 гг."
        );
        assert_eq!(RU.value(&value("FROM 1902 TO 1914")), "с 1902 по 1914 гг.");
        assert_eq!(
            EN.value(&value("BET 1869 AND 1873")),
            "between 1869 and 1873"
        );
        assert_eq!(DE.value(&value("FROM 1902 TO 1914")), "von 1902 bis 1914");
    }

    #[test]
    fn every_form_has_words_in_every_language() {
        assert_eq!(EN.value(&value("ABT 1871")), "about 1871");
        assert_eq!(EN.value(&value("EST 1871")), "estimated 1871");
        assert_eq!(EN.value(&value("FROM 1902")), "from 1902");
        assert_eq!(EN.value(&value("TO 1914")), "until 1914");
        assert_eq!(DE.value(&value("ABT 1871")), "um 1871");
        assert_eq!(DE.value(&value("FROM 1902")), "ab 1902");
        assert_eq!(DE.value(&value("TO 1914")), "bis 1914");
        assert_eq!(DE.value(&value("BEF 1875")), "vor 1875");
        assert_eq!(RU.value(&value("FROM 1902")), "с 1902 г.");
        assert_eq!(RU.value(&value("TO 1914")), "по 1914 г.");
    }

    #[test]
    fn years_before_the_era() {
        let ides = value("15 MAR 44 BCE");
        assert_eq!(RU.value(&ides), "15 марта 44 г. до н. э.");
        assert_eq!(EN.value(&ides), "15 March 44 BCE");
        assert_eq!(DE.value(&ides), "15. März 44 v. Chr.");
    }

    #[test]
    fn hebrew_adar_is_named_by_the_year() {
        let common = date(Calendar::Hebrew, 5785, Some(6), None);
        let leap_first = date(Calendar::Hebrew, 5784, Some(6), None);
        let leap_second = date(Calendar::Hebrew, 5784, Some(7), None);
        assert_eq!(EN.date(&common), "Adar 5785");
        assert_eq!(EN.date(&leap_first), "Adar I 5784");
        assert_eq!(EN.date(&leap_second), "Adar II 5784");
        assert_eq!(RU.date(&common), "адар 5785 г.");
    }

    #[test]
    fn a_hebrew_date_is_paired_with_its_gregorian_day() {
        let passover = date(Calendar::Hebrew, 5785, Some(7), Some(15));
        assert_eq!(RU.date(&passover), "15 нисана 5785 г. (13 апреля 2025 г.)");
        assert_eq!(EN.date(&passover), "15 Nisan 5785 (13 April 2025)");
    }

    #[test]
    fn republican_years_are_roman() {
        let thermidor = date(Calendar::FrenchRepublican, 2, Some(11), Some(9));
        let as_written = DateFormat {
            locale: Locale::Russian,
            calendar: CalendarDisplay::AsRecorded,
        };
        assert_eq!(as_written.date(&thermidor), "9 термидора II года");
        assert_eq!(
            DateFormat {
                locale: Locale::English,
                calendar: CalendarDisplay::AsRecorded
            }
            .date(&thermidor),
            "9 Thermidor Year II"
        );
        assert_eq!(RU.date(&thermidor), "9 термидора II года (27 июля 1794 г.)");
        assert_eq!(
            RU.date(&date(Calendar::FrenchRepublican, 3, None, None)),
            "III год"
        );
        assert_eq!(
            RU.value(&value("BET FRENCH_R 2 AND FRENCH_R 4")),
            "между II годом и IV годом"
        );
    }

    #[test]
    fn the_complementary_days_are_numbered_days() {
        let last = date(Calendar::FrenchRepublican, 3, Some(13), Some(6));
        let written = DateFormat {
            locale: Locale::Russian,
            calendar: CalendarDisplay::AsRecorded,
        };
        assert_eq!(written.date(&last), "6-й дополнительный день III года");
        assert_eq!(
            DateFormat {
                locale: Locale::English,
                calendar: CalendarDisplay::AsRecorded
            }
            .date(&last),
            "6th complementary day, Year III"
        );
    }

    #[test]
    fn a_recorded_date_falls_back_to_its_words_then_to_saying_it_is_unknown() {
        let phrase = RecordedDate::from_phrase("во время войны");
        assert_eq!(RU.recorded(&phrase), "«во время войны»");
        assert_eq!(
            EN.recorded(&RecordedDate::from_phrase("during the war")),
            "\u{201C}during the war\u{201D}"
        );
        assert_eq!(
            DE.recorded(&RecordedDate::from_phrase("im Krieg")),
            "\u{201E}im Krieg\u{201C}"
        );
        assert_eq!(RU.recorded(&RecordedDate::default()), "дата неизвестна");
        assert_eq!(
            RU.recorded(&RecordedDate::from_value(value("1871"))),
            "1871 г."
        );
    }

    #[test]
    fn locale_tags_are_read_by_their_language() {
        assert_eq!(Locale::from_tag("ru-RU"), Some(Locale::Russian));
        assert_eq!(Locale::from_tag("de_AT"), Some(Locale::German));
        assert_eq!(Locale::from_tag("EN"), Some(Locale::English));
        assert_eq!(Locale::from_tag("fr"), None);
    }

    #[test]
    fn numerals_and_ordinals() {
        assert_eq!(roman(2), "II");
        assert_eq!(roman(14), "XIV");
        assert_eq!(roman(1999), "MCMXCIX");
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(22), "22nd");
    }
}
