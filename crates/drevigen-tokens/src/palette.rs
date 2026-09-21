//! The palette, authored in OKLCH, exactly as
//! [art-direction.md](../../../docs/03-design/art-direction.md) §4 specifies it.
//!
//! This file is the single source of the colours. The CSS, the TypeScript tokens and the
//! contrast report are all generated from it, so a colour cannot be changed in one place and
//! stay stale in another — the failure mode every hand-maintained design system eventually has.

use drevigen_color::{Oklch, Srgb, Use, to_srgb_mapped};

/// One named colour in one theme.
#[derive(Debug, Clone, Copy)]
pub struct Token {
    /// The token name, without the `--dv-` prefix the CSS adds.
    pub name: &'static str,
    /// The colour as authored.
    pub oklch: Oklch,
    /// One line on what it is for, carried into the generated CSS as a comment.
    pub purpose: &'static str,
}

impl Token {
    const fn new(name: &'static str, l: f64, c: f64, h: f64, purpose: &'static str) -> Self {
        Self {
            name,
            oklch: Oklch { l, c, h },
            purpose,
        }
    }

    /// The displayable colour, gamut-mapped.
    #[must_use]
    pub fn srgb(&self) -> Srgb {
        to_srgb_mapped(self.oklch)
    }
}

/// A foreground/background pairing the product actually renders, and the standard it must meet.
///
/// The list is a contract, not documentation. Every pair here is checked on every build, and a
/// failure stops it — which is the only way a contrast rule survives contact with a deadline.
#[derive(Debug, Clone, Copy)]
pub struct Pairing {
    /// Foreground token name.
    pub foreground: &'static str,
    /// Background token name.
    pub background: &'static str,
    /// What it is used for, which sets the threshold.
    pub role: Use,
    /// Where this pairing appears, so a failure report says what will break.
    pub context: &'static str,
}

/// A complete theme: its tokens and the pairings that must hold within it.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Machine name, used in the CSS selector.
    pub name: &'static str,
    /// Human name, from the art direction.
    pub title: &'static str,
    /// How the theme is selected in CSS.
    pub selector: Selector,
    /// The tokens.
    pub tokens: Vec<Token>,
    /// The pairings this theme must satisfy.
    pub pairings: Vec<Pairing>,
}

/// How a theme is chosen in the cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selector {
    /// The default, on bare `:root`.
    Root,
    /// The system preference, guarded so an explicit light choice still wins.
    PrefersDark,
    /// An explicit `data-theme` stamp.
    Stamped(&'static str),
}

impl Theme {
    /// Looks up a token by name.
    #[must_use]
    pub fn token(&self, name: &str) -> Option<&Token> {
        self.tokens.iter().find(|t| t.name == name)
    }
}

/// The light theme — «Дневной свет».
#[must_use]
pub fn light() -> Vec<Token> {
    vec![
        Token::new(
            "vellum",
            0.965,
            0.012,
            85.0,
            "the page; every surface starts here",
        ),
        Token::new(
            "vellum-deep",
            0.932,
            0.016,
            82.0,
            "recessed panels, plate wells",
        ),
        Token::new(
            "well",
            0.902,
            0.018,
            82.0,
            "the recess a photograph sits in",
        ),
        Token::new(
            "ink",
            0.185,
            0.022,
            250.0,
            "primary text and engraved line; iron-gall, never pure black",
        ),
        Token::new("ink-soft", 0.430, 0.020, 250.0, "secondary text"),
        Token::new(
            "ink-faint",
            0.560,
            0.016,
            250.0,
            "captions and hairline labels",
        ),
        Token::new(
            "rule",
            0.580,
            0.014,
            82.0,
            "structural rules and borders; must be perceivable",
        ),
        Token::new("rule-soft", 0.870, 0.012, 82.0, "the quietest divider"),
        Token::new(
            "sanguine",
            0.468,
            0.150,
            40.0,
            "the single accent as ink: focus, current person",
        ),
        Token::new(
            "sanguine-fill",
            0.372,
            0.130,
            40.0,
            "the accent as a surface, so its label can be read",
        ),
        Token::new("sage", 0.470, 0.065, 142.0, "verified, sourced, confirmed"),
        Token::new(
            "sepia",
            0.460,
            0.075,
            66.0,
            "inferred, approximate, uncertain",
        ),
        Token::new("verdigris", 0.470, 0.072, 186.0, "places and geography"),
        Token::new("indigo", 0.410, 0.092, 266.0, "the historical strata layer"),
        Token::new(
            "madder",
            0.460,
            0.155,
            25.0,
            "conflict, contradiction, open question",
        ),
    ]
}

