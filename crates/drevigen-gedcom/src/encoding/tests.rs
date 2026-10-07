#![allow(clippy::unwrap_used, clippy::panic)]

use core::fmt::Write as _;

use super::{Basis, Decoded, Encoding, FlawKind, decode, decode_as, tables};

const RUSSIAN: &str = "0 @I1@ INDI
1 NAME Иван Петрович /Смирнов/
1 SEX M
1 BIRT
2 DATE 12 MAR 1871
2 PLAC с. Великое, Ярославский уезд, Ярославская губерния
1 NOTE Крестьянин. Записан в метрической книге церкви Рождества Христова.
0 @I2@ INDI
1 NAME Анна Ивановна /Смирнова/
";

const GERMAN: &str = "0 @I1@ INDI
1 NAME Jürgen /Müller/
1 BIRT
2 DATE 3 FEB 1902
2 PLAC Großenhain, Sachsen, Deutschland
1 NOTE Bäckermeister in Görlitz. Ehefrau: Käthe geb. Schäfer.
";

const FRENCH: &str = "0 @I1@ INDI
1 NAME René /Lévêque/
1 BIRT
2 PLAC Montréal, Québec, Canada
1 NOTE Né à Trois-Rivières, marié en l'église Notre-Dame. Décédé à l'âge de 82 ans.
";

const POLISH: &str = "0 @I1@ INDI
1 NAME Józef /Łukasiewicz/
1 BIRT
2 PLAC Łódź, Polska
1 NOTE Urodzony w Żyrardowie. Ślub w kościele św. Stanisława.
";

/// The German record in ANSEL, written byte by byte rather than by an encoder, so the test does
/// not check the decoder against itself.
const GERMAN_ANSEL: &[u8] = b"0 @I1@ INDI
1 NAME J\xE8urgen /M\xE8uller/
1 BIRT
2 DATE 3 FEB 1902
2 PLAC Gro\xCFenhain, Sachsen, Deutschland
1 NOTE B\xE8ackermeister in G\xE8orlitz. Ehefrau: K\xE8athe geb. Sch\xE8afer.
";

fn gedcom(charset: Option<&str>, body: &str) -> String {
    let mut text =
        String::from("0 HEAD\n1 SOUR TEST\n1 GEDC\n2 VERS 5.5.1\n2 FORM LINEAGE-LINKED\n");
    if let Some(charset) = charset {
        writeln!(text, "1 CHAR {charset}").unwrap();
    }
    text.push_str(body);
    text.push_str("0 TRLR\n");
    text
}

fn gedcom_bytes(charset: Option<&str>, body: &[u8]) -> Vec<u8> {
    let head = gedcom(charset, "");
    let (head, trailer) = head.split_at(head.len() - "0 TRLR\n".len());
    [head.as_bytes(), body, trailer.as_bytes()].concat()
}

/// Writes text in a single-byte encoding by looking each character up in the table the decoder
/// uses. Circular for the tables themselves, which `each_table_is_the_encoding_it_names` pins down
/// separately; what these samples test is the choosing.
fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    let upper = match encoding {
        Encoding::Windows1250 => &tables::WINDOWS_1250,
        Encoding::Windows1251 => &tables::WINDOWS_1251,
        Encoding::Windows1252 => &tables::WINDOWS_1252,
        Encoding::Ibm437 => &tables::IBM437,
        Encoding::Ibm850 => &tables::IBM850,
        Encoding::Ibm866 => &tables::IBM866,
        Encoding::MacRoman => &tables::MACINTOSH,
        other => panic!("{other} is not a single-byte table"),
    };
    text.chars()
        .map(|c| {
            if c.is_ascii() {
                return u8::try_from(c).unwrap();
            }
            let index = upper
                .iter()
                .position(|&code| u32::from(code) == u32::from(c))
                .unwrap_or_else(|| panic!("{c} is not in {encoding}"));
            0x80 + u8::try_from(index).unwrap()
        })
        .collect()
}

fn check(decoded: &Decoded, encoding: Encoding, basis: Basis, original: &str) {
    assert_eq!(decoded.encoding, encoding, "{}", decoded.text);
    assert_eq!(decoded.basis, basis);
    assert_eq!(decoded.text, original);
    assert!(decoded.flaws.is_empty(), "{:?}", decoded.flaws);
}

