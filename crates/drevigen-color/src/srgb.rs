//! sRGB, and the transfer function that separates what a display emits from what a file stores.
//!
//! The distinction matters more here than in most code. Every perceptual calculation in this
//! crate — OKLab conversion, APCA, WCAG luminance, colour-vision simulation — operates on
//! **linear** light. The hex codes in a stylesheet are **encoded**. Mixing the two produces
//! numbers that look plausible and are wrong, which is the failure mode a contrast checker can
//! least afford.

use core::fmt;

/// A colour in the sRGB colour space, with components encoded (gamma-applied) in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Srgb {
    /// Red, encoded.
    pub r: f64,
    /// Green, encoded.
    pub g: f64,
    /// Blue, encoded.
    pub b: f64,
}

/// A colour in linear-light sRGB primaries, with components in `0.0..=1.0` for in-gamut colours.
///
/// Values outside that range are meaningful and are not clamped: they represent colours the
/// display cannot show, which is precisely what gamut mapping needs to know.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearRgb {
    /// Red, linear light.
    pub r: f64,
    /// Green, linear light.
    pub g: f64,
    /// Blue, linear light.
    pub b: f64,
}

impl Srgb {
    /// Creates an encoded sRGB colour.
    #[must_use]
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// Parses `#rgb`, `#rrggbb`, or the same without the leading hash.
    ///
    /// # Errors
    ///
    /// Returns [`ParseHexError`] when the string is not a 3- or 6-digit hex colour.
    pub fn from_hex(hex: &str) -> Result<Self, ParseHexError> {
        let s = hex.strip_prefix('#').unwrap_or(hex);
        let expand = |c: u8| -> Result<u8, ParseHexError> {
            (c as char)
                .to_digit(16)
                .map(|d| d as u8)
                .ok_or(ParseHexError::NotHex(c as char))
        };

        let bytes = s.as_bytes();
        let (r, g, b) = match bytes.len() {
            3 => {
                let (r, g, b) = (expand(bytes[0])?, expand(bytes[1])?, expand(bytes[2])?);
                (r * 17, g * 17, b * 17)
            }
            6 => (
                expand(bytes[0])? * 16 + expand(bytes[1])?,
                expand(bytes[2])? * 16 + expand(bytes[3])?,
                expand(bytes[4])? * 16 + expand(bytes[5])?,
            ),
            n => return Err(ParseHexError::WrongLength(n)),
        };

        Ok(Self::from_u8(r, g, b))
    }

    /// Creates a colour from 8-bit components.
    #[must_use]
    pub fn from_u8(r: u8, g: u8, b: u8) -> Self {
        Self::new(
            f64::from(r) / 255.0,
            f64::from(g) / 255.0,
            f64::from(b) / 255.0,
        )
    }

    /// Returns the colour as 8-bit components, rounded and clamped.
    #[must_use]
    pub fn to_u8(self) -> (u8, u8, u8) {
        let q = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        (q(self.r), q(self.g), q(self.b))
    }

    /// Formats the colour as `#rrggbb`.
    #[must_use]
    pub fn to_hex(self) -> String {
        let (r, g, b) = self.to_u8();
        format!("#{r:02X}{g:02X}{b:02X}")
    }

    /// Whether every component lies within the displayable range, within a small tolerance.
    ///
    /// The tolerance absorbs the rounding that accumulates across a conversion round trip; a
    /// colour a ten-thousandth outside the cube is in gamut for every practical purpose.
    #[must_use]
    pub fn in_gamut(self) -> bool {
        const EPS: f64 = 1e-4;
        [self.r, self.g, self.b]
            .iter()
            .all(|&c| (-EPS..=1.0 + EPS).contains(&c))
    }

    /// Removes the sRGB transfer function, producing linear light.
    #[must_use]
    pub fn to_linear(self) -> LinearRgb {
        LinearRgb {
            r: decode(self.r),
            g: decode(self.g),
            b: decode(self.b),
        }
    }
}

