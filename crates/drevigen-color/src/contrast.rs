//! Contrast, measured two ways because neither is sufficient alone.
//!
//! **WCAG 2** relative-luminance ratios are what conformance is reported against, so we compute
//! them. They are also known to misjudge light text on dark grounds, which is exactly the
//! «Лампа» theme in [art-direction.md](../../../docs/03-design/art-direction.md).
//!
//! **APCA** models perceived lightness contrast and handles that case correctly. It is the
//! primary guide for choosing text colours; WCAG 2 is retained for the conformance report.
//!
//! Both are needed. Using only WCAG 2 would let us ship a dark theme that passes and is hard to
//! read; using only APCA would leave us unable to state conformance to the standard our
//! accessibility obligations are written against.

use crate::srgb::Srgb;

// ── APCA ───────────────────────────────────────────────────────────────────
// Constants from the SAPC / APCA 0.1.9 "G-4g" contrast constants, as published by Myndex.
// They are magic numbers by nature — an empirical fit to perceptual data — and are named here
// rather than inlined so a future revision of the algorithm is a visible diff.

const APCA_EXPONENT: f64 = 2.4;
const NORM_BG: f64 = 0.56;
const NORM_TXT: f64 = 0.57;
const REV_TXT: f64 = 0.62;
const REV_BG: f64 = 0.65;
const BLACK_THRESHOLD: f64 = 0.022;
const BLACK_CLAMP: f64 = 1.414;
const SCALE_BOW: f64 = 1.14;
const SCALE_WOB: f64 = 1.14;
const LO_BOW_OFFSET: f64 = 0.027;
const LO_WOB_OFFSET: f64 = 0.027;
const DELTA_Y_MIN: f64 = 0.0005;
const LO_CLIP: f64 = 0.1;

/// Screen luminance as APCA defines it — its own weighting, not WCAG's.
fn apca_luminance(c: Srgb) -> f64 {
    let f = |v: f64| v.clamp(0.0, 1.0).powf(APCA_EXPONENT);
    0.212_672_9 * f(c.r) + 0.715_152_2 * f(c.g) + 0.072_175_0 * f(c.b)
}

/// Lifts very dark values, modelling the flare that stops real screens reaching true black.
fn soft_clamp_black(y: f64) -> f64 {
    if y < BLACK_THRESHOLD {
        y + (BLACK_THRESHOLD - y).powf(BLACK_CLAMP)
    } else {
        y
    }
}

/// APCA lightness contrast, `Lc`, for `text` on `background`.
///
/// Positive values mean dark text on a light ground; negative means the reverse. The sign is
/// information, not a detail — the two polarities have different thresholds, which is the whole
/// point of using APCA over a ratio.
///
/// Rough guidance from the APCA authors: `Lc 90` for body text, `Lc 75` for larger body text,
/// `Lc 60` for headlines, `Lc 45` for large or non-text elements, `Lc 30` as an absolute floor
/// for anything meant to be perceived.
#[must_use]
pub fn apca(text: Srgb, background: Srgb) -> f64 {
    let y_txt = soft_clamp_black(apca_luminance(text));
    let y_bg = soft_clamp_black(apca_luminance(background));

    if (y_bg - y_txt).abs() < DELTA_Y_MIN {
        return 0.0;
    }

    let contrast = if y_bg > y_txt {
        // Dark text on a light ground.
        let raw = (y_bg.powf(NORM_BG) - y_txt.powf(NORM_TXT)) * SCALE_BOW;
        if raw < LO_CLIP {
            0.0
        } else {
            raw - LO_BOW_OFFSET
        }
    } else {
        // Light text on a dark ground.
        let raw = (y_bg.powf(REV_BG) - y_txt.powf(REV_TXT)) * SCALE_WOB;
        if raw > -LO_CLIP {
            0.0
        } else {
            raw + LO_WOB_OFFSET
        }
    };

    contrast * 100.0
}

// ── WCAG 2 ─────────────────────────────────────────────────────────────────

/// WCAG 2 relative luminance.
#[must_use]
pub fn relative_luminance(c: Srgb) -> f64 {
    let linear = c.to_linear();
    0.2126 * linear.r + 0.7152 * linear.g + 0.0722 * linear.b
}

/// WCAG 2 contrast ratio between two colours, from `1.0` to `21.0`.
///
/// Order does not matter, which is precisely the limitation APCA exists to address.
#[must_use]
pub fn wcag2(a: Srgb, b: Srgb) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

// ── the project's own thresholds ───────────────────────────────────────────

/// What a colour pair is used for, which decides the threshold it must clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// Running text at the body size. The strictest requirement.
    BodyText,
    /// Text at 24 px or larger, or 19 px bold.
    LargeText,
    /// Text that carries meaning but is not read continuously: labels, captions, marks.
    SecondaryText,
    /// Borders, focus rings, icon strokes, chart marks.
    NonText,
}

