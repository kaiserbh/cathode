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