#[test]
fn each_table_is_the_encoding_it_names() {
    let cases: [(&[u8], Encoding, &str); 11] = [
        (b"\xA3\xF3d\x9F", Encoding::Windows1250, "Łódź"),
        (b"\xC8\xE2\xE0\xED", Encoding::Windows1251, "Иван"),
        (b"M\xFCller \x80", Encoding::Windows1252, "Müller €"),
        (b"M\x81ller Stra\xE1e", Encoding::Ibm437, "Müller Straße"),
        (b"S\x9Bren", Encoding::Ibm850, "Søren"),
        (b"\x88\xA2\xA0\xAD", Encoding::Ibm866, "Иван"),
        (b"M\x9Fller B\x8Ar", Encoding::MacRoman, "Müller Bär"),
        (b"\xA1\xE2od\xE2z", Encoding::Ansel, "Łódź"),
        (b"fakul\xA7tet", Encoding::Ansel, "fakulʹtet"),
        (b"Preu\xCFen", Encoding::Ansel, "Preußen"),
        (b"plain", Encoding::Ascii, "plain"),
    ];
    for (bytes, encoding, expected) in cases {
        assert_eq!(decode_as(bytes, encoding).text, expected, "{encoding}");
    }
}

#[test]
fn ansel_diacritics_move_after_their_letter_and_compose() {
    let decoded = decode_as(b"M\xE8uller", Encoding::Ansel);
    assert_eq!(decoded.text, "Müller");
    assert_eq!(
        decoded.text.chars().count(),
        6,
        "ü precomposed, not u + U+0308"
    );
}

#[test]
fn ansel_stacked_diacritics_compose_in_canonical_order() {
    // Cedilla (class 202) goes before acute (230) whichever order the file wrote them in.
    assert_eq!(decode_as(b"\xE2\xF0c", Encoding::Ansel).text, "\u{1E09}");
    assert_eq!(decode_as(b"\xF0\xE2c", Encoding::Ansel).text, "\u{1E09}");
    // Two marks of one class keep their order: diaeresis then acute is ǘ.
    assert_eq!(decode_as(b"\xE8\xE2u", Encoding::Ansel).text, "\u{01D8}");
    // A spacing ANSEL letter takes a diacritic too: Vietnamese ơ with an acute.
    assert_eq!(decode_as(b"\xE2\xBC", Encoding::Ansel).text, "\u{1EDB}");
}

#[test]
fn ansel_diacritics_without_a_composition_stay_combining() {
    // The ligature halves of romanised Russian ts: t︠s︡.
    assert_eq!(
        decode_as(b"\xEBt\xECs", Encoding::Ansel).text,
        "t\u{FE20}s\u{FE21}"
    );
}

#[test]
fn an_ansel_diacritic_crosses_a_conc_split() {
    let decoded = decode_as(b"1 NOTE M\xE8\r\n2 CONC uller\r\n", Encoding::Ansel);
    assert_eq!(decoded.text, "1 NOTE M\r\n2 CONC üller\r\n");
    assert!(decoded.flaws.is_empty());
}

#[test]
fn an_ansel_diacritic_with_nothing_to_go_on_is_kept_and_reported() {
    let decoded = decode_as(b"1 NOTE end\xE8\n2 CONT next\n", Encoding::Ansel);
    assert_eq!(decoded.text, "1 NOTE end\u{A0}\u{308}\n2 CONT next\n");
    assert_eq!(decoded.flaws.len(), 1);
    assert_eq!(decoded.flaws[0].line, 1);
    assert_eq!(decoded.flaws[0].kind, FlawKind::LoneMark('\u{308}'));
}

#[test]
fn russian_in_windows_1251_under_ansi() {
    let original = gedcom(Some("ANSI"), RUSSIAN);
    let decoded = decode(&encode(&original, Encoding::Windows1251));
    check(&decoded, Encoding::Windows1251, Basis::Weighed, &original);
    assert!(!decoded.overrules_declaration(), "ANSI is vague, not wrong");
}

#[test]
fn russian_in_windows_1251_with_no_declaration() {
    let original = gedcom(None, RUSSIAN);
    let decoded = decode(&encode(&original, Encoding::Windows1251));
    check(&decoded, Encoding::Windows1251, Basis::Weighed, &original);
}

