//! Weighing candidate encodings against what the file says.
//!
//! Each candidate decodes the same sample of lines, and the decoding is scored for how unlike
//! writing it is. The costs are few and coarse on purpose — they only have to separate a right
//! reading from a wrong one, and a wrong reading is wrong everywhere:
//!
//! - **A byte that stands for nothing** is the strongest evidence there is.
//! - **A word that mixes scripts** — `Mьller`, windows-1252's `ü` read as windows-1251.
//! - **A run of accented Latin letters** — `Èâàí`, windows-1251's `Иван` read as windows-1252.
//!   Western languages put one accented letter between plain ones; a whole word of them is
//!   another alphabet misread.
//! - **A symbol glued to a letter** — `╚трэ`, windows-1251 read as IBM866.
//! - **A diacritic with no letter under it** — what other encodings' letters become in ANSEL.
//! - **A capital after a small letter** — the case of KOI8-style misreadings, and mild, because
//!   `McDonald` exists.
//!
//! Against these, a diacritic that composes with the letter after it is evidence *for* ANSEL, and
//! the declared encoding starts with an advantage: a declaration is usually right, so it takes
//! several words of contrary evidence to overrule one.

use super::{Encoding, Flaws, decode_into};

/// The encodings weighed, in the order that settles a tie. UTF-16 is not among them: it shows
/// itself before anything is weighed.
const CANDIDATES: [Encoding; 9] = [
    Encoding::Utf8,
    Encoding::Windows1251,
    Encoding::Windows1252,
    Encoding::Ansel,
    Encoding::Windows1250,
    Encoding::Ibm866,
    Encoding::Ibm437,
    Encoding::Ibm850,
    Encoding::MacRoman,
];

/// How many lines with non-ASCII bytes are sampled, spread evenly through the file. Enough to
/// decide on, few enough that weighing a large file costs little more than reading it once.
const SAMPLE_LINES: usize = 2048;

/// The head start a declared encoding gets.
const DECLARED: i64 = 8;
/// A character that cannot be read: U+FFFD for a byte that stands for nothing, or a control code.
const UNREADABLE: i64 = 20;
/// Letters of two scripts side by side.
const MIXED_SCRIPTS: i64 = 5;
/// A diacritic on a space, a digit, punctuation — on anything but a letter.
const MARK_WITHOUT_LETTER: i64 = 5;
/// A symbol next to a letter, counted for each side.
const GLUED_SYMBOL: i64 = 3;
/// Punctuation with a letter on both sides.
const GLUED_PUNCTUATION: i64 = 2;
/// A diacritic on a letter it does not compose with.
const LOOSE_MARK: i64 = 1;
/// Two accented Latin letters side by side.
const ACCENT_RUN: i64 = 1;
/// A capital after a small letter.
const CAPITAL_INSIDE: i64 = 1;

/// Chooses the encoding the bytes are most likely in, giving the declared ones their head start.
pub(super) fn weigh(bytes: &[u8], declared: &[Encoding]) -> Encoding {
    // The first of equal minimums, so the candidates' order settles a tie.
    scores(bytes, declared)
        .into_iter()
        .min_by_key(|&(_, score)| score)
        .map_or(Encoding::Utf8, |(encoding, _)| encoding)
}

/// Every candidate's score, lower being likelier, in the candidates' order.
pub(super) fn scores(bytes: &[u8], declared: &[Encoding]) -> [(Encoding, i64); CANDIDATES.len()] {
    let sample = sample(bytes);
    let mut text = String::new();
    CANDIDATES.map(|candidate| {
        text.clear();
        let mut flaws = Flaws::default();
        let mut composed = 0;
        for line in &sample {
            composed += decode_into(line, candidate, &mut text, &mut flaws);
            text.push('\n');
        }
        let composed = i64::try_from(composed).unwrap_or(i64::MAX);
        let advantage = DECLARED * i64::from(declared.contains(&candidate));
        (candidate, judge(&text).saturating_sub(composed) - advantage)
    })
}

/// The lines that have a byte beyond ASCII — the only ones whose reading depends on the
/// encoding — every so many of them, so the sample spans the whole file.
fn sample(bytes: &[u8]) -> Vec<&[u8]> {
    let lines = || {
        bytes
            .split(|&b| b == b'\n' || b == b'\r')
            .filter(|line| !line.is_ascii())
    };
    let step = lines().count().div_ceil(SAMPLE_LINES).max(1);
    lines().step_by(step).collect()
}

/// How unlike writing a decoded text is. Zero for anything a person would have typed.
fn judge(text: &str) -> i64 {
    let mut classes = text.chars().map(classify).peekable();
    // The last character that is not a diacritic: what a diacritic sits on, and what a letter
    // follows.
    let mut previous = Class::Other;
    let mut score = 0;
    while let Some(current) = classes.next() {
        let next = classes.peek().copied().unwrap_or(Class::Other);
        score += cost(previous, current, next);
        if !matches!(current, Class::Mark) {
            previous = current;
        }
    }
    score
}

