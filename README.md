# Cathode

## Platform setup for playback (libmpv)

Cathode links libmpv directly (the `libmpv2` crate). `libmpv2-sys` emits `-lmpv` without a
search path, so each platform has to point the linker at its libmpv. `src-tauri/build.rs`
handles this per-OS.

### macOS

```sh
brew install mpv
```

`libmpv2-sys` finds it via pkg-config and the build script adds the Homebrew lib dir to the
linker search path. Nothing else is needed.

### Windows (x64)

libmpv and ANGLE are **vendored** in this repo under `src-tauri/vendor/mpv/windows-x64/` —
the 112 MB `libmpv-2.dll` is stored via [Git LFS](https://git-lfs.com/), alongside a
generated `mpv.lib` import library and ANGLE's `libEGL.dll` / `libGLESv2.dll`. So the only
prerequisite is Git LFS:

```sh
git lfs install          # one-time, per machine
git clone <repo>         # pulls the DLLs automatically
# (already cloned without LFS? run `git lfs pull`)
```

`src-tauri/build.rs` points the linker at the vendored `mpv.lib` and copies the runtime DLLs
next to the built binaries, so `cargo tauri dev` / `cargo build` work with no further setup.
If you see `LINK : fatal error LNK1181: cannot open input file 'mpv.lib'`, the LFS files
were not fetched — run `git lfs pull`.

Video renders via the libmpv OpenGL render API on an ANGLE (EGL → Direct3D 11) context. The
surface is a separate top-level window glued directly behind the transparent Tauri window
(`src-tauri/src/playback/windows.rs`); rendering runs on a dedicated thread so the UI stays
responsive.
