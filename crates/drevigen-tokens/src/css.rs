//! Emitting the palette as CSS and as TypeScript.
//!
//! The cascade structure is not a stylistic choice — it is the three-state problem the page
//! contract describes. A viewer has an explicit light choice, an explicit dark choice, or no
//! choice at all, and in the last case only `prefers-color-scheme` separates them. So:
//!
//! 1. bare `:root` carries the **complete** light palette;
//! 2. `@media (prefers-color-scheme: dark)` redefines the tokens, guarded by
//!    `:root:not([data-theme="light"])` so an explicit light choice beats a dark system;
//! 3. `:root[data-theme="dark"]` redefines them again so the toggle wins in both directions.
//!
//! A token whose only definition sits inside a media query or a `[data-theme]` block never
//! applies in the unstamped state, which renders one theme's text on the other theme's ground.

use core::fmt::Write as _;

use crate::palette::{Selector, Theme};
use crate::ramps;

/// Generates the stylesheet.
#[must_use]
pub fn stylesheet(themes: &[Theme]) -> String {
    let mut out = String::with_capacity(16 * 1024);

    out.push_str(
        "/* DreViGen design tokens — GENERATED, do not edit.\n\
         *\n\
         * Source: crates/drevigen-tokens/src/palette.rs\n\
         * Regenerate: cargo run -p drevigen-tokens -- generate\n\
         *\n\
         * Every colour is authored in OKLCH and emitted twice: the oklch() value a modern\n\
         * browser uses directly, and a gamut-mapped hex fallback computed with the same CSS\n\
         * Color 4 algorithm the browser applies, so the two agree.\n\
         */\n\n",
    );

    for theme in themes {
        let (open, close) = match theme.selector {
            Selector::Root => (":root {".to_owned(), "}".to_owned()),
            Selector::PrefersDark => (
                "@media (prefers-color-scheme: dark) {\n  :root:not([data-theme=\"light\"]) {"
                    .to_owned(),
                "  }\n}".to_owned(),
            ),
            Selector::Stamped(name) => (format!("[data-theme=\"{name}\"] {{"), "}".to_owned()),
        };
        let indent = if matches!(theme.selector, Selector::PrefersDark) {
            "    "
        } else {
            "  "
        };

        let _ = writeln!(out, "/* {} — «{}» */", theme.name, theme.title);
        let _ = writeln!(out, "{open}");
        for token in &theme.tokens {
            let _ = writeln!(
                out,
                "{indent}--dv-{:<14} {:<30} /* {} · {} */",
                format!("{}:", token.name),
                format!("{};", token.oklch.to_css()),
                token.srgb().to_hex(),
                token.purpose,
            );
        }
        let _ = writeln!(out, "{close}\n");

        // The dark theme needs its stamped twin so an explicit choice wins in both directions.
        if matches!(theme.selector, Selector::PrefersDark) {
            let _ = writeln!(out, "/* {} — explicit choice */", theme.name);
            let _ = writeln!(out, "[data-theme=\"{}\"] {{", theme.name);
            for token in &theme.tokens {
                let _ = writeln!(
                    out,
                    "  --dv-{:<14} {};",
                    format!("{}:", token.name),
                    token.oklch.to_css()
                );
            }
            let _ = writeln!(out, "}}\n");
        }
    }

    out.push_str("/* Eleven-stop ramps. Theme-independent: a ramp is a scale, not a mood. */\n");
    out.push_str(":root {\n");
    for ramp in ramps() {
        for step in &ramp.steps {
            let _ = writeln!(
                out,
                "  --dv-{:<18} {:<30} /* {} */",
                format!("{}-{}:", ramp.name, step.stop),
                format!("{};", step.oklch.to_css()),
                step.srgb.to_hex(),
            );
        }
    }
    out.push_str("}\n");

    out
}

