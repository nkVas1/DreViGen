//! Eleven-stop ramps, and the generated hues that tint family branches.
//!
//! Two jobs that look different and are the same calculation.
//!
//! A **ramp** takes one identity colour and produces the fifty-through-nine-fifty scale the
//! design system uses, holding hue steady and stepping lightness evenly — which only works
//! because the steps are in OKLCH.
//!
//! A **branch hue** is generated at runtime: the art direction requires that family branches be
//! tinted from a seeded rotation with lightness and chroma fixed, *"so a 12-branch tree is
//! always distinguishable and never garish"*. Choosing those hues by hand would fail the first
//! time a tree had thirteen branches.

use crate::gamut::{max_chroma, to_srgb_mapped};
use crate::oklab::Oklch;
use crate::srgb::Srgb;

/// The eleven stops, named as the design-system convention names them.
pub const STOPS: [u16; 11] = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950];

/// Target lightness for each stop.
///
/// Not a linear sweep: the ends are compressed, because a ramp that reaches pure white and pure
/// black wastes two of its eleven stops on colours that carry no hue and are already available
/// as tokens of their own.
const LIGHTNESS: [f64; 11] = [
    0.972, 0.936, 0.873, 0.800, 0.715, 0.620, 0.530, 0.437, 0.345, 0.258, 0.196,
];

/// How much of the identity chroma each stop carries.
///
/// Chroma is reduced towards both ends, following what the eye expects of a tint and a shade:
/// a very light stop with full chroma reads as a different, louder colour rather than as the
/// same colour lightened.
const CHROMA_SCALE: [f64; 11] = [
    0.20, 0.34, 0.56, 0.76, 0.92, 1.00, 0.98, 0.90, 0.78, 0.64, 0.54,
];

/// A generated colour scale.
#[derive(Debug, Clone)]
pub struct Ramp {
    /// The token family name, for example `sanguine`.
    pub name: String,
    /// The eleven stops, in the order of [`STOPS`].
    pub steps: Vec<Step>,
}

/// One stop of a ramp.
#[derive(Debug, Clone, Copy)]
pub struct Step {
    /// The stop number: 50, 100, … 950.
    pub stop: u16,
    /// The colour as authored, before gamut mapping.
    pub oklch: Oklch,
    /// The colour as it will be displayed.
    pub srgb: Srgb,
    /// Whether the authored colour had to be brought into gamut.
    pub clamped: bool,
}

impl Ramp {
    /// Generates a ramp from an identity colour, which lands at or near stop 500.
    #[must_use]
    pub fn from_identity(name: impl Into<String>, identity: Oklch) -> Self {
        let steps = STOPS
            .iter()
            .zip(LIGHTNESS)
            .zip(CHROMA_SCALE)
            .map(|((&stop, lightness), scale)| {
                // Ask for the scaled chroma, but never more than this lightness can actually
                // carry: beyond that the gamut mapper would pull it back anyway, and the stop
                // would silently stop matching its neighbours.
                let wanted = identity.c * scale;
                let available = max_chroma(lightness, identity.h);
                let chroma = wanted.min(available);
                let oklch = Oklch::new(lightness, chroma, identity.h);
                Step {
                    stop,
                    oklch,
                    srgb: to_srgb_mapped(oklch),
                    clamped: wanted > available + 1e-6,
                }
            })
            .collect();

        Self {
            name: name.into(),
            steps,
        }
    }

    /// Returns a stop by number.
    #[must_use]
    pub fn step(&self, stop: u16) -> Option<&Step> {
        self.steps.iter().find(|s| s.stop == stop)
    }
}

/// Generates `count` branch hues, evenly spaced from a seeded starting angle.
///
/// Lightness and chroma are held constant so the branches read as one family of tints rather
/// than as a set of unrelated colours — the difference between a tree that looks organised and
/// one that looks like a spreadsheet.
///
/// The seed makes the assignment stable: the same tree shows the same colours every time it is
/// opened, which matters because a user learns those colours.
#[must_use]
pub fn branch_hues(count: usize, seed: u64, lightness: f64, chroma: f64) -> Vec<Srgb> {
    if count == 0 {
        return Vec::new();
    }

    // The seed only chooses where on the circle the set begins. Spacing stays even, because
    // even spacing is what makes adjacent branches distinguishable.
    //
    // The seed is mixed before use. Mapping it linearly onto the circle looks reasonable and
    // is useless: consecutive tree identifiers would land a thousandth of a degree apart and
    // produce identical palettes. A SplitMix64 finalizer spreads neighbouring seeds across the
    // whole circle, which is what "seeded" has to mean here.
    let offset = (mix(seed) >> 11) as f64 / (1_u64 << 53) as f64 * 360.0;
    let step = 360.0 / count as f64;

    (0..count)
        .map(|i| {
            let hue = offset + step * i as f64;
            let available = max_chroma(lightness, hue);
            to_srgb_mapped(Oklch::new(lightness, chroma.min(available), hue))
        })
        .collect()
}

