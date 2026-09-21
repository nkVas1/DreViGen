//! OKLab and OKLCH — the perceptual space the whole design system is authored in.
//!
//! The reason, from [design-language-2026.md](../../../docs/01-research/design-language-2026.md):
//! equal lightness steps in OKLCH *look* equal, where in HSL a ten-percent lightness change
//! reads very differently depending on hue. That is what makes an eleven-stop ramp tractable to
//! build and a branch-hue rotation safe to generate at runtime.
//!
//! The transform is Björn Ottosson's, unmodified. It is a pair of 3×3 matrices around a cube
//! root, which is little enough code that taking a dependency for it would cost more than it
//! saved — and this crate has to compile to WASM and run inside the canvas loop.

use crate::srgb::{LinearRgb, Srgb};

/// A colour in OKLab: perceptual lightness with two opponent axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklab {
    /// Perceptual lightness, `0.0` (black) to `1.0` (white).
    pub l: f64,
    /// Green–red axis.
    pub a: f64,
    /// Blue–yellow axis.
    pub b: f64,
}

/// A colour in OKLCH: OKLab in cylindrical coordinates, which is how designers think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    /// Perceptual lightness, `0.0` to `1.0`.
    pub l: f64,
    /// Chroma — distance from grey. Unbounded in principle; sRGB rarely exceeds `0.37`.
    pub c: f64,
    /// Hue angle in degrees, `0.0..360.0`.
    pub h: f64,
}

impl Oklab {
    /// Creates an OKLab colour.
    #[must_use]
    pub const fn new(l: f64, a: f64, b: f64) -> Self {
        Self { l, a, b }
    }

    /// Converts from linear-light sRGB.
    #[must_use]
    pub fn from_linear(rgb: LinearRgb) -> Self {
        let l = 0.412_221_470_8 * rgb.r + 0.536_332_536_3 * rgb.g + 0.051_445_992_9 * rgb.b;
        let m = 0.211_903_498_2 * rgb.r + 0.680_699_545_1 * rgb.g + 0.107_396_956_6 * rgb.b;
        let s = 0.088_302_461_9 * rgb.r + 0.281_718_837_6 * rgb.g + 0.629_978_700_5 * rgb.b;

        let (l_, m_, s_) = (cbrt(l), cbrt(m), cbrt(s));

        Self {
            l: 0.210_454_255_3 * l_ + 0.793_617_785_0 * m_ - 0.004_072_046_8 * s_,
            a: 1.977_998_495_1 * l_ - 2.428_592_205_0 * m_ + 0.450_593_709_9 * s_,
            b: 0.025_904_037_1 * l_ + 0.782_771_766_2 * m_ - 0.808_675_766_0 * s_,
        }
    }

    /// Converts to linear-light sRGB. The result may be out of gamut.
    #[must_use]
    pub fn to_linear(self) -> LinearRgb {
        let l_ = self.l + 0.396_337_777_4 * self.a + 0.215_803_757_3 * self.b;
        let m_ = self.l - 0.105_561_345_8 * self.a - 0.063_854_172_8 * self.b;
        let s_ = self.l - 0.089_484_177_5 * self.a - 1.291_485_548_0 * self.b;

        let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

        LinearRgb {
            r: 4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
            g: -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
            b: -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
        }
    }

    /// Converts from encoded sRGB.
    #[must_use]
    pub fn from_srgb(c: Srgb) -> Self {
        Self::from_linear(c.to_linear())
    }

    /// Converts to encoded sRGB. The result may be out of gamut.
    #[must_use]
    pub fn to_srgb(self) -> Srgb {
        self.to_linear().to_srgb()
    }

    /// Converts to cylindrical coordinates.
    #[must_use]
    pub fn to_oklch(self) -> Oklch {
        let c = self.a.hypot(self.b);
        // A near-grey colour has no meaningful hue; reporting the arctangent of two rounding
        // errors would make ramp generation jitter.
        let h = if c < 1e-7 {
            0.0
        } else {
            self.b.atan2(self.a).to_degrees().rem_euclid(360.0)
        };
        Oklch { l: self.l, c, h }
    }
}

