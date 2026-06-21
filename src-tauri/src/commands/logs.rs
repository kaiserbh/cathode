//! Debug-log commands: read the captured buffer, clear it, and change the live level.

use cathode_core::error::AppError;
use cathode_core::model::LogLevel;
use tauri::State;

use crate::logs::{LogControl, LogStore};

/// The captured log lines, oldest first.
#[tauri::command]
pub fn get_logs(logs: State<'_, LogStore>) -> Result<Vec<String>, AppError> {
    Ok(logs.snapshot())
}

/// Drop all captured log lines.
#[tauri::command]
pub fn clear_logs(logs: State<'_, LogStore>) -> Result<(), AppError> {
    logs.clear();
    Ok(())
}

/// Set the capture level live. `Off` disables capture (no overhead).
#[tauri::command]
pub fn set_log_level(control: State<'_, LogControl>, level: LogLevel) -> Result<(), AppError> {
    control.set(level);
    Ok(())
}
