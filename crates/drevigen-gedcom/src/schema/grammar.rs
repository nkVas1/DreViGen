//! The payload grammars: GEDCOM 7's data types, from the ABNF in chapter 2 of the specification
//! and the standards it cites. Each function quotes the productions it implements.

use drevigen_date::{CalendarDate, DateValue, ParseError, ParseErrorKind};

use super::DataType;

/// Whether a payload is of a data type. An absent payload is the empty string, as GEDCOM 7 treats
/// it. Pointers are the caller's: they are not text.
pub(super) fn accepts(data_type: DataType, text: &str) -> bool {
    match data_type {
        DataType::None => text.is_empty(),
        DataType::Text => true,
        DataType::Y => text.is_empty() || text == "Y",
        DataType::Pointer(_) => false,
        DataType::Enum => is_enum(text),
        DataType::EnumList => is_enum_list(text),
        DataType::TextList => is_text_list(text),
        DataType::Integer => is_integer(text),
        DataType::Date => date(DateValue::parse_gedcom7(text)).is_ok(),
        DataType::DateExact => date(CalendarDate::parse_gedcom7_exact(text)).is_ok(),
        // DatePeriod = [ %s"TO" D date ] / %s"FROM" D date [ D %s"TO" D date ]
        DataType::DatePeriod => matches!(
            date(DateValue::parse_gedcom7(text)),
            Ok(None | Some(None | Some(DateValue::Period { .. })))
        ),
        DataType::Time => is_time(text),
        DataType::Age => is_age(text),
        DataType::PersonalName => is_personal_name(text),
        DataType::Language => is_language_tag(text),
        DataType::MediaType => is_media_type(text),
        DataType::FilePath => is_file_path(text),
        DataType::Uri => is_uri_reference(text),
        DataType::TagDefinition => is_tag_definition(text),
        DataType::Latitude => is_coordinate(text, ('N', 'S'), 90),
        DataType::Longitude => is_coordinate(text, ('E', 'W'), 180),
    }
}

/// A date as the date grammar reads it, or `Err` for one that is not a date. An extension
/// calendar, month or epoch is valid GEDCOM whose meaning this crate cannot know: it is accepted
/// unread, as `Ok(None)`.
fn date<T>(read: Result<T, ParseError>) -> Result<Option<T>, ()> {
    match read {
        Ok(value) => Ok(Some(value)),
        Err(ParseError {
            kind: ParseErrorKind::Extension(_),
            ..
        }) => Ok(None),
        Err(_) => Err(()),
    }
}

/// The items of a list, the spaces around each comma taken as part of the delimiter; the whole
/// text as one item if it is not a list.
pub(super) fn list_items(text: &str) -> Vec<&str> {
    split_list(text).unwrap_or_else(|| vec![text])
}

/// ```abnf
/// stdTag  = ucletter *tagchar
/// extTag  = underscore 1*tagchar
/// stdEnum = stdTag / Integer
/// Enum    = stdEnum / extTag
/// ```
fn is_enum(text: &str) -> bool {
    let tagchar = |c: u8| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_';
    match text.as_bytes() {
        [] => false,
        [b'_', rest @ ..] => !rest.is_empty() && rest.iter().all(|&c| tagchar(c)),
        [first, rest @ ..] if first.is_ascii_uppercase() => rest.iter().all(|&c| tagchar(c)),
        _ => is_integer(text),
    }
}

/// `List-Enum = Enum *(listDelim Enum)`
fn is_enum_list(text: &str) -> bool {
    split_list(text).is_some_and(|items| items.iter().all(|item| is_enum(item)))
}

/// ```abnf
/// listItem  = [ nocommasp / nocommasp *nocomma nocommasp ]
/// nocomma   = %x09-2B / %x2D-10FFFF
/// nocommasp = %x09-1D / %x21-2B / %x2D-10FFFF
/// ```
fn is_text_list(text: &str) -> bool {
    let edge = |c: char| matches!(c, '\u{1E}'..='\u{20}');
    split_list(text).is_some_and(|items| {
        items
            .iter()
            .all(|item| !item.starts_with(edge) && !item.ends_with(edge) && !item.contains('\u{0}'))
    })
}

