//! The gate that keeps generated artwork on the gated palette.
//!
//! The contrast contract in [`crate::audit`] proves the palette is readable. It says nothing
//! about whether the *artwork* uses it, and artwork is where a colour escapes: an illustration
//! carries a hex literal, the literal is a shade off, and the result sits beside gated interface
//! chrome looking subtly wrong and following no theme at all.
//!
//! This found a real one. The first asset delivery was art-directed against `#1E232B` ink and
//! `#B4552C` accent, which are two earlier values; the gated palette says `#0B131C` and
//! `#9B3300`. Twenty-seven and seventeen occurrences respectively, in vectors the interface was
//! about to render. Nothing catches that by eye.
//!
//! # What counts as allowed
//!
//! Every colour any theme defines, plus the branch hues, plus `currentColor` and `none`, which
//! are not colours but instructions to inherit one. A hex that is not in that set fails.

use std::collections::BTreeSet;

use crate::palette;

/// One colour in a file that the palette does not define.
#[derive(Debug, Clone, PartialEq)]
pub struct Stray {
    /// Which file it is in.
    pub file: String,
    /// The offending value, upper-cased.
    pub colour: String,
    /// How many times it appears in that file.
    pub count: usize,
    /// The nearest colour the palette does define, for the fix.
    pub nearest: &'static str,
    /// How far away that nearest colour is, in OKLab.
    pub distance: f64,
}

/// Every hex the palette can legitimately produce, upper-cased and six digits.
#[must_use]
pub fn allowed() -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for theme in palette::themes() {
        for token in &theme.tokens {
            set.insert(token.srgb().to_hex().to_uppercase());
        }
    }
    for (_, oklch) in palette::ramped() {
        set.insert(
            drevigen_color::to_srgb_mapped(oklch)
                .to_hex()
                .to_uppercase(),
        );
    }
    set
}

/// Finds the colours in `source` that the palette does not define.
///
/// `file` is only used to label the findings.
#[must_use]
pub fn stray_colours(file: &str, source: &str) -> Vec<Stray> {
    let permitted = allowed();
    let mut counts: Vec<(String, usize)> = Vec::new();

    for hex in hex_literals(source) {
        // `allowed` holds the values as the palette writes them, with the leading hash.
        if permitted.contains(&format!("#{hex}")) {
            continue;
        }
        if let Some(entry) = counts.iter_mut().find(|(seen, _)| *seen == hex) {
            entry.1 += 1;
        } else {
            counts.push((hex, 1));
        }
    }

    counts
        .into_iter()
        .map(|(colour, count)| {
            let (nearest, distance) = nearest_token(&colour);
            Stray {
                file: file.to_owned(),
                colour,
                count,
                nearest,
                distance,
            }
        })
        .collect()
}

/// Pulls every `#rrggbb` and `#rgb` out of a document, normalised to six upper-case digits.
///
/// An SVG is full of `#` that are not colours: `url(#hatch-40)`, `href="#dv-home"`, fragment
/// identifiers. Two rules separate them from colours — a `#` directly after `(` is a reference,
/// and a run of hex digits that continues into more identifier characters is a name.
fn hex_literals(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'#' {
            index += 1;
            continue;
        }

        // `url(#id)` and `url( #id )`.
        let preceded_by_paren = index > 0 && bytes[index - 1] == b'(';

        let digits: String = bytes[index + 1..]
            .iter()
            .take(8)
            .take_while(|byte| byte.is_ascii_hexdigit())
            .map(|byte| char::from(byte.to_ascii_uppercase()))
            .collect();

        let after = bytes.get(index + 1 + digits.len()).copied();
        let continues_as_name =
            after.is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');

        if !preceded_by_paren && !continues_as_name {
            // Eight digits is a colour with alpha; the first six are still the colour.
            match digits.len() {
                3 => found.push(digits.chars().flat_map(|c| [c, c]).collect()),
                6 | 8 => found.push(digits[..6].to_owned()),
                _ => {}
            }
        }
        index += 1 + digits.len().max(1);
    }

    found
}