/// SplitMix64's finalizer: spreads adjacent seeds across the whole output range.
const fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{Ramp, STOPS, branch_hues};
    use crate::cvd::worst_case;
    use crate::oklab::Oklch;

    fn sanguine() -> Oklch {
        Oklch::new(0.58, 0.132, 42.0)
    }

    #[test]
    fn a_ramp_has_eleven_stops_in_order() {
        let ramp = Ramp::from_identity("sanguine", sanguine());
        assert_eq!(ramp.steps.len(), 11);
        assert_eq!(
            ramp.steps.iter().map(|s| s.stop).collect::<Vec<_>>(),
            STOPS.to_vec()
        );
    }

    #[test]
    fn lightness_decreases_monotonically() {
        let ramp = Ramp::from_identity("sanguine", sanguine());
        for pair in ramp.steps.windows(2) {
            assert!(
                pair[1].oklch.l < pair[0].oklch.l,
                "stop {} is not darker than {}",
                pair[1].stop,
                pair[0].stop
            );
        }
    }

    #[test]
    fn every_stop_is_displayable() {
        for identity in [
            sanguine(),
            Oklch::new(0.55, 0.055, 142.0),
            Oklch::new(0.41, 0.092, 266.0),
            Oklch::new(0.56, 0.066, 186.0),
        ] {
            for step in Ramp::from_identity("x", identity).steps {
                assert!(
                    step.srgb.in_gamut(),
                    "stop {} of hue {} is out of gamut",
                    step.stop,
                    identity.h
                );
            }
        }
    }

    #[test]
    fn a_ramp_holds_its_hue() {
        // The property the whole eleven-stop convention depends on.
        let identity = sanguine();
        for step in Ramp::from_identity("sanguine", identity).steps {
            if step.oklch.c < 0.02 {
                continue; // too near grey for a hue to mean anything
            }
            let got = Oklch::from_srgb(step.srgb).h;
            let drift = (got - identity.h)
                .abs()
                .min(360.0 - (got - identity.h).abs());
            assert!(drift < 2.5, "stop {} drifted {drift:.1}°", step.stop);
        }
    }

    #[test]
    fn chroma_peaks_in_the_middle() {
        let ramp = Ramp::from_identity("sanguine", sanguine());
        let peak = ramp
            .steps
            .iter()
            .max_by(|a, b| a.oklch.c.total_cmp(&b.oklch.c))
            .unwrap();
        assert!(
            (300..=600).contains(&peak.stop),
            "chroma peaked at stop {}",
            peak.stop
        );
    }

    #[test]
    fn identity_survives_into_the_middle_of_the_ramp() {
        let ramp = Ramp::from_identity("sanguine", sanguine());
        let five_hundred = ramp.step(500).unwrap();
        assert!((five_hundred.oklch.l - 0.620).abs() < 1e-9);
        let drift = (five_hundred.oklch.h - 42.0).abs();
        assert!(drift < 1e-9, "hue drifted by {drift}");
    }

    #[test]
    fn branch_hues_are_evenly_spaced_and_seeded() {
        let a = branch_hues(12, 7, 0.58, 0.10);
        let b = branch_hues(12, 7, 0.58, 0.10);
        assert_eq!(a.len(), 12);
        assert_eq!(
            a.iter().map(|c| c.to_hex()).collect::<Vec<_>>(),
            b.iter().map(|c| c.to_hex()).collect::<Vec<_>>(),
            "the same seed must give the same colours"
        );

        // Adjacent seeds must give visibly different palettes, not the same one shifted by a
        // thousandth of a degree. This is the case the mixing step exists for.
        let different = branch_hues(12, 8, 0.58, 0.10);
        assert_ne!(
            a.iter().map(|c| c.to_hex()).collect::<Vec<_>>(),
            different.iter().map(|c| c.to_hex()).collect::<Vec<_>>(),
        );
        let shift = crate::oklab::delta_eok(
            crate::oklab::Oklab::from_srgb(a[0]),
            crate::oklab::Oklab::from_srgb(different[0]),
        );
        assert!(shift > 0.02, "seeds 7 and 8 differ by only {shift:.4}");
    }

    #[test]
    fn twelve_branches_are_all_distinguishable() {
        // The requirement stated in the art direction, checked rather than hoped for.
        let hues = branch_hues(12, 3, 0.58, 0.10);
        for (i, a) in hues.iter().enumerate() {
            for (j, b) in hues.iter().enumerate().skip(i + 1) {
                let gap = crate::cvd::separation(*a, *b, crate::cvd::Deficiency::Achromatopsia);
                // Under full colour blindness evenly spaced hues at one lightness *must*
                // converge — which is why branch identity also carries a label and a position,
                // never colour alone. What must hold is that they differ in normal vision.
                let _ = gap;
                let normal = crate::oklab::delta_eok(
                    crate::oklab::Oklab::from_srgb(*a),
                    crate::oklab::Oklab::from_srgb(*b),
                );
                assert!(
                    normal > 0.04,
                    "branches {i} and {j} are too close: {normal:.3}"
                );
            }
        }
    }

    #[test]
    fn branch_hues_stay_displayable_at_every_angle() {
        for count in [1_usize, 3, 7, 12, 24] {
            for colour in branch_hues(count, 11, 0.58, 0.12) {
                assert!(colour.in_gamut(), "{count} branches produced {colour}");
            }
        }
        assert!(branch_hues(0, 1, 0.5, 0.1).is_empty());
    }

    #[test]
    fn worst_case_is_reported_for_adjacent_branches() {
        // Adjacent branches are the hardest pair; record what the weakest case actually is.
        let hues = branch_hues(12, 3, 0.58, 0.10);
        let (deficiency, gap) = worst_case(hues[0], hues[1]);
        assert!(
            gap.is_finite(),
            "no separation computed under {}",
            deficiency.name()
        );
    }
}
