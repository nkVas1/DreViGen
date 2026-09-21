//! The DreViGen mark, drawn from parameters rather than stored as a file.
//!
//! The mark is a **seed in cross-section whose growth rings are generations**: three concentric
//! rings, hand-irregular rather than mechanically circular, with radial rays between them like
//! the medullary rays in a wood section, and a single filled dot of `sanguine` at the centre —
//! the only colour in it.
//!
//! It is generated rather than drawn once and exported because it has to exist at a dozen sizes
//! in several themes: application icons for five platforms, an adaptive-icon pair for Android, a
//! favicon, a social card, and the wordmark in the interface. Exporting those by hand from a
//! design file is how a logo ends up with eleven slightly different versions in a repository.
//!
//! Colours come from [`drevigen_color`], so the mark cannot drift from the palette the contrast
//! gate checks.
//!
//! ```
//! use drevigen_mark::{Mark, Tone};
//!
//! let svg = Mark::new(Tone::Light).to_svg(512);
//! assert!(svg.starts_with("<svg"));
//! ```

use core::fmt::Write as _;

use drevigen_color::Srgb;

/// Which ground the mark will sit on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Ink on vellum — the default.
    Light,
    /// Warm light on a lamplit ground.
    Dark,
    /// One colour, no accent. For a Windows taskbar or a macOS template icon.
    Monochrome,
}

/// The mark, at whatever size and tone is asked for.
#[derive(Debug, Clone)]
pub struct Mark {
    tone: Tone,
    /// Whether to paint the ground. An adaptive-icon foreground layer must not.
    ground: bool,
    /// Fraction of the canvas the mark occupies, leaving the rest as margin.
    inset: f64,
}

impl Mark {
    /// Creates the mark in the given tone, on its own ground.
    #[must_use]
    pub fn new(tone: Tone) -> Self {
        Self {
            tone,
            ground: true,
            inset: 0.88,
        }
    }

    /// Returns the mark without a ground, for compositing.
    #[must_use]
    pub fn transparent(mut self) -> Self {
        self.ground = false;
        self
    }

    /// Returns the mark occupying `fraction` of the canvas.
    ///
    /// Android's adaptive icons crop to a shape that varies by launcher, and only the middle
    /// 66 % is guaranteed to survive; passing `0.6` keeps the mark inside that safe zone.
    #[must_use]
    pub fn inset(mut self, fraction: f64) -> Self {
        self.inset = fraction.clamp(0.1, 1.0);
        self
    }

    fn ink(&self) -> Srgb {
        match self.tone {
            Tone::Light | Tone::Monochrome => hex("#0B131C"),
            Tone::Dark => hex("#F2EADA"),
        }
    }

    fn accent(&self) -> Srgb {
        match self.tone {
            // The one spot of colour. A monochrome icon has none by definition, so the centre
            // is drawn in the ink and distinguished by being solid.
            Tone::Light => hex("#9B3300"),
            Tone::Dark => hex("#FFB98F"),
            Tone::Monochrome => hex("#0B131C"),
        }
    }

    fn ground_colour(&self) -> Srgb {
        match self.tone {
            Tone::Light | Tone::Monochrome => hex("#F7F3EB"),
            Tone::Dark => hex("#18130E"),
        }
    }

    /// Renders the mark as SVG at the given pixel size.
    #[must_use]
    pub fn to_svg(&self, size: u32) -> String {
        let s = f64::from(size);
        let c = s / 2.0;
        let r = c * self.inset;

        let ink = self.ink().to_hex();
        let accent = self.accent().to_hex();

        let mut svg = String::with_capacity(4096);
        let _ = write!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">"#
        );
        let _ = write!(svg, "<title>DreViGen</title>");

        if self.ground {
            let _ = write!(
                svg,
                r#"<rect width="{size}" height="{size}" fill="{}"/>"#,
                self.ground_colour().to_hex()
            );
        }

        // Stroke weights scale with the mark so the engraving reads the same at 32 px and 1024.
        let hair = (r * 0.012).max(0.6);
        let line = (r * 0.026).max(1.0);

        // The radial rays sit between the outer dotted ring and the second ring. Sixteen of
        // them: enough to read as a wood section, few enough not to become a moiré at 32 px.
        let _ = write!(
            svg,
            r#"<g stroke="{ink}" stroke-width="{hair:.2}" stroke-linecap="round" fill="none" opacity="0.55">"#
        );
        for i in 0..16 {
            let angle = f64::from(i) * (core::f64::consts::TAU / 16.0);
            let (sin, cos) = angle.sin_cos();
            let (x1, y1) = (c + cos * r * 0.84, c + sin * r * 0.84);
            let (x2, y2) = (c + cos * r * 0.70, c + sin * r * 0.70);
            let _ = write!(
                svg,
                r#"<line x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}"/>"#
            );
        }
        let _ = write!(svg, "</g>");

        // The rings. Each is a four-arc circle with slightly unequal radii, so the outline is
        // hand-cut rather than struck with a compass.
        let _ = write!(
            svg,
            r#"<g stroke="{ink}" fill="none" stroke-linejoin="round">"#
        );
        let _ = write!(
            svg,
            r#"<circle cx="{c:.2}" cy="{c:.2}" r="{:.2}" stroke-width="{hair:.2}" stroke-dasharray="{:.2} {:.2}" stroke-linecap="round"/>"#,
            r * 0.97,
            hair * 0.9,
            hair * 7.0
        );
        let _ = write!(svg, "{}", irregular_ring(c, r * 0.80, line, 1.010, 0.992));
        let _ = write!(
            svg,
            r#"<circle cx="{c:.2}" cy="{c:.2}" r="{:.2}" stroke-width="{hair:.2}"/>"#,
            r * 0.72
        );
        let _ = write!(svg, "{}", irregular_ring(c, r * 0.54, line, 0.993, 1.009));
        let _ = write!(
            svg,
            r#"<circle cx="{c:.2}" cy="{c:.2}" r="{:.2}" stroke-width="{hair:.2}"/>"#,
            r * 0.46
        );
        let _ = write!(
            svg,
            r#"<circle cx="{c:.2}" cy="{c:.2}" r="{:.2}" stroke-width="{line:.2}"/>"#,
            r * 0.29
        );
        let _ = write!(svg, "</g>");