impl LinearRgb {
    /// Creates a linear-light colour.
    #[must_use]
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// Applies the sRGB transfer function, producing an encoded colour.
    #[must_use]
    pub fn to_srgb(self) -> Srgb {
        Srgb {
            r: encode(self.r),
            g: encode(self.g),
            b: encode(self.b),
        }
    }
}

impl fmt::Display for Srgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Why a hex colour could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseHexError {
    /// The string had a length other than 3 or 6 digits.
    WrongLength(usize),
    /// A character was not a hexadecimal digit.
    NotHex(char),
}

impl fmt::Display for ParseHexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongLength(n) => {
                write!(f, "a hex colour has 3 or 6 digits, this one has {n}")
            }
            Self::NotHex(c) => write!(f, "{c:?} is not a hexadecimal digit"),
        }
    }
}

impl core::error::Error for ParseHexError {}

/// The sRGB electro-optical transfer function: encoded value to linear light.
///
/// Sign-preserving, so that out-of-gamut components surviving a conversion stay meaningful
/// rather than collapsing to zero — gamut mapping needs to see how far outside they are.
#[must_use]
pub fn decode(value: f64) -> f64 {
    let a = value.abs();
    let linear = if a <= 0.040_449_936 {
        a / 12.92
    } else {
        ((a + 0.055) / 1.055).powf(2.4)
    };
    linear.copysign(value)
}

/// The sRGB opto-electronic transfer function: linear light to encoded value.
#[must_use]
pub fn encode(value: f64) -> f64 {
    let a = value.abs();
    let encoded = if a <= 0.003_130_8 {
        a * 12.92
    } else {
        1.055 * a.powf(1.0 / 2.4) - 0.055
    };
    encoded.copysign(value)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{ParseHexError, Srgb, decode, encode};

    #[test]
    fn hex_round_trips() {
        for hex in ["#000000", "#FFFFFF", "#B4552C", "#1E232B", "#6B8163"] {
            assert_eq!(Srgb::from_hex(hex).unwrap().to_hex(), hex);
        }
    }

    #[test]
    fn short_hex_expands_by_repetition() {
        assert_eq!(Srgb::from_hex("#abc").unwrap().to_hex(), "#AABBCC");
        assert_eq!(Srgb::from_hex("fff").unwrap().to_hex(), "#FFFFFF");
    }

    #[test]
    fn bad_hex_is_rejected_with_a_useful_reason() {
        assert_eq!(Srgb::from_hex("#12345"), Err(ParseHexError::WrongLength(5)));
        assert_eq!(Srgb::from_hex("#gg0000"), Err(ParseHexError::NotHex('g')));
    }

    #[test]
    fn transfer_function_round_trips() {
        for i in 0..=255 {
            let v = f64::from(i) / 255.0;
            assert!((encode(decode(v)) - v).abs() < 1e-12, "failed at {v}");
        }
    }

    #[test]
    fn transfer_function_matches_known_anchors() {
        assert!((decode(0.0) - 0.0).abs() < 1e-12);
        assert!((decode(1.0) - 1.0).abs() < 1e-12);
        // Mid grey 0.5 encoded is ~0.2140 linear — the classic reminder that 50 % grey is not
        // half the light.
        assert!((decode(0.5) - 0.214_041_14).abs() < 1e-6, "{}", decode(0.5));
    }

    #[test]
    fn transfer_function_preserves_sign() {
        // Out-of-gamut components appear during gamut mapping and must not collapse.
        assert!(decode(-0.2) < 0.0);
        assert!(encode(-0.2) < 0.0);
    }

    #[test]
    fn gamut_membership_tolerates_round_trip_noise() {
        assert!(Srgb::new(0.0, 0.5, 1.0).in_gamut());
        assert!(Srgb::new(-0.000_01, 0.5, 1.000_01).in_gamut());
        assert!(!Srgb::new(-0.01, 0.5, 1.0).in_gamut());
        assert!(!Srgb::new(0.0, 0.5, 1.2).in_gamut());
    }
}