/// A list's items, or `None` for text that begins or ends with a space no delimiter accounts for.
///
/// ```abnf
/// list      = listItem *(listDelim listItem)
/// listDelim = *D "," *D
/// ```
fn split_list(text: &str) -> Option<Vec<&str>> {
    let pieces: Vec<&str> = text.split(',').collect();
    let last = pieces.len() - 1;
    let items: Vec<&str> = pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| {
            let piece = if index > 0 {
                piece.trim_start_matches(' ')
            } else {
                piece
            };
            if index < last {
                piece.trim_end_matches(' ')
            } else {
                piece
            }
        })
        .collect();
    let bare = |item: &&str| !item.starts_with(' ') && !item.ends_with(' ');
    items.iter().all(bare).then_some(items)
}

/// `Integer = 1*DIGIT`
fn is_integer(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|c| c.is_ascii_digit())
}

/// ```abnf
/// Time     = hour ":" minute [":" second ["." fraction]] [%s"Z"]
/// hour     = DIGIT / ("0" / "1") DIGIT / "2" ("0" / "1" / "2" / "3")
/// minute   = ("0" / "1" / "2" / "3" / "4" / "5") DIGIT
/// second   = ("0" / "1" / "2" / "3" / "4" / "5") DIGIT
/// fraction = 1*DIGIT
/// ```
fn is_time(text: &str) -> bool {
    let text = text.strip_suffix('Z').unwrap_or(text);
    let mut parts = text.splitn(3, ':');
    let (Some(hour), Some(minute)) = (parts.next(), parts.next()) else {
        return false;
    };
    let hour_ok = match hour.as_bytes() {
        [h] | [b'0' | b'1', h] => h.is_ascii_digit(),
        [b'2', b'0'..=b'3'] => true,
        _ => false,
    };
    let sixty = |text: &str| matches!(text.as_bytes(), [b'0'..=b'5', d] if d.is_ascii_digit());
    let seconds_ok = match parts.next() {
        None => true,
        Some(seconds) => match seconds.split_once('.') {
            Some((whole, fraction)) => sixty(whole) && is_integer(fraction),
            None => sixty(seconds),
        },
    };
    hour_ok && sixty(minute) && seconds_ok
}

/// ```abnf
/// Age         = [[ageBound D] ageDuration]
/// ageBound    = "<" / ">"
/// ageDuration = years [D months] [D weeks] [D days]
///             / months [D weeks] [D days]
///             / weeks [D days]
///             / days
/// years   = Integer %x79    ; 35y
/// months  = Integer %x6D    ; 11m
/// weeks   = Integer %x77    ; 8w
/// days    = Integer %x64    ; 21d
/// ```
fn is_age(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let duration = text
        .strip_prefix("< ")
        .or_else(|| text.strip_prefix("> "))
        .unwrap_or(text);
    let mut next_unit = 0;
    let mut any = false;
    for part in duration.split(' ') {
        let Some((count, unit)) = part.split_at_checked(part.len().saturating_sub(1)) else {
            return false;
        };
        let Some(position) = ["y", "m", "w", "d"].iter().position(|u| *u == unit) else {
            return false;
        };
        if position < next_unit || !is_integer(count) {
            return false;
        }
        next_unit = position + 1;
        any = true;
    }
    any
}

/// ```abnf
/// PersonalName = nameStr
///              / [nameStr] "/" [nameStr] "/" [nameStr]
/// nameChar     = %x20-2E / %x30-10FFFF  ; any but '/' and '\t'
/// nameStr      = 1*nameChar
/// ```
fn is_personal_name(text: &str) -> bool {
    let slashes = text.matches('/').count();
    !text.chars().any(|c| c < ' ') && (slashes == 2 || (slashes == 0 && !text.is_empty()))
}

