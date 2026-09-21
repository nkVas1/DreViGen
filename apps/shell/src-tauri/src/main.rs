//! The desktop entry point. Mobile enters through `lib.rs` instead.

// Without this a release build on Windows opens a console window behind the application.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    drevigen_shell_lib::run();
}
