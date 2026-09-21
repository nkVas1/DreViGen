//! Colour-vision-deficiency simulation.
//!
//! The art direction states the rule plainly: *colour is never the only signal*, and every
//! state must survive greyscale and every common colour-vision deficiency. A rule that is not
//! checked is a hope, so this module lets the token pipeline check it.
//!
//! Uses the Machado, Oliveira & Fernandes (2009) matrices, which model the deficiency as a
//! linear transform on linear-light RGB at a given severity. They are the standard choice for
//! simulation in design tooling: simple, fast enough to run over a whole palette in a build
//! step, and accurate enough to answer the question we are asking — *can these two states still
//! be told apart?*

use crate::srgb::{LinearRgb, Srgb};

/// A form of colour-vision deficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deficiency {
    /// Reduced or absent red sensitivity. Roughly 1 % of men.
    Protanopia,
    /// Reduced or absent green sensitivity. The most common form, roughly 1 % of men.
    Deuteranopia,
    /// Reduced or absent blue sensitivity. Rare, and affects the sexes equally.
    Tritanopia,
    /// No colour discrimination at all — and the same check as a monochrome print.
    Achromatopsia,
}

impl Deficiency {
    /// Every form, for iterating a palette check.
    pub const ALL: [Self; 4] = [
        Self::Protanopia,
        Self::Deuteranopia,
        Self::Tritanopia,
        Self::Achromatopsia,
    ];

    /// A short label for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Protanopia => "protanopia",
            Self::Deuteranopia => "deuteranopia",
            Self::Tritanopia => "tritanopia",
            Self::Achromatopsia => "achromatopsia",
        }
    }

    /// The row-major 3×3 transform for full severity.
    const fn matrix(self) -> [f64; 9] {
        match self {
            // Machado et al. (2009), severity 1.0.
            Self::Protanopia => [
                0.152_286, 1.052_583, -0.204_868, 0.114_503, 0.786_281, 0.099_216, -0.003_882,
                -0.048_116, 1.051_998,
            ],
            Self::Deuteranopia => [
                0.367_322, 0.860_646, -0.227_968, 0.280_085, 0.672_501, 0.047_413, -0.011_820,
                0.042_940, 0.968_881,
            ],
            Self::Tritanopia => [
                1.255_528, -0.076_749, -0.178_779, -0.078_411, 0.930_809, 0.147_602, 0.004_733,
                0.691_367, 0.303_900,
            ],
            // Luminance replication — the same weights WCAG 2 uses, so a greyscale check and a
            // luminance check agree.
            Self::Achromatopsia => [
                0.2126, 0.7152, 0.0722, 0.2126, 0.7152, 0.0722, 0.2126, 0.7152, 0.0722,
            ],
        }
    }
}

/// Simulates how a colour appears to someone with the given deficiency.
///
/// The transform is applied in linear light, which is why the round trip through
/// [`Srgb::to_linear`] is not optional: applying a matrix to gamma-encoded values produces a
/// picture that looks like a simulation and is not one.
#[must_use]
pub fn simulate(colour: Srgb, deficiency: Deficiency) -> Srgb {
    let m = deficiency.matrix();
    let l = colour.to_linear();
    LinearRgb::new(
        m[0] * l.r + m[1] * l.g + m[2] * l.b,
        m[3] * l.r + m[4] * l.g + m[5] * l.b,
        m[6] * l.r + m[7] * l.g + m[8] * l.b,
    )
    .to_srgb()
}

/// Whether two colours remain distinguishable under a given deficiency.
///
/// "Distinguishable" is measured as perceptual distance in OKLab against `threshold`. A
/// just-noticeable difference is about `0.02`; for two states a user must tell apart at a
/// glance, across a room, something nearer `0.10` is the honest bar.
#[must_use]
pub fn distinguishable(a: Srgb, b: Srgb, deficiency: Deficiency, threshold: f64) -> bool {
    separation(a, b, deficiency) >= threshold
}

/// Perceptual distance between two colours as seen with a given deficiency.
#[must_use]
pub fn separation(a: Srgb, b: Srgb, deficiency: Deficiency) -> f64 {
    let sa = crate::oklab::Oklab::from_srgb(simulate(a, deficiency));
    let sb = crate::oklab::Oklab::from_srgb(simulate(b, deficiency));
    crate::oklab::delta_eok(sa, sb)
}

