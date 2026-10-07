//! Bytes into text: which character encoding a GEDCOM file is in, and the decoding itself.
//!
//! GEDCOM 7 is UTF-8 and nothing else. Older files are not so simple. 5.5.1 allowed ANSEL,
//! UTF-8, `UNICODE` (UTF-16) and ASCII, and programs wrote whatever their platform used besides:
//! Russian software windows-1251 under the vague `CHAR ANSI`, DOS software a code page under
//! `CHAR IBMPC` — which 5.5.1 forbids, because such a file "cannot be interpreted properly without
//! knowing which code page the sender was using" — and old Macs Mac OS Roman. Nor is the
//! declaration always true: there are files that say ANSEL and are windows-1252, and files that
//! say UTF-8 and are not.
//!
//! So the declaration is evidence, not law. [`decode`] settles the encoding in this order, and
//! [`Decoded::basis`] says which step did:
//!
//! 1. **A byte-order mark** names UTF-8 or UTF-16 outright.
//! 2. **UTF-16 without one** shows itself: a GEDCOM file begins `0 HEAD`, and in UTF-16 each of
//!    those characters has a zero byte beside it.
//! 3. **Bytes that are all ASCII** leave nothing to decide.
//! 4. **Well-formed UTF-8** with anything beyond ASCII is UTF-8. Text in a single-byte encoding
//!    practically never forms valid UTF-8 sequences: in windows-1251 a capital letter is followed
//!    by a small one, never by the continuation byte UTF-8 would need.
//! 5. **Everything else is weighed.** Each candidate decodes a sample of the lines that have
//!    non-ASCII bytes, and each decoding is scored for how much it looks like writing — a word
//!    that mixes Cyrillic and Latin letters, a box-drawing character inside a name, a byte with no
//!    character at all all count against it. The declared encoding starts with an advantage, and
//!    the lowest score wins.
//!
//! Whatever cannot be decoded becomes U+FFFD and is listed in [`Decoded::flaws`] with its line, so
//! the loss report can say where. A person who can see the result is wrong — the names come out as
//! `Ð˜Ð²Ð°Ð½` — can decode again with [`decode_as`] and the encoding they choose.
//!
//! ANSEL text comes out composed: `u` with a diaeresis written before it becomes `ü`, not `u`
//! followed by a combining mark, so that it compares equal to what a keyboard types.

mod ansel;
mod guess;
mod tables;

use core::fmt;

/// A character encoding this module reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// UTF-8: GEDCOM 7's only encoding, and 5.5.1's `UTF-8`.
    Utf8,
    /// UTF-16, little-endian: 5.5.1's `UNICODE`, as Windows programs wrote it.
    Utf16Le,
    /// UTF-16, big-endian.
    Utf16Be,
    /// ASCII, and nothing beyond it.
    Ascii,
    /// ANSEL as GEDCOM uses it: the Library of Congress's MARC-8 Extended Latin set with GEDCOM's
    /// additions. Diacritics are written before the letter they belong to.
    Ansel,
    /// windows-1250: Polish, Czech, Slovak, Hungarian and the rest of Central Europe.
    Windows1250,
    /// windows-1251: Russian, Ukrainian, Belarusian, Bulgarian, Serbian.
    Windows1251,
    /// windows-1252: English, German, French and the rest of Western Europe.
    Windows1252,
    /// IBM437: the original IBM PC code page, used by English-language DOS programs.
    Ibm437,
    /// IBM850: DOS for Western Europe.
    Ibm850,
    /// IBM866: DOS for Russian.
    Ibm866,
    /// Mac OS Roman: Macintosh programs before Mac OS X.
    MacRoman,
}

impl Encoding {
    /// Every encoding, in the order a person choosing one would want them listed.
    pub const ALL: [Self; 12] = [
        Self::Utf8,
        Self::Windows1251,
        Self::Windows1252,
        Self::Ansel,
        Self::Utf16Le,
        Self::Utf16Be,
        Self::Windows1250,
        Self::Ibm866,
        Self::Ibm437,
        Self::Ibm850,
        Self::MacRoman,
        Self::Ascii,
    ];

