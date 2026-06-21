//! In-memory capture of `tracing` output for the debug Logs panel.
//!
//! A bounded ring buffer holds the most recent formatted log lines. A `tracing` fmt
//! layer writes into it through [`LogStore`] (each line re-run through
//! [`cathode_core::redact`] as a safety net so no credential survives), gated by a
//! level filter that [`LogControl`] flips at runtime. The user picks the level (`Off`
//! disables capture entirely, with no overhead); the choice is persisted in `Settings`
//! and applied on launch by the frontend.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};

use cathode_core::model::LogLevel;
use cathode_core::redact;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::MakeWriter;

/// How many recent log lines to retain.
const CAPACITY: usize = 1000;

/// A bounded, shared ring buffer of captured log lines. Cheap to clone (it is just an
/// `Arc`), so the same store backs both the tracing writer and the `get_logs` command.
#[derive(Clone, Default)]
pub struct LogStore {
    buf: Arc<Mutex<VecDeque<String>>>,
}

impl LogStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// The captured lines, oldest first.
    pub fn snapshot(&self) -> Vec<String> {
        self.buf
            .lock()
            .map(|b| b.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Drop all captured lines.
    pub fn clear(&self) {
        if let Ok(mut b) = self.buf.lock() {
            b.clear();
        }
    }

    fn push_line(&self, line: String) {
        if let Ok(mut b) = self.buf.lock() {
            if b.len() >= CAPACITY {
                b.pop_front();
            }
            b.push_back(line);
        }
    }
}

/// The per-event sink the fmt layer writes one formatted event into. On drop it splits
/// the buffered bytes into lines, redacts each, and appends them to the store.
pub struct LogWriter {
    store: LogStore,
    buf: Vec<u8>,
}

impl io::Write for LogWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for LogWriter {
    fn drop(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(&self.buf);
        for line in text.lines() {
            if !line.is_empty() {
                self.store.push_line(redact::secrets(line));
            }
        }
    }
}

impl<'a> MakeWriter<'a> for LogStore {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter {
            store: self.clone(),
            buf: Vec::new(),
        }
    }
}

/// Map the user-facing [`LogLevel`] onto a tracing level filter. `Off` disables capture.
pub fn level_filter(level: LogLevel) -> LevelFilter {
    match level {
        LogLevel::Off => LevelFilter::OFF,
        LogLevel::Error => LevelFilter::ERROR,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Debug => LevelFilter::DEBUG,
        LogLevel::Trace => LevelFilter::TRACE,
    }
}

/// A runtime switch for the capture layer's level. Wraps the boxed reload closure so
/// the (verbose) tracing handle type never leaks into the managed-state signature.
pub struct LogControl {
    set: Box<dyn Fn(LogLevel) + Send + Sync>,
}

impl LogControl {
    pub fn new(set: impl Fn(LogLevel) + Send + Sync + 'static) -> Self {
        Self { set: Box::new(set) }
    }

    /// Apply a new level. `Off` stops capture; any other level (re)enables it.
    pub fn set(&self, level: LogLevel) {
        (self.set)(level);
    }
}
