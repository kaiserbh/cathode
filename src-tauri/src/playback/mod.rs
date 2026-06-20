//! Native embedded playback backend.
//!
//! We link libmpv directly (via `libmpv2`) and drive it through the render API,
//! rather than the plugin's `--wid` window embedding (which does not work on
//! macOS or Wayland). `vo=libmpv` means mpv never creates its own window; video
//! is drawn by a platform render surface (see `macos`, added in a later phase).
//! The UI calls only the playback commands, so this backend is swappable.

use cathode_core::error::AppError;
use libmpv2::Mpv;

/// Owns the libmpv instance. `Mpv` is `Send + Sync`, so this lives in Tauri state
/// behind a shared reference without a mutex.
pub struct Player {
    mpv: Mpv,
}

fn err(context: &str, e: impl std::fmt::Display) -> AppError {
    AppError {
        code: "playback".to_string(),
        message: format!("{context}: {e}"),
    }
}

impl Player {
    /// Create the mpv instance in render-API mode (no auto-created window).
    pub fn new() -> Result<Self, AppError> {
        let mpv = Mpv::with_initializer(|init| {
            // Render through the render API; mpv must not open its own window.
            init.set_property("vo", "libmpv")?;
            init.set_property("hwdec", "auto-safe")?;
            Ok(())
        })
        .map_err(|e| err("mpv init", e))?;
        Ok(Self { mpv })
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