/// BCP 47's `Language-Tag`, RFC 5646 section 2.1, as the specification's `languagetag.abnf`
/// copies it. Case is not significant.
///
/// ```abnf
/// Language-Tag  = langtag / privateuse / grandfathered
/// langtag       = language ["-" script] ["-" region] *("-" variant) *("-" extension)
///                 ["-" privateuse]
/// language      = 2*3ALPHA ["-" extlang] / 4ALPHA / 5*8ALPHA
/// extlang       = 3ALPHA *2("-" 3ALPHA)
/// script        = 4ALPHA
/// region        = 2ALPHA / 3DIGIT
/// variant       = 5*8alphanum / (DIGIT 3alphanum)
/// extension     = singleton 1*("-" (2*8alphanum))
/// singleton     = DIGIT / %x41-57 / %x59-5A / %x61-77 / %x79-7A
/// privateuse    = "x" 1*("-" (1*8alphanum))
/// ```
fn is_language_tag(text: &str) -> bool {
    const GRANDFATHERED: [&str; 26] = [
        "en-GB-oed",
        "i-ami",
        "i-bnn",
        "i-default",
        "i-enochian",
        "i-hak",
        "i-klingon",
        "i-lux",
        "i-mingo",
        "i-navajo",
        "i-pwn",
        "i-tao",
        "i-tay",
        "i-tsu",
        "sgn-BE-FR",
        "sgn-BE-NL",
        "sgn-CH-DE",
        "art-lojban",
        "cel-gaulish",
        "no-bok",
        "no-nyn",
        "zh-guoyu",
        "zh-hakka",
        "zh-min",
        "zh-min-nan",
        "zh-xiang",
    ];
    if GRANDFATHERED
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case(text))
    {
        return true;
    }
    let subtags: Vec<&str> = text.split('-').collect();
    let alnum = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric());
    if !subtags.iter().all(|s| alnum(s)) {
        return false;
    }
    let alpha = |s: &str| s.bytes().all(|c| c.is_ascii_alphabetic());
    let digits = |s: &str| s.bytes().all(|c| c.is_ascii_digit());
    let private = |rest: &[&str]| !rest.is_empty() && rest.iter().all(|s| s.len() <= 8);
    if subtags[0].eq_ignore_ascii_case("x") {
        return private(&subtags[1..]);
    }

    let mut i = 1;
    match subtags[0].len() {
        2 | 3 if alpha(subtags[0]) => {
            let mut extlangs = 0;
            while extlangs < 3 && i < subtags.len() && subtags[i].len() == 3 && alpha(subtags[i]) {
                i += 1;
                extlangs += 1;
            }
        }
        4..=8 if alpha(subtags[0]) => {}
        _ => return false,
    }
    if i < subtags.len() && subtags[i].len() == 4 && alpha(subtags[i]) {
        i += 1;
    }
    if i < subtags.len()
        && ((subtags[i].len() == 2 && alpha(subtags[i]))
            || (subtags[i].len() == 3 && digits(subtags[i])))
    {
        i += 1;
    }
    let variant =
        |s: &str| (5..=8).contains(&s.len()) || (s.len() == 4 && s.as_bytes()[0].is_ascii_digit());
    while i < subtags.len() && variant(subtags[i]) {
        i += 1;
    }
    while i < subtags.len() && subtags[i].len() == 1 && !subtags[i].eq_ignore_ascii_case("x") {
        i += 1;
        let start = i;
        while i < subtags.len() && (2..=8).contains(&subtags[i].len()) {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    if i < subtags.len() && subtags[i].eq_ignore_ascii_case("x") {
        return private(&subtags[i + 1..]);
    }
    i == subtags.len()
}

/// RFC 2045's `type "/" subtype`, RFC 6838's restricted names, and RFC 9110's parameters, as
/// the specification's `mediatype.abnf` combines them. Case is not significant.
///
/// ```abnf
/// MediaType       = type "/" subtype parameters
/// extension-token = ietf-token / x-token
/// x-token         = "x-" token
/// restricted-name = restricted-name-first *126restricted-name-chars
/// restricted-name-first = ALPHA / DIGIT
/// restricted-name-chars = ALPHA / DIGIT / "!" / "#" / "$" / "&" / "-" / "^" / "_" / "." / "+"
/// parameters      = *( OWS ";" OWS [ parameter ] )
/// parameter       = parameter-name "=" parameter-value
/// parameter-value = ( token / quoted-string )
/// ```
fn is_media_type(text: &str) -> bool {
    let (essence, parameters) = text
        .split_once(';')
        .map_or((text, None), |(e, p)| (e, Some(p)));
    let essence = essence.trim_end_matches([' ', '\t']);
    let Some((kind, subtype)) = essence.split_once('/') else {
        return false;
    };
    let name = |s: &str| {
        let lower = s.to_ascii_lowercase();
        let restricted = s.len() <= 127
            && s.bytes().next().is_some_and(|c| c.is_ascii_alphanumeric())
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"!#$&-^_.+".contains(&c));
        restricted || lower.strip_prefix("x-").is_some_and(is_token)
    };
    if !name(kind) || !name(subtype) {
        return false;
    }
    let Some(parameters) = parameters else {
        return true;
    };
    // `OWS ";" OWS [ parameter ]`, repeated: split at semicolons outside quoted strings.
    let mut rest = parameters;
    loop {
        rest = rest.trim_start_matches([' ', '\t']);
        if rest.is_empty() {
            return true;
        }
        if let Some(after) = rest.strip_prefix(';') {
            rest = after;
            continue;
        }
        let Some((parameter, after)) = rest.split_once('=') else {
            return false;
        };
        if !is_token(parameter) {
            return false;
        }
        let consumed = if after.starts_with('"') {
            match quoted_string(after) {
                Some(length) => length,
                None => return false,
            }
        } else {
            let end = after.find([';', ' ', '\t']).unwrap_or(after.len());
            if !is_token(&after[..end]) {
                return false;
            }
            end
        };
        rest = after[consumed..].trim_start_matches([' ', '\t']);
        if !rest.is_empty() && !rest.starts_with(';') {
            return false;
        }
    }
}