    /// The encoding's standard name: the IANA registration where there is one.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16LE",
            Self::Utf16Be => "UTF-16BE",
            Self::Ascii => "US-ASCII",
            Self::Ansel => "ANSEL",
            Self::Windows1250 => "windows-1250",
            Self::Windows1251 => "windows-1251",
            Self::Windows1252 => "windows-1252",
            Self::Ibm437 => "IBM437",
            Self::Ibm850 => "IBM850",
            Self::Ibm866 => "IBM866",
            Self::MacRoman => "macintosh",
        }
    }

    /// Whether the encoding writes ASCII as ASCII, one byte each. All do but UTF-16.
    const fn keeps_ascii(self) -> bool {
        !matches!(self, Self::Utf16Le | Self::Utf16Be)
    }
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the encoding was settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// A byte-order mark named it. Certain.
    ByteOrderMark,
    /// The bytes' own shape: UTF-16's zero bytes, well-formed UTF-8, or nothing beyond ASCII.
    /// Certain.
    Shape,
    /// The header declared it, and the bytes bear the declaration out.
    Declaration,
    /// The bytes were weighed, because the header declared nothing usable — no `CHAR`, a family
    /// such as `ANSI`, a name nobody uses — or declared something the bytes contradict. A guess,
    /// and a good one, but one the person importing should be able to see and correct.
    Weighed,
    /// The caller chose it.
    Chosen,
}

/// A place where decoding lost something.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flaw {
    /// The line it is on, counting from 1 as the document reader and a text editor count: CR LF
    /// is one line break, and any other CR or LF is one each. 5.5.1 also allowed LF CR as one
    /// terminator; an editor shows it as two, and so does this.
    pub line: usize,
    /// What was lost.
    pub kind: FlawKind,
}

/// What decoding lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlawKind {
    /// A byte that stands for no character in the encoding. Decoded as U+FFFD.
    Unmapped(u8),
    /// Bytes that do not form a character in UTF-8 or UTF-16: a broken sequence, an unpaired
    /// surrogate, an odd byte at the end. Decoded as U+FFFD.
    Malformed(Vec<u8>),
    /// An ANSEL diacritic with no letter after it on its line or on a `CONC` continuing it. Kept,
    /// on a no-break space, so the mark survives without landing on the wrong letter.
    LoneMark(char),
}

impl fmt::Display for Flaw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line)?;
        match &self.kind {
            FlawKind::Unmapped(byte) => write!(f, "byte 0x{byte:02X} stands for no character"),
            FlawKind::Malformed(bytes) => {
                f.write_str("bytes")?;
                for byte in bytes {
                    write!(f, " {byte:02X}")?;
                }
                f.write_str(" do not form a character")
            }
            FlawKind::LoneMark(mark) => {
                write!(
                    f,
                    "diacritic U+{:04X} has no letter to go on",
                    u32::from(*mark)
                )
            }
        }
    }
}

/// A file's bytes as text, with an account of how they were read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The text, without a byte-order mark.
    pub text: String,
    /// The encoding it was read in.
    pub encoding: Encoding,
    /// How that encoding was settled.
    pub basis: Basis,
    /// What the header's `CHAR` line says, verbatim, if it has one.
    pub declared: Option<String>,
    /// Where decoding lost something, in order, up to [`Decoded::LISTED_FLAWS`] of them.
    pub flaws: Vec<Flaw>,
    /// How many more flaws there were than are listed.
    pub unlisted_flaws: usize,
}

impl Decoded {
    /// How many flaws are listed one by one. A file read in the wrong encoding can have one on
    /// every line; past this many, they are only counted.
    pub const LISTED_FLAWS: usize = 1000;

    /// Whether the header declared an encoding other than the one the file was read in — a
    /// declaration the bytes contradicted, or the caller set aside. A declaration too vague to
    /// contradict, such as `ANSI` read as windows-1251, is not overruled.
    ///
    /// Text that is all ASCII overrules nothing: every encoding but UTF-16 reads it alike.
    #[must_use]
    pub fn overrules_declaration(&self) -> bool {
        let agrees = |declared: Encoding| {
            declared == self.encoding
                || (self.text.is_ascii() && declared.keeps_ascii() && self.encoding.keeps_ascii())
        };
        match self.declared.as_deref().map(Claim::of) {
            Some(Claim::One(declared)) => !agrees(declared),
            Some(Claim::Family(family)) => !family.iter().copied().any(agrees),
            Some(Claim::Nothing) | None => false,
        }
    }
}