/// The palette colour closest to a stray, so the report can suggest the fix.
///
/// Measured against the **light** theme only. Artwork is authored on the vellum ground, and
/// searching all three themes finds technically-nearer answers that are no use to a person —
/// `#1E232B` is closest to the high-contrast `ink-soft`, and the colour it was meant to be is
/// the light `ink`.
fn nearest_token(hex: &str) -> (&'static str, f64) {
    let Ok(stray) = drevigen_color::Srgb::from_hex(&format!("#{hex}")) else {
        return ("unknown", f64::INFINITY);
    };
    let stray = drevigen_color::Oklab::from_srgb(stray);

    let mut best = ("unknown", f64::INFINITY);
    for token in &palette::light() {
        let candidate = drevigen_color::Oklab::from_srgb(token.srgb());
        let distance = drevigen_color::delta_eok(stray, candidate);
        if distance < best.1 {
            best = (token.name, distance);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{allowed, hex_literals, stray_colours};

    #[test]
    fn the_palettes_own_colours_pass() {
        let ink = "#0B131C";
        assert!(allowed().contains(ink), "the light ink must be allowed");
        let svg = format!(r##"<svg><path stroke="{ink}" fill="#9B3300"/></svg>"##);
        assert!(stray_colours("t.svg", &svg).is_empty());
    }

    #[test]
    fn the_colour_the_first_delivery_drifted_to_is_caught() {
        // The actual defect this gate was written for.
        let svg = r##"<svg stroke="#1E232B"><circle fill="#B4552C"/></svg>"##;
        let found = stray_colours("12-mode-fan.svg", svg);
        assert_eq!(found.len(), 2);

        let ink = found.iter().find(|s| s.colour == "1E232B").unwrap();
        assert_eq!(
            ink.nearest, "ink",
            "should point at the colour it was meant to be"
        );

        let accent = found.iter().find(|s| s.colour == "B4552C").unwrap();
        assert_eq!(accent.nearest, "sanguine");

        // Both are three to four times the just-noticeable difference of about 0.02, so this
        // was never a rounding artefact — it was two colours nobody compared.
        assert!(
            ink.distance > 0.06 && ink.distance < 0.08,
            "{}",
            ink.distance
        );
        assert!(accent.distance > 0.08, "{}", accent.distance);
    }

    #[test]
    fn inheritance_keywords_are_not_colours() {
        let svg = r#"<svg stroke="currentColor" fill="none"><g color="inherit"/></svg>"#;
        assert!(stray_colours("t.svg", svg).is_empty());
    }

    #[test]
    fn occurrences_are_counted_rather_than_repeated() {
        let svg = r##"<svg><a fill="#123456"/><b fill="#123456"/><c fill="#123456"/></svg>"##;
        let found = stray_colours("t.svg", svg);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].count, 3);
    }

    #[test]
    fn short_form_and_alpha_hexes_are_read() {
        assert_eq!(hex_literals("#fff"), vec!["FFFFFF"]);
        assert_eq!(hex_literals("#0b131cff"), vec!["0B131C"]);
        assert_eq!(
            hex_literals("fill=\"#0B131C\" stroke=\"#9b3300\""),
            vec!["0B131C", "9B3300"]
        );
    }

    #[test]
    fn an_svg_id_that_looks_like_a_colour_is_not_mistaken_for_one() {
        // `url(#hatch-40)`, `url(#abc)` and `href="#dv-home"` are references, not colours —
        // and `#abc` is three hex digits, which is exactly the trap.
        let svg =
            r##"<rect fill="url(#hatch-40)"/><rect fill="url(#abc)"/><use href="#dv-home"/>"##;
        assert!(stray_colours("t.svg", svg).is_empty());
    }
}