/// The dark theme — «Лампа», a desk under a lamp rather than an inverted page.
///
/// Derived per token rather than by flipping lightness, as the art direction requires: chroma
/// comes down, because a saturated colour on a dark ground reads louder than the same colour on
/// paper, and the accent is lifted so it still leads the eye.
#[must_use]
pub fn dark() -> Vec<Token> {
    vec![
        Token::new("vellum", 0.190, 0.012, 62.0, "lamplit desk"),
        Token::new("vellum-deep", 0.150, 0.010, 62.0, "recesses"),
        Token::new(
            "well",
            0.230,
            0.012,
            62.0,
            "the recess a photograph sits in",
        ),
        Token::new("ink", 0.930, 0.014, 85.0, "warm light text"),
        Token::new("ink-soft", 0.790, 0.014, 85.0, "secondary text"),
        Token::new(
            "ink-faint",
            0.700,
            0.012,
            85.0,
            "captions and hairline labels",
        ),
        Token::new(
            "rule",
            0.695,
            0.010,
            70.0,
            "structural rules and borders; must be perceivable",
        ),
        Token::new("rule-soft", 0.280, 0.008, 70.0, "the quietest divider"),
        Token::new(
            "sanguine",
            0.875,
            0.082,
            50.0,
            "the single accent as ink, lifted for a dark ground",
        ),
        Token::new(
            "sanguine-fill",
            0.940,
            0.062,
            52.0,
            "the accent as a surface, so its dark label can be read",
        ),
        Token::new("sage", 0.760, 0.058, 142.0, "verified, sourced, confirmed"),
        Token::new(
            "sepia",
            0.770,
            0.070,
            70.0,
            "inferred, approximate, uncertain",
        ),
        Token::new("verdigris", 0.770, 0.065, 186.0, "places and geography"),
        Token::new("indigo", 0.810, 0.075, 266.0, "the historical strata layer"),
        Token::new(
            "madder",
            0.820,
            0.115,
            25.0,
            "conflict, contradiction, open question",
        ),
    ]
}

/// The high-contrast theme, a first-class theme rather than a filter.
///
/// Texture is suppressed elsewhere in the stylesheet; here the job is to push every ink to the
/// far end of the lightness axis so body text clears `Lc 90` with margin to spare.
#[must_use]
pub fn high_contrast() -> Vec<Token> {
    vec![
        Token::new("vellum", 1.000, 0.000, 0.0, "pure page"),
        Token::new("vellum-deep", 0.965, 0.006, 85.0, "recessed panels"),
        Token::new(
            "well",
            0.930,
            0.008,
            85.0,
            "the recess a photograph sits in",
        ),
        Token::new("ink", 0.120, 0.010, 250.0, "primary text"),
        Token::new("ink-soft", 0.280, 0.012, 250.0, "secondary text"),
        Token::new("ink-faint", 0.380, 0.012, 250.0, "captions"),
        Token::new("rule", 0.420, 0.010, 250.0, "borders, at 1.5 px minimum"),
        Token::new(
            "sanguine-fill",
            0.340,
            0.150,
            38.0,
            "the accent as a surface",
        ),
        Token::new("rule-soft", 0.560, 0.008, 250.0, "the quietest divider"),
        Token::new("sanguine", 0.420, 0.160, 38.0, "the single accent"),
        Token::new("sage", 0.380, 0.070, 142.0, "verified"),
        Token::new("sepia", 0.380, 0.080, 66.0, "inferred"),
        Token::new("verdigris", 0.380, 0.078, 186.0, "places"),
        Token::new("indigo", 0.330, 0.100, 266.0, "historical strata"),
        Token::new("madder", 0.380, 0.170, 25.0, "conflict"),
    ]
}

