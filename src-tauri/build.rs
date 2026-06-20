fn main() {
    // libmpv2-sys emits `-lmpv` but no search path; Homebrew's lib dir isn't on
    // the default linker path. Point the linker at it (Apple Silicon or Intel).
    // libmpv.2.dylib has an absolute install name, so runtime resolution is fine.
    #[cfg(target_os = "macos")]
    for dir in ["/opt/homebrew/lib", "/usr/local/lib"] {
        if std::path::Path::new(dir).join("libmpv.dylib").exists() {
            println!("cargo:rustc-link-search=native={dir}");
            break;
        }
    }

    tauri_build::build()
}
