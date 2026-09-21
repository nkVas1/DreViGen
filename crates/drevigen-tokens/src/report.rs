//! The contrast report, written so a failure tells you what to change.
//!
//! A gate that prints "contrast check failed" teaches nobody anything and gets disabled. This
//! one names the pairing, the theme, where it appears in the product, what it measured and
//! what it needed — so the next step is a palette edit rather than an investigation.

use core::fmt::Write as _;

use crate::Finding;

/// Formats the full audit as a table.
#[must_use]
pub fn table(findings: &[Finding]) -> String {
    let mut out = String::with_capacity(4 * 1024);

    out.push_str("Contrast audit — APCA is the guide, WCAG 2 the conformance report.\n\n");
    let _ = writeln!(
        out,
        "{:<9} {:>10} on {:<12} {:<15} {:>8} {:>6} {:>8} {:>6}",
        "theme", "foreground", "background", "role", "Lc", "need", "ratio", "need"
    );
    out.push_str(&"─".repeat(104));
    out.push('\n');

    let mut current = "";
    for f in findings {
        if f.theme != current {
            if !current.is_empty() {
                out.push('\n');
            }
            current = f.theme;
        }
        let _ = writeln!(
            out,
            "{:<9} {:>10} on {:<12} {:<15} {:>8.1} {:>6.0} {:>8.2} {:>6.1}  {}",
            f.theme,
            f.pairing.foreground,
            f.pairing.background,
            format!("{:?}", f.pairing.role),
            f.verdict.apca,
            f.pairing.role.min_apca(),
            f.verdict.wcag2,
            f.pairing.role.min_wcag2(),
            if f.passes() { "" } else { "◀ FAILS" },
        );
    }

    let failures = findings.iter().filter(|f| !f.passes()).count();
    out.push('\n');
    if failures == 0 {
        let _ = writeln!(out, "All {} pairings pass.", findings.len());
    } else {
        let _ = writeln!(out, "{failures} of {} pairings fail.\n", findings.len());
        for f in findings.iter().filter(|f| !f.passes()) {
            let _ = writeln!(
                out,
                "  {} · {} on {} ({})",
                f.theme, f.pairing.foreground, f.pairing.background, f.pairing.context
            );
            if !f.verdict.passes_apca() {
                let _ = writeln!(
                    out,
                    "      APCA   Lc {:.1}, needs {:.0} — {} {} by {:.1}",
                    f.verdict.apca,
                    f.pairing.role.min_apca(),
                    f.pairing.foreground,
                    if f.verdict.apca.is_sign_positive() {
                        "is too light"
                    } else {
                        "is too dark"
                    },
                    f.pairing.role.min_apca() - f.verdict.apca.abs(),
                );
            }
            if !f.verdict.passes_wcag2() {
                let _ = writeln!(
                    out,
                    "      WCAG 2 ratio {:.2}, needs {:.1}",
                    f.verdict.wcag2,
                    f.pairing.role.min_wcag2(),
                );
            }
            let _ = writeln!(
                out,
                "      {} {} on {} {}",
                f.pairing.foreground,
                f.foreground.to_hex(),
                f.pairing.background,
                f.background.to_hex(),
            );
        }
    }

    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::table;
    use crate::audit;

    #[test]
    fn the_report_names_every_theme_and_pairing() {
        let findings = audit();
        let text = table(&findings);
        for theme in ["light", "dark", "contrast"] {
            assert!(text.contains(theme), "{theme} missing from the report");
        }
        assert!(text.contains("sanguine"));
        assert!(text.contains("vellum"));
    }

    #[test]
    fn a_passing_audit_says_so_plainly() {
        let findings = audit();
        let text = table(&findings);
        if findings.iter().all(super::Finding::passes) {
            assert!(text.contains("All"), "a clean audit should say so");
            assert!(!text.contains("FAILS"));
        }
    }
}
