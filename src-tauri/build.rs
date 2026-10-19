/// Generates the Tauri build context for the application crate.
fn main() {
    if embed_manifest_for_all_targets() {
        let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        if let Err(error) = tauri_build::try_build(attributes) {
            println!("cargo:warning=tauri-build failed: {error:#}");
            std::process::exit(1);
        }
    } else {
        tauri_build::build()
    }
}

/// tauri-build embeds its app manifest (Common Controls v6) into the app exe
/// only, so test binaries load Common Controls v5 and fail to start with
/// STATUS_ENTRYPOINT_NOT_FOUND. On MSVC, embed the same manifest into every
/// target (exe and tests) through the linker instead. Returns true if it did.
fn embed_manifest_for_all_targets() -> bool {
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        return false;
    };
    if !(windows && msvc) {
        return false;
    }
    let manifest = std::path::Path::new(&dir).join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed=windows-app-manifest.xml");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    true
}