/// `token = 1*tchar`, RFC 9110.
fn is_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&c))
}

/// The length of the quoted string at the start of `text`, RFC 9110:
/// `DQUOTE *( qdtext / quoted-pair ) DQUOTE`.
fn quoted_string(text: &str) -> Option<usize> {
    let mut escaped = false;
    for (index, c) in text.char_indices().skip(1) {
        match (escaped, c) {
            (true, _) => escaped = false,
            (false, '\\') => escaped = true,
            (false, '"') => return Some(index + 1),
            (false, c) if (c < ' ' && c != '\t') || c == '\u{7F}' => return None,
            _ => {}
        }
    }
    None
}

/// A WHATWG "valid URL string", within the URLs version 7.0 supports: `ftp`, `http`, `https` and
/// `file` URLs, and relative local file paths with no leading `/`, no `..` segment, no `\`, and
/// no query or fragment.
///
/// What is checked of a valid URL string is its characters — URL code points and well-formed
/// percent-encodings — not the full WHATWG parser's structure.
fn is_file_path(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    match scheme(text) {
        Some(scheme) => {
            ["ftp", "http", "https", "file"]
                .iter()
                .any(|known| known.eq_ignore_ascii_case(scheme))
                && url_characters(&text[scheme.len() + 1..], true)
        }
        None => {
            !text.starts_with('/')
                && !text
                    .split('/')
                    .any(|segment| segment == ".." || segment == "%2E%2E")
                && !text.contains(['\\', '?', '#'])
                && !text.to_ascii_uppercase().contains("%5C")
                && url_characters(text, false)
        }
    }
}