        // The seed. At 16 px this dot and the outer ring are the whole mark.
        let _ = write!(
            svg,
            r#"<circle cx="{c:.2}" cy="{c:.2}" r="{:.2}" fill="{accent}"/>"#,
            r * 0.105
        );

        svg.push_str("</svg>");
        svg
    }
}

/// A circle drawn as four cubic arcs with unequal radii, so it is not perfectly round.
///
/// The irregularity is the point: a compass-struck circle reads as a corporate logo, and a
/// specimen in a botanical plate does not have one.
fn irregular_ring(c: f64, r: f64, width: f64, stretch_x: f64, stretch_y: f64) -> String {
    let rx = r * stretch_x;
    let ry = r * stretch_y;
    // The constant that makes a cubic Bezier approximate a quarter circle.
    let k = 0.552_284_749_831;
    let (ox, oy) = (rx * k, ry * k);

    // Four quarter arcs, clockwise from the top. Each is (first control, second control, end).
    let quarters = [
        [(c + ox, c - ry), (c + rx, c - oy), (c + rx, c)],
        [(c + rx, c + oy), (c + ox, c + ry), (c, c + ry)],
        [(c - ox, c + ry), (c - rx, c + oy), (c - rx, c)],
        [(c - rx, c - oy), (c - ox, c - ry), (c, c - ry)],
    ];

    let mut d = format!("M {c:.2} {:.2}", c - ry);
    for [(x1, y1), (x2, y2), (x, y)] in quarters {
        let _ = write!(d, " C {x1:.2} {y1:.2}, {x2:.2} {y2:.2}, {x:.2} {y:.2}");
    }
    d.push_str(" Z");

    format!(r#"<path stroke-width="{width:.2}" d="{d}"/>"#)
}

fn hex(value: &str) -> Srgb {
    Srgb::from_hex(value).unwrap_or(Srgb::new(0.0, 0.0, 0.0))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{Mark, Tone};

    #[test]
    fn the_svg_is_well_formed_enough_to_parse() {
        for tone in [Tone::Light, Tone::Dark, Tone::Monochrome] {
            let svg = Mark::new(tone).to_svg(512);
            assert!(svg.starts_with("<svg"), "missing root element");
            assert!(svg.ends_with("</svg>"), "unterminated root element");
            assert_eq!(svg.matches("<g").count(), svg.matches("</g>").count());
            assert!(!svg.contains("NaN"), "a coordinate came out NaN");
        }
    }

    #[test]
    fn the_accent_appears_exactly_once_in_colour_tones() {
        // "One accent. If two things are sanguine, one of them is wrong." — art-direction §4.
        let svg = Mark::new(Tone::Light).to_svg(256);
        assert_eq!(svg.matches("#9B3300").count(), 1);
    }

    #[test]
    fn the_monochrome_tone_uses_no_accent() {
        let svg = Mark::new(Tone::Monochrome).to_svg(256);
        assert!(
            !svg.contains("#9B3300"),
            "a template icon must be one colour"
        );
    }

    #[test]
    fn a_transparent_mark_paints_no_ground() {
        let opaque = Mark::new(Tone::Light).to_svg(256);
        let clear = Mark::new(Tone::Light).transparent().to_svg(256);
        assert!(opaque.contains("<rect"), "the default should have a ground");
        assert!(
            !clear.contains("<rect"),
            "a foreground layer must be transparent"
        );
    }

    #[test]
    fn the_inset_keeps_the_mark_inside_the_canvas() {
        // Android crops adaptive icons to a launcher-dependent shape; only the middle 66 % is
        // guaranteed. The outermost ring must stay inside whatever inset is asked for.
        for inset in [0.6_f64, 0.88, 1.0] {
            let size = 1000_u32;
            let svg = Mark::new(Tone::Light).inset(inset).to_svg(size);
            let radius: f64 = svg
                .split("r=\"")
                .skip(1)
                .filter_map(|s| s.split('"').next())
                .filter_map(|s| s.parse::<f64>().ok())
                .fold(0.0, f64::max);
            let limit = f64::from(size) / 2.0 * inset;
            assert!(
                radius <= limit + 0.01,
                "inset {inset}: {radius} exceeds {limit}"
            );
        }
    }

    #[test]
    fn stroke_weight_scales_with_the_mark() {
        // The same engraving at 32 px and at 1024 px, not a hairline that vanishes.
        let small = Mark::new(Tone::Light).to_svg(32);
        let large = Mark::new(Tone::Light).to_svg(1024);
        let widest = |svg: &str| -> f64 {
            svg.split("stroke-width=\"")
                .skip(1)
                .filter_map(|s| s.split('"').next())
                .filter_map(|s| s.parse::<f64>().ok())
                .fold(0.0, f64::max)
        };
        assert!(
            widest(&large) > widest(&small) * 8.0,
            "strokes did not scale"
        );
    }

    #[test]
    fn size_scales_every_coordinate() {
        let svg = Mark::new(Tone::Light).to_svg(64);
        assert!(svg.contains(r#"width="64""#));
        assert!(svg.contains(r#"viewBox="0 0 64 64""#));
    }
}
