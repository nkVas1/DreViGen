//! Perceptual colour for DreViGen.
//!
//! One implementation, used in two places that must not disagree: the build step that generates
//! design tokens and refuses to ship a palette that fails contrast, and the canvas, which
//! generates branch tints at runtime. Two implementations would drift, and the drift would show
//! up as a colour that passed the gate and is unreadable on screen.
//!
//! ```
//! use drevigen_color::{Oklch, Srgb, Use, check, to_srgb_mapped};
//!
//! // Author in OKLCH, because equal lightness steps there look equal.
//! let sanguine = Oklch::new(0.58, 0.132, 42.0);
//! let ink = Srgb::from_hex("#1E232B")?;
//! let vellum = Srgb::from_hex("#FBF7EE")?;
//!
//! // Anything displayed goes through gamut mapping, which holds hue and lightness.
//! assert!(to_srgb_mapped(sanguine).in_gamut());
//!
//! // Contrast is checked two ways: APCA to choose, WCAG 2 to report.
//! let verdict = check(ink, vellum, Use::BodyText);
//! assert!(verdict.passes());
//! # Ok::<(), drevigen_color::ParseHexError>(())
//! ```
//!
//! ## Why this is not a dependency
//!
//! Good OKLCH libraries exist. This crate is written rather than imported for three reasons,
//! in order of weight: the canvas calls it inside a frame budget and through the WASM bridge,
//! where an extra layer costs measurably; the contrast thresholds encode *this project's*
//! accessibility obligations rather than a general library's defaults; and the whole thing is
//! two matrices, a binary search and a published formula — the case the project guidelines
//! describe as write it rather than take a dependency for it.
//!
//! ## What lives where
//!
//! | Module | Concern |
//! |---|---|
//! | [`srgb`] | Encoded and linear sRGB, and the transfer function between them |
//! | [`oklab`] | OKLab and OKLCH, and perceptual distance |
//! | [`gamut`] | CSS Color 4 gamut mapping — hue-preserving, unlike clipping |
//! | [`contrast`] | APCA and WCAG 2, and the thresholds this project holds itself to |
//! | [`cvd`] | Colour-vision-deficiency simulation, so *colour is never the only signal* is checked |
//! | [`ramp`] | Eleven-stop scales, and the seeded branch hues the canvas generates |

pub mod contrast;
pub mod cvd;
pub mod gamut;
pub mod oklab;
pub mod ramp;
pub mod srgb;

pub use contrast::{Use, Verdict, apca, check, relative_luminance, wcag2};
pub use cvd::{Deficiency, simulate, worst_case};
pub use gamut::{is_displayable, max_chroma, to_srgb_mapped};
pub use oklab::{Oklab, Oklch, delta_eok};
pub use ramp::{Ramp, STOPS, Step, branch_hues};
pub use srgb::{LinearRgb, ParseHexError, Srgb};