#[test]
fn russian_from_dos_under_ibmpc() {
    let original = gedcom(Some("IBMPC"), RUSSIAN);
    let decoded = decode(&encode(&original, Encoding::Ibm866));
    check(&decoded, Encoding::Ibm866, Basis::Weighed, &original);
}

#[test]
fn russian_in_windows_1251_that_claims_to_be_ansel() {
    let original = gedcom(Some("ANSEL"), RUSSIAN);
    let decoded = decode(&encode(&original, Encoding::Windows1251));
    check(&decoded, Encoding::Windows1251, Basis::Weighed, &original);
    assert!(decoded.overrules_declaration());
}

#[test]
fn one_russian_name_among_english_ones_is_enough() {
    let body = "0 @I1@ INDI\n1 NAME John /Smith/\n0 @I2@ INDI\n1 NAME Иван /Smirnov/\n";
    let original = gedcom(Some("ANSI"), body);
    let decoded = decode(&encode(&original, Encoding::Windows1251));
    check(&decoded, Encoding::Windows1251, Basis::Weighed, &original);
}

#[test]
fn german_in_windows_1252_under_ansi() {
    let original = gedcom(Some("ANSI"), GERMAN);
    let decoded = decode(&encode(&original, Encoding::Windows1252));
    check(&decoded, Encoding::Windows1252, Basis::Weighed, &original);
}

#[test]
fn german_in_windows_1252_that_claims_to_be_ansel() {
    let original = gedcom(Some("ANSEL"), GERMAN);
    let decoded = decode(&encode(&original, Encoding::Windows1252));
    check(&decoded, Encoding::Windows1252, Basis::Weighed, &original);
    assert!(decoded.overrules_declaration());
}

#[test]
fn french_in_windows_1252_that_claims_to_be_ansel() {
    let original = gedcom(Some("ANSEL"), FRENCH);
    let decoded = decode(&encode(&original, Encoding::Windows1252));
    check(&decoded, Encoding::Windows1252, Basis::Weighed, &original);
}

#[test]
fn german_in_ansel_as_declared() {
    let decoded = decode(&gedcom_bytes(Some("ANSEL"), GERMAN_ANSEL));
    check(
        &decoded,
        Encoding::Ansel,
        Basis::Declaration,
        &gedcom(Some("ANSEL"), GERMAN),
    );
}

#[test]
fn german_in_ansel_with_no_declaration() {
    let decoded = decode(&gedcom_bytes(None, GERMAN_ANSEL));
    check(
        &decoded,
        Encoding::Ansel,
        Basis::Weighed,
        &gedcom(None, GERMAN),
    );
}

#[test]
fn german_from_dos_and_from_an_old_mac() {
    let original = gedcom(Some("IBMPC"), GERMAN);
    let decoded = decode(&encode(&original, Encoding::Ibm437));
    check(&decoded, Encoding::Ibm437, Basis::Weighed, &original);

    let original = gedcom(Some("MACINTOSH"), GERMAN);
    let decoded = decode(&encode(&original, Encoding::MacRoman));
    check(&decoded, Encoding::MacRoman, Basis::Declaration, &original);

    // Undeclared, Mac OS Roman's ü and ß read in IBM437 as ƒ and º: letters to Unicode, symbols
    // in use, and that is what tells the two apart.
    let original = gedcom(None, GERMAN);
    let decoded = decode(&encode(&original, Encoding::MacRoman));
    check(&decoded, Encoding::MacRoman, Basis::Weighed, &original);
}

#[test]
fn polish_in_windows_1250_under_ansi() {
    let original = gedcom(Some("ANSI"), POLISH);
    let decoded = decode(&encode(&original, Encoding::Windows1250));
    check(&decoded, Encoding::Windows1250, Basis::Weighed, &original);
}

#[test]
fn a_byte_order_mark_outranks_the_declaration() {
    let original = gedcom(Some("ANSEL"), RUSSIAN);
    let bytes = ["\u{FEFF}".as_bytes(), original.as_bytes()].concat();
    let decoded = decode(&bytes);
    check(&decoded, Encoding::Utf8, Basis::ByteOrderMark, &original);
    assert!(decoded.overrules_declaration());
}

