//! ANSEL, with its diacritics put where Unicode wants them.
//!
//! In ANSEL a diacritic is a byte of its own written *before* the letter it goes on: `ü` is 0xE8,
//! the diaeresis, then `u`. Unicode puts combining marks *after* their letter, and prefers the
//! precomposed letter where one exists. So the decoder holds each run of diacritics until the
//! letter arrives, sets them after it in canonical order, and composes what Unicode composes —
//! the result normalisation to NFC would give, computed only where it is needed.
//!
//! Two cases are particular to GEDCOM:
//!
//! - **`CONC` can split a value anywhere**, including between a diacritic and its letter. A
//!   diacritic that ends a line goes on the first character of the `CONC` line after it.
//! - **A diacritic with no letter at all** stays, on a no-break space, and is reported. On a plain
//!   space it would join the words either side; dropped, it would be lost.
//!
//! Checked against H. Eichmann's 1999 test file, which writes every ANSEL character and every
//! diacritic on every Latin letter, beside his Unicode rendering of the same tree. The two agree
//! except where his table predates the Library of Congress's: he read the underscore, 0xF6, as
//! U+0331 macron below, which composes into `ḇ`, `ḏ` and the like; the Library maps it to U+0332
//! low line, which composes into nothing. This decoder follows the Library.

use super::tables::{ANSEL, COMBINING_CLASSES, COMPOSITIONS};
use super::{FlawKind, Flaws};

/// Decodes ANSEL onto the end of `text`. Returns how many diacritics were composed into a letter.
pub(super) fn decode(bytes: &[u8], text: &mut String, flaws: &mut Flaws) -> usize {
    let mut cluster = Cluster::default();
    let mut composed = 0;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(mark) = mark(byte) {
            cluster.marks.push(mark);
            index += 1;
            continue;
        }
        if !cluster.marks.is_empty() && matches!(byte, b'\n' | b'\r') {
            if let Some(value) = continuation(bytes, index) {
                // The line break and the CONC prefix are ASCII; they pass through while the
                // diacritics wait for the value.
                text.extend(bytes[index..value].iter().copied().map(char::from));
                index = value;
                continue;
            }
            cluster.lone(text, flaws);
        }
        let base = if byte.is_ascii() {
            char::from(byte)
        } else if let Some(character) = spacing(byte) {
            character
        } else {
            flaws.note(text.len(), FlawKind::Unmapped(byte));
            char::REPLACEMENT_CHARACTER
        };
        composed += cluster.close(base, text);
        index += 1;
    }
    cluster.lone(text, flaws);
    composed
}

/// The diacritics waiting for their letter, and room to sort them.
#[derive(Default)]
struct Cluster {
    marks: Vec<char>,
    apart: Vec<char>,
}

impl Cluster {
    /// Writes `base` with the waiting diacritics, composed as far as Unicode composes them.
    /// Returns how many were composed.
    ///
    /// UAX #15's algorithm, for one starter: marks in canonical order — a stable sort by combining
    /// class — and each composed with the starter unless a mark already kept apart has the same
    /// class or higher, which blocks it.
    fn close(&mut self, base: char, text: &mut String) -> usize {
        if self.marks.is_empty() {
            text.push(base);
            return 0;
        }
        self.marks.sort_by_key(|&mark| combining_class(mark));

        let mut starter = base;
        let mut composed = 0;
        let mut blocking = 0;
        self.apart.clear();
        for &mark in &self.marks {
            let class = combining_class(mark);
            if blocking < class
                && let Some(composite) = compose(starter, mark)
            {
                starter = composite;
                composed += 1;
            } else {
                self.apart.push(mark);
                blocking = class;
            }
        }
        text.push(starter);
        text.extend(&self.apart);
        self.marks.clear();
        composed
    }

    /// Diacritics that have no letter to go on: kept on a no-break space, and reported.
    fn lone(&mut self, text: &mut String, flaws: &mut Flaws) {
        if self.marks.is_empty() {
            return;
        }
        for &mark in &self.marks {
            flaws.note(text.len(), FlawKind::LoneMark(mark));
        }
        self.close('\u{A0}', text);
    }
}

/// The diacritic a byte stands for, if it is one.
fn mark(byte: u8) -> Option<char> {
    (byte >= 0xE0).then(|| upper_half(byte)).flatten()
}

/// The spacing character a byte from 0x80 to 0xDF stands for, if any.
fn spacing(byte: u8) -> Option<char> {
    (0x80..0xE0)
        .contains(&byte)
        .then(|| upper_half(byte))
        .flatten()
}

fn upper_half(byte: u8) -> Option<char> {
    char::from_u32(u32::from(ANSEL[usize::from(byte - 0x80)])).filter(|&c| c != '\0')
}

fn combining_class(mark: char) -> u8 {
    let Ok(code) = u16::try_from(u32::from(mark)) else {
        return 0;
    };
    COMBINING_CLASSES
        .binary_search_by_key(&code, |&(mark, _)| mark)
        .map_or(0, |found| COMBINING_CLASSES[found].1)
}

fn compose(starter: char, mark: char) -> Option<char> {
    let key = (
        u16::try_from(u32::from(starter)).ok()?,
        u16::try_from(u32::from(mark)).ok()?,
    );
    let found = COMPOSITIONS
        .binary_search_by_key(&key, |&(starter, mark, _)| (starter, mark))
        .ok()?;
    char::from_u32(u32::from(COMPOSITIONS[found].2))
}

/// Where the value of a `CONC` line begins, if the line break at `index` is followed by one with
/// something in it. 5.5.1 allowed whitespace before the level, so this does too.
fn continuation(bytes: &[u8], index: usize) -> Option<usize> {
    let terminator = if bytes[index..].starts_with(b"\r\n") {
        2
    } else {
        1
    };
    let mut at = index + terminator;
    at += count(&bytes[at..], |b| b == b' ' || b == b'\t');
    let digits = count(&bytes[at..], |b| b.is_ascii_digit());
    if digits == 0 {
        return None;
    }
    at += digits;
    let spaces = count(&bytes[at..], |b| b == b' ');
    if spaces == 0 || !bytes[at + spaces..].starts_with(b"CONC ") {
        return None;
    }
    at += spaces + b"CONC ".len();
    match bytes.get(at) {
        Some(b'\n' | b'\r') | None => None,
        Some(_) => Some(at),
    }
}

fn count(bytes: &[u8], wanted: impl Fn(u8) -> bool) -> usize {
    bytes.iter().take_while(|&&b| wanted(b)).count()
}