impl Use {
    /// The APCA `Lc` magnitude this use must reach.
    ///
    /// Stricter than the APCA guidance in one place: secondary text is held to `Lc 60` rather
    /// than `45`, because in this product it carries dates and source marks that an older
    /// reader has to be able to resolve. See
    /// [elder-ux-and-accessibility.md](../../../docs/01-research/elder-ux-and-accessibility.md).
    #[must_use]
    pub const fn min_apca(self) -> f64 {
        match self {
            Self::BodyText => 90.0,
            Self::LargeText => 75.0,
            Self::SecondaryText => 60.0,
            Self::NonText => 45.0,
        }
    }

    /// The WCAG 2 ratio this use must reach, for the conformance report.
    #[must_use]
    pub const fn min_wcag2(self) -> f64 {
        match self {
            Self::BodyText | Self::SecondaryText => 4.5,
            Self::LargeText | Self::NonText => 3.0,
        }
    }
}

/// The verdict on one foreground/background pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Verdict {
    /// APCA lightness contrast, signed.
    pub apca: f64,
    /// WCAG 2 contrast ratio.
    pub wcag2: f64,
    /// What the pair is used for.
    pub role: Use,
}

impl Verdict {
    /// Whether the pair meets the APCA threshold for its use.
    #[must_use]
    pub fn passes_apca(self) -> bool {
        self.apca.abs() >= self.role.min_apca()
    }

    /// Whether the pair meets the WCAG 2 threshold for its use.
    #[must_use]
    pub fn passes_wcag2(self) -> bool {
        self.wcag2 >= self.role.min_wcag2()
    }

    /// Whether the pair meets both. This is what the build gate checks.
    #[must_use]
    pub fn passes(self) -> bool {
        self.passes_apca() && self.passes_wcag2()
    }
}

/// Evaluates a foreground/background pair against the threshold for its use.
#[must_use]
pub fn check(text: Srgb, background: Srgb, role: Use) -> Verdict {
    Verdict {
        apca: apca(text, background),
        wcag2: wcag2(text, background),
        role,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{Use, apca, check, wcag2};
    use crate::srgb::Srgb;

    fn c(hex: &str) -> Srgb {
        Srgb::from_hex(hex).unwrap()
    }

    #[test]
    fn apca_matches_the_published_anchors() {
        // The two values every APCA implementation is checked against.
        let black_on_white = apca(c("#000000"), c("#FFFFFF"));
        assert!(
            (black_on_white - 106.04).abs() < 0.2,
            "black on white: {black_on_white}"
        );

        let white_on_black = apca(c("#FFFFFF"), c("#000000"));
        assert!(
            (white_on_black + 107.88).abs() < 0.2,
            "white on black: {white_on_black}"
        );
    }

    #[test]
    fn apca_sign_encodes_polarity() {
        assert!(
            apca(c("#000000"), c("#FFFFFF")) > 0.0,
            "dark on light is positive"
        );
        assert!(
            apca(c("#FFFFFF"), c("#000000")) < 0.0,
            "light on dark is negative"
        );
    }

    #[test]
    fn apca_is_not_symmetric_and_that_is_the_point() {
        // A ratio cannot tell these apart; perception can, and so must we.
        let a = apca(c("#000000"), c("#FFFFFF")).abs();
        let b = apca(c("#FFFFFF"), c("#000000")).abs();
        assert!((a - b).abs() > 1.0, "APCA collapsed to a symmetric measure");
        assert!(
            (wcag2(c("#000000"), c("#FFFFFF")) - wcag2(c("#FFFFFF"), c("#000000"))).abs() < 1e-12
        );
    }

    #[test]
    fn identical_colours_have_no_contrast() {
        assert!(apca(c("#8A6A44"), c("#8A6A44")).abs() < 1e-9);
        assert!((wcag2(c("#8A6A44"), c("#8A6A44")) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn wcag2_matches_its_definition_at_the_extremes() {
        assert!((wcag2(c("#000000"), c("#FFFFFF")) - 21.0).abs() < 1e-9);
        assert!((wcag2(c("#FFFFFF"), c("#FFFFFF")) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_projects_light_theme_carries_body_text() {
        // ink on vellum — the pairing most of the product is made of.
        let verdict = check(c("#1E232B"), c("#FBF7EE"), Use::BodyText);
        assert!(
            verdict.passes(),
            "ink on vellum: Lc {:.1}, ratio {:.2}",
            verdict.apca,
            verdict.wcag2
        );
    }

    #[test]
    fn thresholds_are_ordered_by_strictness() {
        assert!(Use::BodyText.min_apca() > Use::LargeText.min_apca());
        assert!(Use::LargeText.min_apca() > Use::SecondaryText.min_apca());
        assert!(Use::SecondaryText.min_apca() > Use::NonText.min_apca());
    }

    #[test]
    fn a_failing_pair_is_reported_as_failing() {
        // Deliberately poor: sepia on vellum is too close for running text.
        let verdict = check(c("#8A6A44"), c("#FBF7EE"), Use::BodyText);
        assert!(
            !verdict.passes_apca(),
            "Lc {:.1} should not pass body text",
            verdict.apca
        );
    }
}
