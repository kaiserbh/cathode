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

libmpv is **vendored** in this repo under `src-tauri/vendor/mpv/windows-x64/` — the 112 MB
`libmpv-2.dll` is stored via [Git LFS](https://git-lfs.com/), alongside a generated `mpv.lib`
import library. So the only prerequisite is Git LFS:

```sh
git lfs install          # one-time, per machine
git clone <repo>         # pulls the DLL automatically
# (already cloned without LFS? run `git lfs pull`)
```

`src-tauri/build.rs` points the linker at the vendored `mpv.lib` and copies `libmpv-2.dll`
next to the built binaries, so `cargo tauri dev` / `cargo build` work with no further setup.
If you see `LINK : fatal error LNK1181: cannot open input file 'mpv.lib'`, the LFS files
were not fetched — run `git lfs pull`.

> **Note:** Windows video rendering is not implemented yet — the libmpv render surface
> currently exists only for macOS. Once the build links, the app launches and audio playback
> works, but video will not display on Windows until a platform render surface is added.