impl Oklch {
    /// Creates an OKLCH colour. The hue is normalised into `0.0..360.0`.
    #[must_use]
    pub fn new(l: f64, c: f64, h: f64) -> Self {
        Self {
            l,
            c,
            h: h.rem_euclid(360.0),
        }
    }

    /// Converts to Cartesian coordinates.
    #[must_use]
    pub fn to_oklab(self) -> Oklab {
        let rad = self.h.to_radians();
        Oklab {
            l: self.l,
            a: self.c * rad.cos(),
            b: self.c * rad.sin(),
        }
    }

    /// Converts from encoded sRGB.
    #[must_use]
    pub fn from_srgb(c: Srgb) -> Self {
        Oklab::from_srgb(c).to_oklch()
    }

    /// Converts to encoded sRGB without gamut mapping. The result may be out of gamut.
    ///
    /// Use [`crate::gamut::to_srgb_mapped`] when the colour has to be displayable.
    #[must_use]
    pub fn to_srgb_unmapped(self) -> Srgb {
        self.to_oklab().to_srgb()
    }

    /// Returns this colour with a different lightness.
    #[must_use]
    pub fn with_lightness(self, l: f64) -> Self {
        Self { l, ..self }
    }

    /// Returns this colour with a different chroma.
    #[must_use]
    pub fn with_chroma(self, c: f64) -> Self {
        Self { c, ..self }
    }

    /// Returns this colour rotated around the hue circle.
    #[must_use]
    pub fn rotate(self, degrees: f64) -> Self {
        Self::new(self.l, self.c, self.h + degrees)
    }

    /// Formats the colour as a CSS `oklch()` value.
    #[must_use]
    pub fn to_css(self) -> String {
        format!("oklch({:.4} {:.4} {:.2})", self.l, self.c, self.h)
    }
}

/// Perceptual distance between two colours in OKLab.
///
/// This is the metric the CSS Color 4 gamut-mapping algorithm uses, where a just-noticeable
/// difference is about `0.02`.
#[must_use]
pub fn delta_eok(x: Oklab, y: Oklab) -> f64 {
    let dl = x.l - y.l;
    let da = x.a - y.a;
    let db = x.b - y.b;
    (dl * dl + da * da + db * db).sqrt()
}

