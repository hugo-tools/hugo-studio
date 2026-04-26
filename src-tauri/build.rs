fn main() {
    // Expose the build target triple so the sidecar resolver can find
    // the right `binaries/hugo-<triple>(.exe)` in dev builds.
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown-triple".into());
    println!("cargo:rustc-env=TARGET_TRIPLE={target}");
    println!("cargo:rerun-if-env-changed=TARGET");

    tauri_build::build()
}