/// Reads a file's bytes, settling the encoding as the module documentation describes.
#[must_use]
pub fn decode(bytes: &[u8]) -> Decoded {
    if let Some((encoding, length)) = byte_order_mark(bytes) {
        return read(&bytes[length..], encoding, Basis::ByteOrderMark);
    }
    if let Some(encoding) = utf16_shape(bytes) {
        return read(bytes, encoding, Basis::Shape);
    }

    let claim = declared_charset(bytes)
        .as_deref()
        .map_or(Claim::Nothing, Claim::of);
    if bytes.is_ascii() {
        let encoding = match claim {
            Claim::One(encoding) if encoding.keeps_ascii() => encoding,
            _ => Encoding::Ascii,
        };
        return read(bytes, encoding, Basis::Shape);
    }
    if core::str::from_utf8(bytes).is_ok() {
        return read(bytes, Encoding::Utf8, Basis::Shape);
    }

    let favoured: &[Encoding] = match &claim {
        Claim::One(encoding) => core::slice::from_ref(encoding),
        Claim::Family(family) => family,
        Claim::Nothing => &[],
    };
    let encoding = guess::weigh(bytes, favoured);
    let basis = if matches!(claim, Claim::One(declared) if declared == encoding) {
        Basis::Declaration
    } else {
        Basis::Weighed
    };
    read(bytes, encoding, basis)
}

/// Reads a file's bytes in an encoding the caller chose. A byte-order mark for that encoding is
/// skipped; anything that does not decode is listed as a flaw, as always.
#[must_use]
pub fn decode_as(bytes: &[u8], encoding: Encoding) -> Decoded {
    let mark: &[u8] = match encoding {
        Encoding::Utf8 => &[0xEF, 0xBB, 0xBF],
        Encoding::Utf16Le => &[0xFF, 0xFE],
        Encoding::Utf16Be => &[0xFE, 0xFF],
        _ => &[],
    };
    let bytes = bytes.strip_prefix(mark).unwrap_or(bytes);
    read(bytes, encoding, Basis::Chosen)
}

/// Decodes and accounts for it.
fn read(bytes: &[u8], encoding: Encoding, basis: Basis) -> Decoded {
    let mut text = String::with_capacity(bytes.len());
    let mut flaws = Flaws::default();
    decode_into(bytes, encoding, &mut text, &mut flaws);
    let declared = declared_charset(text.as_bytes());
    let (flaws, unlisted_flaws) = flaws.located(&text);
    Decoded {
        text,
        encoding,
        basis,
        declared,
        flaws,
        unlisted_flaws,
    }
}

/// Decodes `bytes` onto the end of `text`. Returns how many ANSEL diacritics were composed into the
/// letter after them, which the weighing counts as evidence for ANSEL; `0` for any other encoding.
fn decode_into(bytes: &[u8], encoding: Encoding, text: &mut String, flaws: &mut Flaws) -> usize {
    let upper: &[u16; 128] = match encoding {
        Encoding::Ansel => return ansel::decode(bytes, text, flaws),
        Encoding::Utf8 => {
            utf8(bytes, text, flaws);
            return 0;
        }
        Encoding::Utf16Le => {
            utf16(bytes, false, text, flaws);
            return 0;
        }
        Encoding::Utf16Be => {
            utf16(bytes, true, text, flaws);
            return 0;
        }
        Encoding::Ascii => &[0; 128],
        Encoding::Windows1250 => &tables::WINDOWS_1250,
        Encoding::Windows1251 => &tables::WINDOWS_1251,
        Encoding::Windows1252 => &tables::WINDOWS_1252,
        Encoding::Ibm437 => &tables::IBM437,
        Encoding::Ibm850 => &tables::IBM850,
        Encoding::Ibm866 => &tables::IBM866,
        Encoding::MacRoman => &tables::MACINTOSH,
    };
    single_byte(bytes, upper, text, flaws);
    0
}

