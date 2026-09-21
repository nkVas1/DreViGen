//! The native shell.
//!
//! This crate is deliberately thin. It opens a window, points it at the same front end the web
//! build serves, and does the three things a browser tab cannot: remember where the window was,
//! refuse to open twice, and write a log file the user can send us.
//!
//! Everything about genealogy lives elsewhere. The rule is that the shell may not become a
//! place where behaviour hides: if a feature works in the Tauri build and not in the browser,
//! it belongs in the front end or in the core, not here.
//!
//! ## Why a single instance matters here
//!
//! The archive is one SQLite database with one writer (ADR 0008). Two windows would be two
//! processes racing for the same file. On desktop the second launch focuses the first window
//! instead; on mobile the platform already guarantees it.

/// Runs the application. Called by `main.rs` on desktop and by the platform on mobile.
///
/// # Panics
///
/// If the webview cannot be created — no WebView2 on Windows, no WebKitGTK on Linux — there is
/// no window in which to show an error, so the process exits with the reason on stderr.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(
    clippy::expect_used,
    reason = "there is no UI in which to report this failure"
)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(desktop)]
    let builder = builder
        // Registered first, as the plugin requires: it has to see the launch before anything
        // else does. The closure runs in the *original* process when a second one starts.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_main_window(app);
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build());

    builder
        .plugin(log_plugin())
        .run(tauri::generate_context!())
        .expect("the webview could not be created");
}

/// Brings the existing window forward when a second launch is attempted.
#[cfg(desktop)]
fn focus_main_window(app: &tauri::AppHandle) {
    use tauri::Manager as _;

    if let Some(window) = app.get_webview_window("main") {
        // Order matters: an unminimised window can still be behind others.
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Logging: to stderr while developing, and to a rotating file in the platform log directory
/// otherwise, because "it did something odd" is the bug report we will actually receive.
fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_log::{Target, TargetKind};

    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    tauri_plugin_log::Builder::default()
        .level(level)
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
        ])
        .build()
}
