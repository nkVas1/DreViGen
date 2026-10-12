//! Generates the Tauri context: the config, the icons, and the capability schemas.

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Since tauri-build 2.7 the Windows C runtime is linked statically by default, so the app runs
    // on a machine that has never had the Visual C++ redistributable installed — which a release
    // must. It works by putting a stub `msvcrt.lib` on the library search path, and in a workspace
    // test run cargo hands that path to every crate's doctests, which then cannot link. So:
    // static for release builds, the platform's default for everything else.
    let release = std::env::var("PROFILE").is_ok_and(|profile| profile == "release");
    let windows = tauri_build::WindowsAttributes::new().static_vc_runtime(release);
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))?;
    Ok(())
}
