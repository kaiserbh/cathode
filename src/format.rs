//! Small display-formatting helpers shared across components.

use js_sys::Date;

/// Format a Unix-seconds timestamp as local `HH:MM` (24-hour).
pub fn hhmm(unix_secs: i64) -> String {
    let date = Date::new(&((unix_secs as f64) * 1000.0).into());
    format!("{:02}:{:02}", date.get_hours(), date.get_minutes())
}

/// The current time as Unix seconds.
pub fn now_unix() -> i64 {
    (Date::now() / 1000.0) as i64
}

/// Convert a slider's raw `f64` position into a clamped 0–100 volume. The
/// dioxus-primitives `Slider` reports `f64`; the player models volume as `u8`.
pub fn volume_from_slider(value: f64) -> u8 {
    value.round().clamp(0.0, 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_rounds_and_clamps() {
        assert_eq!(volume_from_slider(0.0), 0);
        assert_eq!(volume_from_slider(50.4), 50);
        assert_eq!(volume_from_slider(50.5), 51);
        assert_eq!(volume_from_slider(100.0), 100);
        // Out-of-range inputs clamp rather than wrap or panic on the cast.
        assert_eq!(volume_from_slider(-5.0), 0);
        assert_eq!(volume_from_slider(150.0), 100);
    }
}