/// The pairings every theme must satisfy.
///
/// Identical across themes on purpose: a dark theme that quietly relaxes its own standard is
/// how a product ends up unreadable at night.
#[must_use]
pub fn contract() -> Vec<Pairing> {
    use Use::{BodyText, LargeText, NonText, SecondaryText};
    vec![
        Pairing {
            foreground: "ink",
            background: "vellum",
            role: BodyText,
            context: "running text on the page",
        },
        Pairing {
            foreground: "ink",
            background: "vellum-deep",
            role: BodyText,
            context: "text in a recessed panel",
        },
        Pairing {
            foreground: "rule",
            background: "well",
            role: NonText,
            context: "the hairline framing a photograph in its well",
        },
        Pairing {
            foreground: "ink-soft",
            background: "vellum",
            role: SecondaryText,
            context: "dates and secondary detail",
        },
        Pairing {
            foreground: "ink-soft",
            background: "vellum-deep",
            role: SecondaryText,
            context: "secondary detail in a panel",
        },
        Pairing {
            foreground: "ink-faint",
            background: "vellum",
            role: NonText,
            context: "hairline labels and captions",
        },
        Pairing {
            foreground: "sanguine",
            background: "vellum",
            role: LargeText,
            context: "the focal person's name",
        },
        Pairing {
            foreground: "sanguine",
            background: "vellum-deep",
            role: NonText,
            context: "the focus ring",
        },
        Pairing {
            foreground: "sage",
            background: "vellum",
            role: SecondaryText,
            context: "source marks on a specimen",
        },
        Pairing {
            foreground: "sepia",
            background: "vellum",
            role: SecondaryText,
            context: "an approximate date",
        },
        Pairing {
            foreground: "verdigris",
            background: "vellum",
            role: SecondaryText,
            context: "a place name",
        },
        Pairing {
            foreground: "indigo",
            background: "vellum",
            role: SecondaryText,
            context: "a historical stratum label",
        },
        Pairing {
            foreground: "madder",
            background: "vellum",
            role: SecondaryText,
            context: "a contested fact",
        },
        Pairing {
            foreground: "rule",
            background: "vellum",
            role: NonText,
            context: "an engraved rule",
        },
        Pairing {
            foreground: "vellum",
            background: "sanguine-fill",
            role: BodyText,
            context: "the label on a primary button",
        },
    ]
}

/// Every theme, in cascade order.
#[must_use]
pub fn themes() -> Vec<Theme> {
    vec![
        Theme {
            name: "light",
            title: "Дневной свет",
            selector: Selector::Root,
            tokens: light(),
            pairings: contract(),
        },
        Theme {
            name: "dark",
            title: "Лампа",
            selector: Selector::PrefersDark,
            tokens: dark(),
            pairings: contract(),
        },
        Theme {
            name: "contrast",
            title: "Высокий контраст",
            selector: Selector::Stamped("contrast"),
            tokens: high_contrast(),
            pairings: contract(),
        },
    ]
}

/// The families that get a full eleven-stop ramp.
///
/// Not every token needs one. A ramp exists where the interface genuinely needs tints and
/// shades of a colour — the accent, and the semantic states that appear as fills as well as
/// strokes.
#[must_use]
pub fn ramped() -> Vec<(&'static str, Oklch)> {
    vec![
        (
            "sanguine",
            Oklch {
                l: 0.520,
                c: 0.145,
                h: 40.0,
            },
        ),
        (
            "sage",
            Oklch {
                l: 0.470,
                c: 0.065,
                h: 142.0,
            },
        ),
        (
            "sepia",
            Oklch {
                l: 0.460,
                c: 0.075,
                h: 66.0,
            },
        ),
        (
            "verdigris",
            Oklch {
                l: 0.470,
                c: 0.072,
                h: 186.0,
            },
        ),
        (
            "indigo",
            Oklch {
                l: 0.410,
                c: 0.092,
                h: 266.0,
            },
        ),
        (
            "madder",
            Oklch {
                l: 0.460,
                c: 0.155,
                h: 25.0,
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{contract, ramped, themes};

    #[test]
    fn every_theme_defines_every_token() {
        let names: Vec<_> = super::light().iter().map(|t| t.name).collect();
        for theme in themes() {
            for name in &names {
                assert!(
                    theme.token(name).is_some(),
                    "theme {} is missing token {name}",
                    theme.name
                );
            }
            assert_eq!(
                theme.tokens.len(),
                names.len(),
                "theme {} has extra tokens",
                theme.name
            );
        }
    }

    #[test]
    fn the_contract_only_references_real_tokens() {
        for theme in themes() {
            for pair in &theme.pairings {
                assert!(
                    theme.token(pair.foreground).is_some(),
                    "{}: unknown foreground {}",
                    theme.name,
                    pair.foreground
                );
                assert!(
                    theme.token(pair.background).is_some(),
                    "{}: unknown background {}",
                    theme.name,
                    pair.background
                );
            }
        }
    }

    #[test]
    fn no_token_is_defined_twice() {
        for theme in themes() {
            let mut names: Vec<_> = theme.tokens.iter().map(|t| t.name).collect();
            names.sort_unstable();
            let before = names.len();
            names.dedup();
            assert_eq!(before, names.len(), "theme {} repeats a token", theme.name);
        }
    }

    #[test]
    fn every_ramped_family_is_also_a_token() {
        let light = super::light();
        for (name, _) in ramped() {
            assert!(
                light.iter().any(|t| t.name == name),
                "{name} has a ramp but no token"
            );
        }
    }

    #[test]
    fn the_contract_is_not_empty_and_covers_body_text() {
        let contract = contract();
        assert!(contract.len() >= 10);
        assert!(
            contract
                .iter()
                .any(|p| p.role == drevigen_color::Use::BodyText),
            "a contrast contract without body text is not a contract"
        );
    }
}