/// The scheme of a URL or URI that has one: `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )` before
/// the first `:`, if that comes before any `/`, `?` or `#`.
fn scheme(text: &str) -> Option<&str> {
    let end = text.find(':')?;
    let candidate = &text[..end];
    let mut bytes = candidate.bytes();
    let valid = bytes.next().is_some_and(|c| c.is_ascii_alphabetic())
        && bytes.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'));
    (valid && !candidate.contains(['/', '?', '#'])).then_some(candidate)
}

/// WHATWG's URL code points, with `%` only as a percent-encoded byte and `#` once, before a
/// fragment, where one is allowed.
fn url_characters(text: &str, fragment: bool) -> bool {
    let (text, fragment_text) = match text.split_once('#') {
        Some((before, after)) if fragment => (before, Some(after)),
        Some(_) => return false,
        None => (text, None),
    };
    let code_point = |c: char| {
        c.is_ascii_alphanumeric()
            || "!$&'()*+,-./:;=?@_~".contains(c)
            || (('\u{A0}'..='\u{10FFFD}').contains(&c)
                && !('\u{FDD0}'..='\u{FDEF}').contains(&c)
                && (u32::from(c) & 0xFFFE) != 0xFFFE)
    };
    [Some(text), fragment_text]
        .into_iter()
        .flatten()
        .all(|part| percent_encoded(part, code_point))
}

/// Every character allowed, or a `%` and two hexadecimal digits.
fn percent_encoded(text: &str, allowed: impl Fn(char) -> bool) -> bool {
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex = |c: Option<char>| c.is_some_and(|c| c.is_ascii_hexdigit());
            if !(hex(chars.next()) && hex(chars.next())) {
                return false;
            }
        } else if !allowed(c) {
            return false;
        }
    }
    true
}

/// RFC 3986's `URI-reference`, by its characters — `unreserved`, `reserved` and percent-encoded
/// octets — and the one structural rule a relative reference adds: a colon in the first segment
/// makes that segment a scheme, which must then be a valid one.
fn is_uri_reference(text: &str) -> bool {
    let first_segment_end = text.find(['/', '?', '#']).unwrap_or(text.len());
    if text[..first_segment_end].contains(':') && scheme(text).is_none() {
        return false;
    }
    if text.matches('#').count() > 1 {
        return false;
    }
    percent_encoded(text, |c| {
        c.is_ascii_alphanumeric() || "-._~:/?#[]@!$&'()*+,;=".contains(c)
    })
}

/// `TagDef = extTag D URI-reference`
fn is_tag_definition(text: &str) -> bool {
    text.split_once(' ')
        .is_some_and(|(tag, uri)| tag.starts_with('_') && is_enum(tag) && is_uri_reference(uri))
}

/// ```abnf
/// Latitude  = ("N" / "S") upto90 [ "." 1*digit]
/// upto90    = "90" / [upto8] digit
/// Longitude = ("E" / "W") upto180 [ "." 1*digit]
/// upto180   = "180" / "1" upto7 digit / [["0"] digit] digit
/// ```
fn is_coordinate(text: &str, (positive, negative): (char, char), most: u32) -> bool {
    let Some(degrees) = text
        .strip_prefix(positive)
        .or_else(|| text.strip_prefix(negative))
    else {
        return false;
    };
    let (whole, fraction) = degrees.split_once('.').unwrap_or((degrees, "1"));
    let width = if most == 90 { 2 } else { 3 };
    is_integer(whole)
        && whole.len() <= width
        && is_integer(fraction)
        && whole.parse::<u32>().is_ok_and(|value| {
            // `upto90` and `upto180` admit the bound itself with any fraction after it, and
            // nothing above it.
            value <= most
        })
}