fn utf8(bytes: &[u8], text: &mut String, flaws: &mut Flaws) {
    for chunk in bytes.utf8_chunks() {
        text.push_str(chunk.valid());
        if !chunk.invalid().is_empty() {
            flaws.note(text.len(), FlawKind::Malformed(chunk.invalid().to_vec()));
            text.push(char::REPLACEMENT_CHARACTER);
        }
    }
}

fn utf16(bytes: &[u8], big_endian: bool, text: &mut String, flaws: &mut Flaws) {
    let unit = |pair: [u8; 2]| {
        if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        }
    };
    let pairs = bytes.chunks_exact(2);
    let odd = pairs.remainder();
    for decoded in char::decode_utf16(pairs.map(|pair| unit([pair[0], pair[1]]))) {
        match decoded {
            Ok(character) => text.push(character),
            Err(error) => {
                let surrogate = error.unpaired_surrogate();
                let bytes = if big_endian {
                    surrogate.to_be_bytes()
                } else {
                    surrogate.to_le_bytes()
                };
                flaws.note(text.len(), FlawKind::Malformed(bytes.to_vec()));
                text.push(char::REPLACEMENT_CHARACTER);
            }
        }
    }
    if !odd.is_empty() {
        flaws.note(text.len(), FlawKind::Malformed(odd.to_vec()));
        text.push(char::REPLACEMENT_CHARACTER);
    }
}

fn single_byte(bytes: &[u8], upper: &[u16; 128], text: &mut String, flaws: &mut Flaws) {
    for &byte in bytes {
        if byte.is_ascii() {
            text.push(char::from(byte));
            continue;
        }
        match char::from_u32(u32::from(upper[usize::from(byte - 0x80)])) {
            Some(character) if character != '\0' => text.push(character),
            _ => {
                flaws.note(text.len(), FlawKind::Unmapped(byte));
                text.push(char::REPLACEMENT_CHARACTER);
            }
        }
    }
}

/// The flaws found while decoding, by their byte offset in the decoded text, until the text is
/// finished and lines can be counted.
#[derive(Default)]
struct Flaws {
    listed: Vec<(usize, FlawKind)>,
    unlisted: usize,
}

impl Flaws {
    fn note(&mut self, offset: usize, kind: FlawKind) {
        if self.listed.len() < Decoded::LISTED_FLAWS {
            self.listed.push((offset, kind));
        } else {
            self.unlisted += 1;
        }
    }

    /// Turns offsets into line numbers, counting CR LF, CR and LF as one terminator each — the
    /// document reader's three.
    fn located(self, text: &str) -> (Vec<Flaw>, usize) {
        let bytes = text.as_bytes();
        let mut line = 1;
        let mut position = 0;
        let mut flaws = Vec::with_capacity(self.listed.len());
        for (offset, kind) in self.listed {
            while position < offset {
                match bytes[position] {
                    b'\n' => line += 1,
                    b'\r' if bytes.get(position + 1) != Some(&b'\n') => line += 1,
                    _ => {}
                }
                position += 1;
            }
            flaws.push(Flaw { line, kind });
        }
        (flaws, self.unlisted)
    }
}

/// What a header's `CHAR` claims about the encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Claim {
    /// One encoding.
    One(Encoding),
    /// One of a family, without saying which: `ANSI` is whichever Windows code page the writer's
    /// system used, `IBMPC` whichever DOS one.
    Family(&'static [Encoding]),
    /// Nothing usable: a name no program is known to mean anything by, or one this module does
    /// not read.
    Nothing,
}

