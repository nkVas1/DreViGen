//! GEDCOM's `NAME` into the archive's name parts.
//!
//! GEDCOM writes a name as one value with the surname between slashes — `Иван Петрович /Смирнов/`
//! — and may add its parts as substructures. Where the file gives a part, it wins; where it does
//! not, the part is read from the value.
//!
//! GEDCOM has no patronymic. Russian programs write it into the given name, and this model keeps
//! it apart, because a given name of *Иван Петрович* would never agree with a later record giving
//! *Иван* and *Петрович* separately. So a given name's last word comes out as the patronymic when
//! it can be nothing else:
//!
//! - **The full forms** — *-ович*, *-евич*, *-ич*; *-овна*, *-евна*, *-ична* — always. No Russian
//!   given name ends that way, and pre-reform spellings such as *Петровичъ* fold to the same.
//! - **The short forms** of revision lists and parish registers — *Иван Петров*, *Марья Петрова*,
//!   which wrote a peasant's patronymic like a surname — only when the file marks a surname
//!   separately. Without one, *Петров* may well be the surname.
//!
//! Only Cyrillic is read this way. A transliteration such as *Iwan Petrowitsch* stays a given name:
//! it still agrees with itself, and guessing at every alphabet's spelling of every suffix would be
//! wrong more often than it helped.

use drevigen_core::{NameKind, PersonName, fold};
use drevigen_gedcom::Structure;

/// The substructures of `NAME` that become parts of the name. `TYPE` is read separately, because
/// a type the model cannot express is kept rather than read.
pub(super) const PARTS: [&str; 6] = ["NPFX", "GIVN", "NICK", "SPFX", "SURN", "NSFX"];

/// The name a `NAME` structure records.
pub(super) fn person_name(name: &Structure) -> PersonName {
    let value = name.payload.text().unwrap_or("");
    let (before, between, after) = split_slashes(value);
    let part = |tag: &str| {
        name.child(tag)
            .and_then(|part| part.payload.text())
            .and_then(non_empty)
    };

    let prefix = part("NPFX");
    let given = part("GIVN").or_else(|| {
        // "Dr. John /Smith/" with NPFX "Dr.": the value repeats the prefix before the given name.
        let before = match &prefix {
            Some(prefix) => before
                .trim_start()
                .strip_prefix(prefix.as_str())
                .unwrap_or(before),
            None => before,
        };
        non_empty(before)
    });
    let mut parsed = PersonName {
        kind: name.child("TYPE").and_then(kind),
        prefix,
        given,
        patronymic: None,
        surname_prefix: part("SPFX"),
        surname: part("SURN").or_else(|| non_empty(between)),
        suffix: part("NSFX").or_else(|| non_empty(after)),
        nickname: part("NICK"),
    };
    split_patronymic(&mut parsed);
    parsed
}

/// The kind a `TYPE` structure names, when it names one of GEDCOM's and carries nothing more. A
/// 5.5.1 file may write any word there, and 7.0 a `PHRASE` beneath `OTHER`; either says more than
/// the model can hold, and the structure is kept instead.
pub(super) fn kind(kind: &Structure) -> Option<NameKind> {
    if !kind.children.is_empty() {
        return None;
    }
    match kind.payload.text()?.trim().to_ascii_uppercase().as_str() {
        "BIRTH" => Some(NameKind::Birth),
        "MAIDEN" => Some(NameKind::Maiden),
        "MARRIED" => Some(NameKind::Married),
        "IMMIGRANT" => Some(NameKind::Immigrant),
        "PROFESSIONAL" => Some(NameKind::Professional),
        "AKA" => Some(NameKind::AlsoKnownAs),
        _ => None,
    }
}

/// The value before, between and after the first two slashes.
fn split_slashes(value: &str) -> (&str, &str, &str) {
    let Some((before, rest)) = value.split_once('/') else {
        return (value, "", "");
    };
    match rest.split_once('/') {
        Some((between, after)) => (before, between, after),
        // An unclosed slash: everything after it is the surname, as most programs read it.
        None => (before, rest, ""),
    }
}

fn non_empty(text: &str) -> Option<String> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

fn split_patronymic(name: &mut PersonName) {
    let Some((first, last)) = name
        .given
        .as_deref()
        .and_then(|given| given.rsplit_once(' '))
    else {
        return;
    };
    if !last.starts_with(|c: char| c.is_uppercase() && is_cyrillic(c)) {
        return;
    }
    let folded = fold(last);
    let full = ["ич", "вна", "чна"].iter().any(|end| folded.ends_with(end));
    let short = name.surname.is_some()
        && ["ов", "ев", "ова", "ева"]
            .iter()
            .any(|end| folded.ends_with(end));
    if full || short {
        let (given, patronymic) = (first.to_owned(), last.to_owned());
        name.given = Some(given);
        name.patronymic = Some(patronymic);
    }
}

const fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{052F}')
}