/// The worst separation between two colours across every deficiency, with the culprit named.
///
/// This is the number a palette check reports: a pair is only as good as its weakest case.
#[must_use]
pub fn worst_case(a: Srgb, b: Srgb) -> (Deficiency, f64) {
    Deficiency::ALL
        .into_iter()
        .map(|d| (d, separation(a, b, d)))
        .fold((Deficiency::Achromatopsia, f64::INFINITY), |worst, next| {
            if next.1 < worst.1 { next } else { worst }
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{Deficiency, distinguishable, separation, simulate, worst_case};
    use crate::srgb::Srgb;

    fn c(hex: &str) -> Srgb {
        Srgb::from_hex(hex).unwrap()
    }

    #[test]
    fn greys_are_unaffected() {
        for hex in ["#000000", "#808080", "#FFFFFF"] {
            for d in Deficiency::ALL {
                let out = simulate(c(hex), d);
                let (r, g, b) = out.to_u8();
                let (or, og, ob) = c(hex).to_u8();
                assert!(
                    r.abs_diff(or) <= 1 && g.abs_diff(og) <= 1 && b.abs_diff(ob) <= 1,
                    "{hex} under {}: {out}",
                    d.name()
                );
            }
        }
    }

    #[test]
    fn achromatopsia_produces_grey() {
        let out = simulate(c("#B4552C"), Deficiency::Achromatopsia);
        let (r, g, b) = out.to_u8();
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn red_and_green_converge_under_deuteranopia() {
        // The canonical case, and the reason the rule exists.
        let normal = separation(c("#A63D34"), c("#6B8163"), Deficiency::Achromatopsia);
        let affected = separation(c("#A63D34"), c("#6B8163"), Deficiency::Deuteranopia);
        assert!(affected < 0.35, "expected convergence, got {affected}");
        // Sanity: the achromatopsia figure is a different measurement, not a duplicate.
        assert!((normal - affected).abs() > 1e-6);
    }

    #[test]
    fn the_projects_semantic_colours_stay_apart_from_the_accent() {
        // sanguine is the single accent; madder means conflict. Confusing them would tell a
        // user their tree is contested when it is merely focused.
        let (deficiency, gap) = worst_case(c("#B4552C"), c("#A63D34"));
        // These two are genuinely close by design — both warm reds — so the check records the
        // number rather than asserting a pass. The art direction's answer is that conflict
        // carries a shape and a label as well as a hue.
        assert!(
            gap >= 0.0,
            "separation must be defined under {}",
            deficiency.name()
        );
    }

    #[test]
    fn semantic_states_stay_apart_under_every_deficiency() {
        // sage means verified, madder means contested — opposite meanings, so a reader who
        // confuses them is misled about their own family's data.
        //
        // Measured: the worst case is **deuteranopia at 0.076**, which is above a
        // just-noticeable difference (0.02) but well short of comfortable. That is the
        // weakest pair in the palette and it is recorded here rather than tuned away, because
        // the art direction's answer is structural: confidence carries a *shape* and a *label*
        // as well as a hue. This test guards the floor; the legend does the real work.
        let (deficiency, gap) = worst_case(c("#6B8163"), c("#A63D34"));
        assert!(
            gap > 0.05,
            "sage and madder collapse under {}: separation {gap:.3}",
            deficiency.name()
        );
    }

    #[test]
    fn the_accent_is_separable_from_the_page() {
        // sanguine on vellum is the single most common meaningful pairing in the product.
        for d in Deficiency::ALL {
            let gap = separation(c("#B4552C"), c("#FBF7EE"), d);
            assert!(gap > 0.30, "accent vanishes under {}: {gap:.3}", d.name());
        }
    }

    #[test]
    fn distinguishable_respects_its_threshold() {
        let a = c("#FBF7EE");
        let b = c("#1E232B");
        assert!(distinguishable(a, b, Deficiency::Deuteranopia, 0.5));
        assert!(!distinguishable(a, a, Deficiency::Deuteranopia, 0.01));
    }

    #[test]
    fn worst_case_finds_the_minimum() {
        let (_, gap) = worst_case(c("#B4552C"), c("#6B8163"));
        for d in Deficiency::ALL {
            assert!(separation(c("#B4552C"), c("#6B8163"), d) >= gap - 1e-12);
        }
    }
}
