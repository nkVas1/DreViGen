//! The DreViGen palette, its contrast contract, and the generators that emit it.
//!
//! The palette lives in [`palette`] as OKLCH values, and everything else is derived: the CSS
//! the application loads, the TypeScript constants the canvas imports, and the contrast report.
//! Nothing is hand-maintained in two places, because that is how a design system acquires a
//! colour that is `#B4552C` in one file and `#B45530` in another.
//!
//! The contrast check is the point. [`audit`] evaluates every pairing in every theme and the
//! binary exits non-zero on a failure, so a palette that cannot be read does not reach a
//! branch. A rule enforced by review is a rule that is enforced when there is time.

pub mod css;
pub mod palette;
pub mod report;

use drevigen_color::{Ramp, Verdict, check};
pub use palette::{Pairing, Selector, Theme, Token};

/// One evaluated pairing.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Which theme it was evaluated in.
    pub theme: &'static str,
    /// The pairing.
    pub pairing: Pairing,
    /// Foreground as it will be displayed.
    pub foreground: drevigen_color::Srgb,
    /// Background as it will be displayed.
    pub background: drevigen_color::Srgb,
    /// The measured verdict.
    pub verdict: Verdict,
}

impl Finding {
    /// Whether this pairing meets its standard.
    #[must_use]
    pub fn passes(&self) -> bool {
        self.verdict.passes()
    }
}

/// Evaluates every pairing in every theme.
///
/// # Panics
///
/// Panics if the contract references a token a theme does not define. That is a programming
/// error in [`palette`] rather than a runtime condition, and `palette::tests` catches it first.
#[must_use]
#[allow(
    clippy::panic,
    reason = "the contract and the themes are static data in this crate, and palette::tests proves every reference resolves; a Result would push a case that cannot occur onto every caller"
)]
pub fn audit() -> Vec<Finding> {
    let mut findings = Vec::new();
    for theme in palette::themes() {
        for pairing in &theme.pairings {
            let fg = theme
                .token(pairing.foreground)
                .unwrap_or_else(|| panic!("{}: no token {}", theme.name, pairing.foreground));
            let bg = theme
                .token(pairing.background)
                .unwrap_or_else(|| panic!("{}: no token {}", theme.name, pairing.background));
            let (foreground, background) = (fg.srgb(), bg.srgb());
            findings.push(Finding {
                theme: theme.name,
                pairing: *pairing,
                foreground,
                background,
                verdict: check(foreground, background, pairing.role),
            });
        }
    }
    findings
}

/// Generates the eleven-stop ramps.
#[must_use]
pub fn ramps() -> Vec<Ramp> {
    palette::ramped()
        .into_iter()
        .map(|(name, identity)| Ramp::from_identity(name, identity))
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{audit, ramps};

    #[test]
    fn the_palette_satisfies_its_own_contract() {
        // The test this crate exists for. When it fails, the message names every pairing that
        // is not readable, in which theme, and by how much — so the fix is a palette edit
        // rather than an investigation.
        let findings = audit();
        let failures: Vec<_> = findings.iter().filter(|f| !f.passes()).collect();

        assert!(
            failures.is_empty(),
            "{} pairing(s) fail their standard:\n{}",
            failures.len(),
            failures
                .iter()
                .map(|f| format!(
                    "  {:<9} {:>10} on {:<12} {:<16} Lc {:>6.1} (need {:>4.0})  ratio {:>5.2} (need {:.1})  — {}",
                    f.theme,
                    f.pairing.foreground,
                    f.pairing.background,
                    format!("{:?}", f.pairing.role),
                    f.verdict.apca,
                    f.pairing.role.min_apca(),
                    f.verdict.wcag2,
                    f.pairing.role.min_wcag2(),
                    f.pairing.context,
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn the_audit_covers_every_theme() {
        let findings = audit();
        for theme in ["light", "dark", "contrast"] {
            assert!(
                findings.iter().any(|f| f.theme == theme),
                "{theme} was not audited"
            );
        }
    }

    #[test]
    fn every_ramp_stop_is_displayable() {
        for ramp in ramps() {
            for step in &ramp.steps {
                assert!(
                    step.srgb.in_gamut(),
                    "{} stop {} is out of gamut",
                    ramp.name,
                    step.stop
                );
            }
        }
    }
}