impl Claim {
    /// The names in use, compared without case, spaces or punctuation: `UTF-8`, `utf8` and
    /// `UTF 8` are one name. Beyond the four 5.5.1 allows, these are the ones programs are known
    /// to have written.
    fn of(declared: &str) -> Self {
        use Encoding as E;
        // UTF-16 shows itself before any declaration is consulted, so a file that says UNICODE
        // and is not UTF-16 was most likely written as UTF-8 by a program using the word loosely.
        const UNICODE: &[Encoding] = &[E::Utf16Le, E::Utf16Be, E::Utf8];
        const WINDOWS: &[Encoding] = &[E::Windows1251, E::Windows1252, E::Windows1250];
        const DOS: &[Encoding] = &[E::Ibm866, E::Ibm437, E::Ibm850];

        let key: String = declared
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|c| c.to_ascii_uppercase())
            .collect();
        match key.as_str() {
            "UTF8" => Self::One(E::Utf8),
            "UNICODE" | "UTF16" | "UCS2" => Self::Family(UNICODE),
            "ANSEL" => Self::One(E::Ansel),
            "ASCII" | "USASCII" => Self::One(E::Ascii),
            "ANSI" | "WINDOWS" | "IBMWINDOWS" => Self::Family(WINDOWS),
            "WINDOWS1250" | "CP1250" | "ANSI1250" => Self::One(E::Windows1250),
            "WINDOWS1251" | "CP1251" | "ANSI1251" => Self::One(E::Windows1251),
            // ISO 8859-1 differs from windows-1252 only where 8859-1 has control codes no text
            // contains; browsers read the one as the other for the same reason.
            "WINDOWS1252" | "CP1252" | "ANSI1252" | "ISO88591" | "LATIN1" => {
                Self::One(E::Windows1252)
            }
            "IBMPC" | "IBM" | "PC" | "DOS" | "MSDOS" | "OEM" => Self::Family(DOS),
            "IBM437" | "CP437" => Self::One(E::Ibm437),
            "IBM850" | "CP850" => Self::One(E::Ibm850),
            "IBM866" | "CP866" => Self::One(E::Ibm866),
            "MACINTOSH" | "MAC" | "MACROMAN" => Self::One(E::MacRoman),
            _ => Self::Nothing,
        }
    }
}

/// How many lines of a header to search for `CHAR` before giving up on it.
const HEADER_LINES: usize = 1000;

/// The value of the header's `CHAR` line, if it has one.
///
/// Read from the bytes before decoding, which works because every encoding but UTF-16 keeps ASCII
/// as ASCII, and the header is ASCII in practice. Tolerant in the ways 5.5.1 files need: leading
/// whitespace, runs of spaces, blank lines.
fn declared_charset(bytes: &[u8]) -> Option<String> {
    let bytes = bytes.strip_prefix("\u{FEFF}".as_bytes()).unwrap_or(bytes);
    let mut records = 0;
    for line in bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .take(HEADER_LINES)
    {
        let Some((level, rest)) = first_word(line) else {
            continue;
        };
        if level == b"0" {
            records += 1;
            if records > 1 {
                break;
            }
            continue;
        }
        if level == b"1"
            && let Some((b"CHAR", value)) = first_word(rest)
        {
            let value = value.trim_ascii();
            return (!value.is_empty()).then(|| String::from_utf8_lossy(value).into_owned());
        }
    }
    None
}

/// Splits off the first whitespace-delimited word, and the rest without its leading whitespace.
fn first_word(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let line = line.trim_ascii_start();
    let end = line
        .iter()
        .position(u8::is_ascii_whitespace)
        .unwrap_or(line.len());
    (end > 0).then(|| (&line[..end], line[end..].trim_ascii_start()))
}

fn byte_order_mark(bytes: &[u8]) -> Option<(Encoding, usize)> {
    match bytes {
        [0xEF, 0xBB, 0xBF, ..] => Some((Encoding::Utf8, 3)),
        [0xFF, 0xFE, ..] => Some((Encoding::Utf16Le, 2)),
        [0xFE, 0xFF, ..] => Some((Encoding::Utf16Be, 2)),
        _ => None,
    }
}

/// UTF-16 without a byte-order mark: a file that begins `0 HEAD` has a zero byte beside each of
/// its first characters, after it in little-endian order and before it in big-endian. No text in
/// a single-byte encoding contains a zero byte.
fn utf16_shape(bytes: &[u8]) -> Option<Encoding> {
    match bytes {
        [a, 0, b, 0, ..] if *a != 0 && *b != 0 => Some(Encoding::Utf16Le),
        [0, a, 0, b, ..] if *a != 0 && *b != 0 => Some(Encoding::Utf16Be),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