/// Generates the TypeScript constants.
///
/// Hex rather than `oklch()`: these are consumed by the canvas, which uploads colours to a
/// shader as floats, and by anything that has to compute with a colour rather than declare it.
#[must_use]
pub fn typescript(themes: &[Theme]) -> String {
    let mut out = String::with_capacity(8 * 1024);

    out.push_str(
        "// DreViGen design tokens — GENERATED, do not edit.\n\
         //\n\
         // Source: crates/drevigen-tokens/src/palette.rs\n\
         // Regenerate: cargo run -p drevigen-tokens -- generate\n\n",
    );

    let _ = writeln!(
        out,
        "export type ThemeName = {};\n",
        themes
            .iter()
            .map(|t| format!("'{}'", t.name))
            .collect::<Vec<_>>()
            .join(" | ")
    );

    if let Some(first) = themes.first() {
        let _ = writeln!(
            out,
            "export type TokenName =\n{};\n",
            first
                .tokens
                .iter()
                .map(|t| format!("  | '{}'", t.name))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    out.push_str("export const tokens: Record<ThemeName, Record<TokenName, string>> = {\n");
    for theme in themes {
        let _ = writeln!(out, "  {}: {{", theme.name);
        for token in &theme.tokens {
            let _ = writeln!(
                out,
                "    {:<14} '{}', // {}",
                format!("{}:", token.name),
                token.srgb().to_hex(),
                token.purpose
            );
        }
        out.push_str("  },\n");
    }
    out.push_str("};\n\n");

    out.push_str("export const ramps = {\n");
    for ramp in ramps() {
        let _ = writeln!(out, "  {}: {{", ramp.name);
        for step in &ramp.steps {
            let _ = writeln!(out, "    {}: '{}',", step.stop, step.srgb.to_hex());
        }
        out.push_str("  },\n");
    }
    out.push_str("} as const;\n");

    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::{stylesheet, typescript};
    use crate::palette::themes;

    #[test]
    fn light_tokens_are_defined_on_bare_root() {
        // The bug this guards against renders one theme's text on the other theme's ground.
        let css = stylesheet(&themes());
        let root = css.split(":root {").nth(1).unwrap();
        let block = root.split('}').next().unwrap();
        for token in crate::palette::light() {
            assert!(
                block.contains(&format!("--dv-{}:", token.name)),
                "{} is not defined on bare :root",
                token.name
            );
        }
    }

    #[test]
    fn the_dark_theme_is_emitted_both_ways() {
        let css = stylesheet(&themes());
        assert!(css.contains("@media (prefers-color-scheme: dark)"));
        assert!(
            css.contains(":root:not([data-theme=\"light\"])"),
            "the media query must be guarded, or a dark system beats an explicit light choice"
        );
        assert!(
            css.contains("[data-theme=\"dark\"]"),
            "the toggle must win in both directions"
        );
    }

    #[test]
    fn every_token_carries_a_hex_fallback_in_a_comment() {
        let css = stylesheet(&themes());
        for token in crate::palette::light() {
            let hex = token.srgb().to_hex();
            assert!(css.contains(&hex), "{} has no hex for {}", token.name, hex);
        }
    }

    #[test]
    fn ramps_reach_the_stylesheet() {
        let css = stylesheet(&themes());
        assert!(css.contains("--dv-sanguine-500:"));
        assert!(css.contains("--dv-madder-950:"));
    }

    #[test]
    fn typescript_declares_a_union_of_theme_names() {
        let ts = typescript(&themes());
        assert!(ts.contains("export type ThemeName = 'light' | 'dark' | 'contrast';"));
        assert!(ts.contains("export type TokenName ="));
        assert!(ts.contains("  | 'sanguine'"));
    }

    #[test]
    fn typescript_emits_every_theme_and_ramp() {
        let ts = typescript(&themes());
        for theme in ["light:", "dark:", "contrast:"] {
            assert!(ts.contains(theme), "{theme} missing from the TypeScript");
        }
        assert!(ts.contains("export const ramps"));
    }

    #[test]
    fn every_declaration_is_terminated() {
        // The first version of this generator omitted the semicolon and produced a stylesheet
        // in which one malformed declaration swallowed the rest of its block. CSS comments are
        // stripped before parsing, so nothing about the output looked wrong.
        for line in stylesheet(&themes()).lines() {
            let code = line.split("/*").next().unwrap_or("").trim_end();
            if code.trim_start().starts_with("--dv-") {
                assert!(
                    code.ends_with(';'),
                    "unterminated custom property: {}",
                    line.trim()
                );
            }
        }
    }

    #[test]
    fn braces_balance() {
        let css = stylesheet(&themes());
        let opens = css.matches('{').count();
        let closes = css.matches('}').count();
        assert_eq!(
            opens, closes,
            "unbalanced braces: {opens} open, {closes} close"
        );
    }

    #[test]
    fn generated_files_announce_that_they_are_generated() {
        assert!(stylesheet(&themes()).contains("GENERATED, do not edit"));
        assert!(typescript(&themes()).contains("GENERATED, do not edit"));
    }
}
