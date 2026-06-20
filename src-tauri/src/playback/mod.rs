//! Native embedded playback backend.
//!
//! We link libmpv directly (via `libmpv2`) and drive it through the render API,
//! rather than the plugin's `--wid` window embedding (which does not work on
//! macOS or Wayland). `vo=libmpv` means mpv never creates its own window; video
//! is drawn by a platform render surface (see `macos`). The UI calls only the
//! playback commands, so this backend is swappable.

#[cfg(target_os = "macos")]
pub mod macos;

use cathode_core::error::AppError;
use libmpv2::Mpv;

/// Holds the libmpv instance for the command layer. `Mpv` is `Send + Sync`, so
/// this lives in Tauri state behind a shared reference without a mutex.
///
/// The handle is a `&'static Mpv` (see [`create_mpv`]): the render context borrows
/// the `Mpv`, so leaking it lets the surface keep a `RenderContext<'static>`.
pub struct Player {
    mpv: &'static Mpv,
}

fn err(context: &str, e: impl std::fmt::Display) -> AppError {
    AppError {
        code: "playback".to_string(),
        message: format!("{context}: {e}"),
    }
}

/// Create the mpv instance in render-API mode (no auto-created window) and leak it
/// for the app's lifetime, yielding a `&'static Mpv` shared by [`Player`] and the
/// platform video surface.
pub fn create_mpv() -> Result<&'static Mpv, AppError> {
    let mpv = Mpv::with_initializer(|init| {
        // Render through the render API; mpv must not open its own window.
        init.set_property("vo", "libmpv")?;
        init.set_property("hwdec", "auto-safe")?;
        Ok(())
    })
    .map_err(|e| err("mpv init", e))?;
    Ok(Box::leak(Box::new(mpv)))
}

impl Player {
    /// Wrap a (leaked) mpv handle for use by the playback commands.
    pub fn new(mpv: &'static Mpv) -> Self {
        Self { mpv }
    }

    /// Load and play a URL, replacing anything currently playing.
    pub fn load(&self, url: &str) -> Result<(), AppError> {
        self.mpv
            .command("loadfile", &[url, "replace"])
            .map_err(|e| err("loadfile", e))?;
        self.set_pause(false)
    }

    pub fn set_pause(&self, paused: bool) -> Result<(), AppError> {
        self.mpv
            .set_property("pause", paused)
            .map_err(|e| err("set pause", e))
    }

    pub fn stop(&self) -> Result<(), AppError> {
        self.mpv.command("stop", &[]).map_err(|e| err("stop", e))
    }
}
