fn main() {
    tauri_build::build();

    // The libmpv-wrapper `dlopen`s "libmpv.dylib" by leaf name, and the plugin
    // loads the wrapper from the executable's directory. `cargo run` (used by
    // `cargo tauri dev`) injects its own DYLD_FALLBACK_LIBRARY_PATH, so an env
    // var can't be relied on; instead we drop both dylibs next to the binary,
    // which is on the runtime search path. Bundling a standalone .app is separate.
    #[cfg(target_os = "macos")]
    place_macos_dylibs();
}

#[cfg(target_os = "macos")]
fn place_macos_dylibs() {
    use std::path::{Path, PathBuf};

    // OUT_DIR = <target>/<profile>/build/cathode-<hash>/out; 3rd ancestor is the
    // profile dir where the binary lands.
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    let Some(target_dir) = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
    else {
        println!("cargo:warning=could not derive target dir from OUT_DIR; mpv dylibs not placed");
        return;
    };

    // The bundled wrapper (committed in this crate).
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set");
    let wrapper_src = Path::new(&manifest_dir).join("lib/libmpv-wrapper.dylib");
    link_into(&wrapper_src, &target_dir.join("libmpv-wrapper.dylib"));
    println!("cargo:rerun-if-changed=lib/libmpv-wrapper.dylib");

    // Homebrew's libmpv (system-installed via `brew install mpv`).
    let libmpv_src = [
        "/opt/homebrew/lib/libmpv.dylib",
        "/usr/local/lib/libmpv.dylib",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|p| p.exists());
    match libmpv_src {
        Some(src) => link_into(&src, &target_dir.join("libmpv.dylib")),
        None => println!(
            "cargo:warning=libmpv.dylib not found in Homebrew prefixes; run `brew install mpv`"
        ),
    }
}

/// Force-create a symlink at `dst` pointing to `src` (replacing any existing one).
#[cfg(target_os = "macos")]
fn link_into(src: &std::path::Path, dst: &std::path::Path) {
    use std::os::unix::fs::symlink;
    if !src.exists() {
        println!(
            "cargo:warning=missing {}; mpv may fail to load at runtime",
            src.display()
        );
        return;
    }
    let _ = std::fs::remove_file(dst);
    if let Err(e) = symlink(src, dst) {
        println!("cargo:warning=failed to symlink {}: {e}", dst.display());
    }
}