#[test]
fn well_formed_utf8_is_utf8_whatever_it_declares() {
    let original = gedcom(Some("ANSI"), GERMAN);
    let decoded = decode(original.as_bytes());
    check(&decoded, Encoding::Utf8, Basis::Shape, &original);
    assert!(decoded.overrules_declaration());
}

#[test]
fn utf16_with_and_without_a_byte_order_mark() {
    let original = gedcom(Some("UNICODE"), RUSSIAN);
    let little: Vec<u8> = original.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let big: Vec<u8> = original.encode_utf16().flat_map(u16::to_be_bytes).collect();

    let decoded = decode(&[&[0xFF, 0xFE], little.as_slice()].concat());
    check(&decoded, Encoding::Utf16Le, Basis::ByteOrderMark, &original);
    assert_eq!(decoded.declared.as_deref(), Some("UNICODE"));
    assert!(!decoded.overrules_declaration());

    check(&decode(&little), Encoding::Utf16Le, Basis::Shape, &original);
    check(&decode(&big), Encoding::Utf16Be, Basis::Shape, &original);
}

#[test]
fn a_broken_byte_in_utf8_is_one_flaw_on_its_line() {
    let original = gedcom(Some("UTF-8"), RUSSIAN).replace('\n', "\r\n");
    let mut bytes = original.into_bytes();
    // Cut the second byte of the "И" in "Иван", on line 8: six header lines, then the record.
    let at = bytes
        .windows(2)
        .position(|pair| pair == "И".as_bytes())
        .unwrap();
    bytes.remove(at + 1);

    let decoded = decode(&bytes);
    assert_eq!(decoded.encoding, Encoding::Utf8);
    assert_eq!(decoded.basis, Basis::Declaration);
    assert_eq!(decoded.flaws.len(), 1);
    assert_eq!(decoded.flaws[0].line, 8);
    assert_eq!(decoded.flaws[0].kind, FlawKind::Malformed(vec![0xD0]));
    assert!(decoded.text.contains("\u{FFFD}ван Петрович"));
}

#[test]
fn a_byte_with_no_character_is_reported_where_it_is() {
    let mut bytes = encode(&gedcom(Some("ANSI"), RUSSIAN), Encoding::Windows1251);
    let at = bytes.iter().position(|&b| b == 0xC8).unwrap();
    bytes[at] = 0x98;
    let decoded = decode(&bytes);
    assert_eq!(decoded.encoding, Encoding::Windows1251);
    assert_eq!(decoded.flaws.len(), 1);
    assert_eq!(decoded.flaws[0].line, 8);
    assert_eq!(decoded.flaws[0].kind, FlawKind::Unmapped(0x98));
}

#[test]
fn ascii_settles_nothing_and_overrules_nothing() {
    let decoded = decode(gedcom(Some("ANSEL"), "0 @I1@ INDI\n").as_bytes());
    assert_eq!(decoded.encoding, Encoding::Ansel);
    assert_eq!(decoded.basis, Basis::Shape);

    let decoded = decode(gedcom(Some("ANSI"), "0 @I1@ INDI\n").as_bytes());
    assert_eq!(decoded.encoding, Encoding::Ascii);
    assert!(!decoded.overrules_declaration());
}

#[test]
fn the_caller_can_overrule_everything() {
    let original = gedcom(Some("UTF-8"), GERMAN);
    let decoded = decode_as(original.as_bytes(), Encoding::Windows1252);
    assert_eq!(decoded.basis, Basis::Chosen);
    assert!(decoded.text.contains("JÃ¼rgen"));
    assert!(decoded.overrules_declaration());
}

#[test]
fn the_declaration_is_found_in_untidy_headers() {
    let text = "0 HEAD\r\n\r\n   1   CHAR   ANSEL  \r\n0 TRLR\r\n";
    assert_eq!(decode(text.as_bytes()).declared.as_deref(), Some("ANSEL"));
    let late = "0 HEAD\n0 @I1@ INDI\n1 CHAR ANSEL\n0 TRLR\n";
    assert_eq!(decode(late.as_bytes()).declared, None, "outside the header");
}

#[test]
fn past_a_thousand_flaws_they_are_counted() {
    let bytes = vec![0x98; 1500];
    let decoded = decode_as(&bytes, Encoding::Windows1251);
    assert_eq!(decoded.flaws.len(), Decoded::LISTED_FLAWS);
    assert_eq!(decoded.unlisted_flaws, 500);
}