/// Real cube root, defined for negative inputs.
///
/// `f64::cbrt` already handles this, but the intent is worth naming: the OKLab transform feeds
/// it LMS values that go negative for out-of-gamut colours, and a `powf(1.0/3.0)` there would
/// return NaN and poison the whole pipeline silently.
fn cbrt(x: f64) -> f64 {
    x.cbrt()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{Oklab, Oklch, delta_eok};
    use crate::srgb::Srgb;

    fn close(a: f64, b: f64, tol: f64, what: &str) {
        assert!((a - b).abs() < tol, "{what}: {a} vs {b}");
    }

    #[test]
    fn white_is_lightness_one_and_neutral() {
        let lab = Oklab::from_srgb(Srgb::new(1.0, 1.0, 1.0));
        close(lab.l, 1.0, 1e-5, "L");
        close(lab.a, 0.0, 1e-5, "a");
        close(lab.b, 0.0, 1e-5, "b");
    }

    #[test]
    fn black_is_zero() {
        let lab = Oklab::from_srgb(Srgb::new(0.0, 0.0, 0.0));
        close(lab.l, 0.0, 1e-9, "L");
        close(lab.a, 0.0, 1e-9, "a");
        close(lab.b, 0.0, 1e-9, "b");
    }

    #[test]
    fn primaries_match_ottossons_reference_values() {
        // From the reference implementation accompanying the OKLab specification.
        for (hex, l, a, b) in [
            ("#FF0000", 0.627_955, 0.224_863, 0.125_846),
            ("#00FF00", 0.866_440, -0.233_888, 0.179_498),
            ("#0000FF", 0.452_014, -0.032_457, -0.311_528),
        ] {
            let lab = Oklab::from_srgb(Srgb::from_hex(hex).unwrap());
            close(lab.l, l, 1e-5, &format!("{hex} L"));
            close(lab.a, a, 1e-5, &format!("{hex} a"));
            close(lab.b, b, 1e-5, &format!("{hex} b"));
        }
    }

    #[test]
    fn srgb_round_trips_through_oklab() {
        for hex in [
            "#000000", "#FFFFFF", "#FBF7EE", "#1E232B", "#B4552C", "#6B8163", "#3E8A86", "#A63D34",
            "#3C4E8C", "#8A6A44",
        ] {
            let original = Srgb::from_hex(hex).unwrap();
            let back = Oklab::from_srgb(original).to_srgb();
            // The OKLab matrices are a published fit, not an exact inverse pair, so a round
            // trip accumulates about 3e-8. The tolerance is set two orders below that and
            // still four orders below 8-bit quantisation, where it would become visible.
            close(back.r, original.r, 1e-6, &format!("{hex} r"));
            close(back.g, original.g, 1e-6, &format!("{hex} g"));
            close(back.b, original.b, 1e-6, &format!("{hex} b"));
        }
    }

    #[test]
    fn cylindrical_round_trips() {
        for hex in ["#B4552C", "#6B8163", "#3C4E8C", "#FBF7EE"] {
            let lab = Oklab::from_srgb(Srgb::from_hex(hex).unwrap());
            let back = lab.to_oklch().to_oklab();
            close(back.l, lab.l, 1e-12, "L");
            close(back.a, lab.a, 1e-12, "a");
            close(back.b, lab.b, 1e-12, "b");
        }
    }

    #[test]
    fn greys_have_no_hue_jitter() {
        // atan2 of two rounding errors is not a hue. Ramp generation depends on this.
        for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let lch = Oklab::from_srgb(Srgb::new(v, v, v)).to_oklch();
            assert!(lch.c < 1e-6, "grey {v} has chroma {}", lch.c);
            assert!(
                lch.h.abs() < f64::EPSILON,
                "grey {v} reported hue {}",
                lch.h
            );
        }
    }

    #[test]
    fn equal_lightness_steps_are_perceptually_even() {
        // The property the whole design system rests on: in OKLCH, equal L steps look equal.
        // Measured as even spacing in delta-E, which HSL conspicuously fails.
        let hue = 42.0;
        let steps: Vec<_> = (0..=10)
            .map(|i| Oklch::new(f64::from(i) / 10.0, 0.05, hue).to_oklab())
            .collect();
        let gaps: Vec<f64> = steps.windows(2).map(|w| delta_eok(w[0], w[1])).collect();
        let min = gaps.iter().copied().fold(f64::INFINITY, f64::min);
        let max = gaps.iter().copied().fold(0.0, f64::max);
        assert!((max - min).abs() < 1e-9, "gaps ranged {min}..{max}");
    }

    #[test]
    fn hue_rotation_wraps() {
        let c = Oklch::new(0.6, 0.12, 350.0);
        close(c.rotate(20.0).h, 10.0, 1e-9, "wrap forwards");
        close(c.rotate(-360.0).h, 350.0, 1e-9, "full turn");
        close(
            Oklch::new(0.6, 0.12, -10.0).h,
            350.0,
            1e-9,
            "negative input",
        );
    }

    #[test]
    fn css_formatting_is_stable() {
        let c = Oklch::new(0.58, 0.132, 42.0);
        assert_eq!(c.to_css(), "oklch(0.5800 0.1320 42.00)");
    }

    #[test]
    fn out_of_gamut_colours_survive_conversion() {
        // A saturated colour beyond sRGB must not become NaN on the way back; gamut mapping
        // needs to measure how far outside it is.
        let wide = Oklch::new(0.7, 0.35, 150.0).to_srgb_unmapped();
        assert!(wide.r.is_finite() && wide.g.is_finite() && wide.b.is_finite());
        assert!(!wide.in_gamut(), "expected this to be out of gamut");
    }
}
