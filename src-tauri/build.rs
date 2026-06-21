fn main() {
    // libmpv2-sys emits `-lmpv` but no search path; Homebrew's lib dir isn't on
    // the default linker path. Point the linker at it (Apple Silicon or Intel).
    // libmpv.2.dylib has an absolute install name, so runtime resolution is fine.
    #[cfg(target_os = "macos")]
    {
        for dir in ["/opt/homebrew/lib", "/usr/local/lib"] {
            if std::path::Path::new(dir).join("libmpv.dylib").exists() {
                println!("cargo:rustc-link-search=native={dir}");
                break;
            }
        }
        // The mpv render API draws via OpenGL; we resolve GL symbols out of the
        // (deprecated but still functional) OpenGL framework via dlsym.
        println!("cargo:rustc-link-lib=framework=OpenGL");
    }

    // libmpv2-sys emits `-lmpv` with no search path. We vendor the Windows libmpv under
    // `vendor/mpv/windows-x64` (the 112 MB DLL via Git LFS, plus a generated `mpv.lib`
    // import library) so contributors need no manual setup beyond `git lfs`.
    #[cfg(target_os = "windows")]
    {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let vendor = std::path::Path::new(&manifest).join("vendor/mpv/windows-x64");
        println!("cargo:rustc-link-search=native={}", vendor.display());
        println!(
            "cargo:rerun-if-changed={}",
            vendor.join("mpv.lib").display()
        );

        // `mpv.lib` satisfies the linker; `libmpv-2.dll` must sit next to the built
        // binaries to load at runtime. Windows only searches the executable's own
        // directory, so place it both in the profile dir (the app binary) and `deps/`
        // (test/example binaries, which link libmpv and won't even load without it).
        let dll = vendor.join("libmpv-2.dll");
        let out_dir = std::env::var("OUT_DIR").unwrap();
        if let Some(profile) = std::path::Path::new(&out_dir).ancestors().nth(3) {
            for dir in [profile.to_path_buf(), profile.join("deps")] {
                let _ = std::fs::create_dir_all(&dir);
                let dest = dir.join("libmpv-2.dll");
                // Skip the 112 MB copy when an identical DLL is already present.
                let stale = std::fs::metadata(&dest)
                    .ok()
                    .zip(std::fs::metadata(&dll).ok())
                    .is_none_or(|(d, s)| d.len() != s.len());
                if stale {
                    let _ = std::fs::copy(&dll, &dest);
                }
            }
        }
    }

    tauri_build::build()
}