fn cost(previous: Class, current: Class, next: Class) -> i64 {
    match current {
        Class::Unreadable => UNREADABLE,
        Class::Symbol => {
            GLUED_SYMBOL * (i64::from(previous.is_letter()) + i64::from(next.is_letter()))
        }
        Class::Punctuation if previous.is_letter() && next.is_letter() => GLUED_PUNCTUATION,
        Class::Mark if previous.is_letter() => LOOSE_MARK,
        Class::Mark => MARK_WITHOUT_LETTER,
        Class::Letter(after) => match previous {
            Class::Letter(before) => pair(before, after),
            _ => 0,
        },
        _ => 0,
    }
}

fn pair(before: Letter, after: Letter) -> i64 {
    if before.ascii && after.ascii {
        return 0;
    }
    let mut cost = 0;
    if before.script != after.script
        && before.script != Script::Neutral
        && after.script != Script::Neutral
    {
        cost += MIXED_SCRIPTS;
    }
    let accented = |letter: Letter| !letter.ascii && letter.script == Script::Latin;
    if accented(before) && accented(after) {
        cost += ACCENT_RUN;
    }
    if before.lower && after.upper {
        cost += CAPITAL_INSIDE;
    }
    cost
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Script {
    Latin,
    Cyrillic,
    Greek,
    /// Letters that belong with any script: `ª`, the modifier letters `ʹ` and `ʺ` that romanised
    /// Russian uses for its soft and hard signs.
    Neutral,
}

#[derive(Debug, Clone, Copy)]
struct Letter {
    script: Script,
    upper: bool,
    lower: bool,
    ascii: bool,
}

#[derive(Debug, Clone, Copy)]
enum Class {
    Letter(Letter),
    Mark,
    Punctuation,
    Symbol,
    Unreadable,
    /// Spaces, digits, ASCII punctuation, and the few marks that belong inside words.
    Other,
}

impl Class {
    const fn is_letter(self) -> bool {
        matches!(self, Self::Letter(_))
    }
}

fn classify(c: char) -> Class {
    if c.is_ascii() {
        return if c.is_ascii_alphabetic() {
            Class::Letter(Letter {
                script: Script::Latin,
                upper: c.is_ascii_uppercase(),
                lower: c.is_ascii_lowercase(),
                ascii: true,
            })
        } else {
            Class::Other
        };
    }
    match c {
        // With U+FFFD, GEDCOM 5.5's slash-through diacritic: next to never used in a real ANSEL
        // file, and exactly what windows-1252's ü becomes when read as ANSEL.
        char::REPLACEMENT_CHARACTER | '\u{0338}' => Class::Unreadable,
        '\u{0300}'..='\u{036F}'
        | '\u{1AB0}'..='\u{1AFF}'
        | '\u{1DC0}'..='\u{1DFF}'
        | '\u{20D0}'..='\u{20FF}'
        | '\u{FE20}'..='\u{FE2F}' => Class::Mark,
        // What sits inside words: the soft hyphen, hyphens, the typographic apostrophe, the
        // joiners.
        '\u{00AD}' | '\u{2010}' | '\u{2011}' | '\u{2019}' | '\u{200C}' | '\u{200D}' => Class::Other,
        '\u{00A1}'
        | '\u{00A7}'
        | '\u{00AB}'
        | '\u{00B6}'
        | '\u{00B7}'
        | '\u{00BB}'
        | '\u{00BF}'
        | '\u{2012}'..='\u{2018}'
        | '\u{201A}'..='\u{2027}'
        | '\u{2030}'..='\u{205E}' => Class::Punctuation,
        // Letters by Unicode's category and symbols in use — the ordinal indicators, the micro
        // sign, the florin, the litre sign — and what Mac OS Roman's ß and ü become in IBM437.
        '\u{00AA}' | '\u{00B5}' | '\u{00BA}' | '\u{0192}' | '\u{2113}' => Class::Symbol,
        _ if c.is_control() => Class::Unreadable,
        _ if c.is_whitespace() => Class::Other,
        _ if c.is_alphabetic() => Class::Letter(Letter {
            script: script(c),
            upper: c.is_uppercase(),
            lower: c.is_lowercase(),
            ascii: false,
        }),
        _ => Class::Symbol,
    }
}

const fn script(c: char) -> Script {
    match c {
        'A'..='Z' | 'a'..='z' | '\u{00C0}'..='\u{02AF}' | '\u{1E00}'..='\u{1EFF}' => Script::Latin,
        '\u{0370}'..='\u{03FF}' | '\u{1F00}'..='\u{1FFF}' => Script::Greek,
        '\u{0400}'..='\u{052F}' => Script::Cyrillic,
        _ => Script::Neutral,
    }
}
