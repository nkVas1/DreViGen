//! Colouring the parts of the window the web page cannot reach.
//!
//! On Windows 11 the title bar is painted by the desktop compositor, and by default it takes
//! the accent colour the user chose for the whole system. That is the right default for a tool
//! and the wrong one for this: a bright accent bar above a vellum page reads as a web page
//! someone put in a window. `DwmSetWindowAttribute` lets an application say otherwise, so the
//! caption continues the chrome instead of interrupting it.
//!
//! Colours come from [`drevigen_tokens`], the same source the stylesheet is generated from, so
//! the title bar and the header below it cannot drift apart.
//!
//! Everything here is best-effort. The attributes exist from Windows 11 build 22000; on
//! Windows 10 the call fails and the system title bar is used, which is correct rather than
//! degraded. On every other platform this module is empty.

use drevigen_tokens::palette;
use tauri::{Theme, WebviewWindow};

/// Which palette the page is showing.
///
/// Not the same question as "is the system in dark mode". Someone can run a light desktop and
/// choose the lamplit theme, or the high-contrast one, and the frame has to follow the page
/// rather than the desktop — which is why the front end tells us instead of us guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dressing {
    /// Ink on vellum.
    Light,
    /// Warm light on a lamplit ground.
    Dark,
    /// Black on white, no texture.
    Contrast,
}

impl Dressing {
    /// What the page chose, given the name the front end uses for it and the system theme it
    /// would fall back to.
    pub(crate) fn parse(choice: &str, system: Theme) -> Self {
        match choice {
            "light" => Self::Light,
            "dark" => Self::Dark,
            "contrast" => Self::Contrast,
            // "system", or anything a newer front end sends that this shell does not know.
            _ => Self::from(system),
        }
    }

    fn tokens(self) -> Vec<palette::Token> {
        match self {
            Self::Light => palette::light(),
            Self::Dark => palette::dark(),
            Self::Contrast => palette::high_contrast(),
        }
    }
}

impl From<Theme> for Dressing {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::Dark,
            // `Theme` is non-exhaustive: an unknown future variant should look like the page's
            // default rather than fall through to nothing.
            _ => Self::Light,
        }
    }
}

/// Paints the window chrome to match what the page is showing.
pub(crate) fn paint(window: &WebviewWindow, dressing: Dressing) {
    let tokens = dressing.tokens();

    let caption = colorref(&tokens, "vellum-deep");
    let text = colorref(&tokens, "ink");
    let border = colorref(&tokens, "rule");

    apply(window, caption, text, border);
}

/// Reads a palette token as a Win32 `COLORREF`: `0x00BBGGRR`, not the RGB order everyone
/// expects, which is exactly the sort of detail worth writing down once.
fn colorref(tokens: &[palette::Token], name: &str) -> u32 {
    let Some(srgb) = tokens
        .iter()
        .find(|t| t.name == name)
        .map(palette::Token::srgb)
    else {
        return 0;
    };
    let channel = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    channel(srgb.b) << 16 | channel(srgb.g) << 8 | channel(srgb.r)
}

#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "DwmSetWindowAttribute is the only way to colour the caption, and there is no safe wrapper for it in the dependency tree"
)]
fn apply(window: &WebviewWindow, caption: u32, text: u32, border: u32) {
    use core::ffi::c_void;

    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DwmSetWindowAttribute,
    };

    let Ok(handle) = window.hwnd() else {
        log::warn!("no window handle: the title bar keeps the system colour");
        return;
    };
    let hwnd = handle.0.cast::<c_void>();

    // The constants are declared as the signed `DWMWINDOWATTRIBUTE` enum but the function
    // takes them unsigned, so the cast happens once, here.
    for (attribute, value) in [
        (DWMWA_CAPTION_COLOR as u32, caption),
        (DWMWA_TEXT_COLOR as u32, text),
        (DWMWA_BORDER_COLOR as u32, border),
    ] {
        // SAFETY: `hwnd` is a live window handle owned by this process for the duration of the
        // call, and each attribute takes exactly one `u32` — the size passed is that of the
        // value being pointed at.
        let result = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute,
                std::ptr::from_ref(&value).cast::<c_void>(),
                u32::try_from(size_of::<u32>()).unwrap_or(4),
            )
        };
        if result != 0 {
            // Windows 10 does not know these attributes. Not an error: the system title bar is
            // a correct outcome, and saying so once at debug level is enough.
            log::debug!("the compositor declined attribute {attribute}: 0x{result:08X}");
        }
    }
}

/// Everywhere else the window manager owns the frame, and an application that repaints it is
/// the application that looks wrong on the next desktop environment.
#[cfg(not(windows))]
#[allow(
    clippy::needless_pass_by_value,
    reason = "matches the Windows signature"
)]
fn apply(_window: &WebviewWindow, _caption: u32, _text: u32, _border: u32) {}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{Dressing, colorref};
    use drevigen_tokens::palette;
    use tauri::Theme;

    #[test]
    fn a_colorref_is_bgr_not_rgb() {
        // Win32 packs 0x00BBGGRR. Getting this backwards produces a plausible-looking colour,
        // which is the kind of bug that survives review, so it is pinned here.
        let tokens = palette::light();
        let sanguine = tokens
            .iter()
            .find(|t| t.name == "sanguine")
            .map(palette::Token::srgb)
            .expect("the light palette defines sanguine");

        let packed = colorref(&tokens, "sanguine");
        let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;

        assert_eq!(
            packed & 0xFF,
            byte(sanguine.r),
            "red belongs in the low byte"
        );
        assert_eq!(
            (packed >> 16) & 0xFF,
            byte(sanguine.b),
            "blue in the high byte"
        );
        assert_eq!(packed >> 24, 0, "the top byte must stay clear");
    }

    #[test]
    fn an_unknown_token_does_not_paint_something_arbitrary() {
        assert_eq!(colorref(&palette::light(), "no-such-token"), 0);
    }

    #[test]
    fn the_page_wins_over_the_desktop() {
        // A light desktop with the lamplit theme chosen is the case this whole module exists
        // for: the frame must follow the page.
        assert_eq!(Dressing::parse("dark", Theme::Light), Dressing::Dark);
        assert_eq!(Dressing::parse("light", Theme::Dark), Dressing::Light);
        assert_eq!(Dressing::parse("contrast", Theme::Dark), Dressing::Contrast);
    }

    #[test]
    fn following_the_system_means_following_the_system() {
        assert_eq!(Dressing::parse("system", Theme::Dark), Dressing::Dark);
        assert_eq!(Dressing::parse("system", Theme::Light), Dressing::Light);
        // A theme a newer front end knows about and this shell does not.
        assert_eq!(Dressing::parse("sepia-1974", Theme::Dark), Dressing::Dark);
    }

    #[test]
    fn every_theme_defines_the_three_tokens_the_frame_needs() {
        for dressing in [Dressing::Light, Dressing::Dark, Dressing::Contrast] {
            let tokens = dressing.tokens();
            for name in ["vellum-deep", "ink", "rule"] {
                assert!(
                    tokens.iter().any(|t| t.name == name),
                    "{dressing:?} has no {name}"
                );
            }
        }
    }
}
