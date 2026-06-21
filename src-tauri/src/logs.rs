//! In-memory capture of `tracing` events for the debug Logs panel.
//!
//! A bounded ring buffer holds the most recent events as structured [`LogLine`]s
//! (time, level, target, message) rather than pre-formatted text — so the UI can color
//! and align them, and so no ANSI escapes or span-field noise leak in. A custom layer
//! feeds the buffer, gated by a [`Targets`] filter that [`LogControl`] swaps at runtime:
//! capture is scoped to Cathode's own crates (dependency spam like hyper/reqwest/tao is
//! excluded), and `Off` captures nothing. The message is credential-redacted.

use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use cathode_core::model::{LogLevel, LogLine};
use cathode_core::redact;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::{Context, Layer};

/// How many recent log lines to retain.
const CAPACITY: usize = 1000;

/// A bounded, shared ring buffer of captured log lines. Cheap to clone (just an `Arc`),
/// so the same store backs both the capture layer and the `get_logs` command.
#[derive(Clone, Default)]
pub struct LogStore {
    buf: Arc<Mutex<VecDeque<LogLine>>>,
}

impl LogStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// The captured lines, oldest first.
    pub fn snapshot(&self) -> Vec<LogLine> {
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

    fn push(&self, line: LogLine) {
        if let Ok(mut b) = self.buf.lock() {
            if b.len() >= CAPACITY {
                b.pop_front();
            }
            b.push_back(line);
        }
    }
}

/// Collects an event's message and any extra fields into displayable strings.
#[derive(Default)]
struct EventVisitor {
    message: String,
    fields: String,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            self.fields.push_str(&format!("{}={value:?}", field.name()));
        }
    }
}

/// A `tracing` layer that turns each event into a [`LogLine`] and stores it.
pub struct CaptureLayer {
    store: LogStore,
}

impl CaptureLayer {
    pub fn new(store: LogStore) -> Self {
        Self { store }
    }
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        let message = match (visitor.message.is_empty(), visitor.fields.is_empty()) {
            (true, _) => visitor.fields,
            (false, true) => visitor.message,
            (false, false) => format!("{} {}", visitor.message, visitor.fields),
        };
        let meta = event.metadata();
        self.store.push(LogLine {
            time: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            level: meta.level().to_string().to_lowercase(),
            target: meta.target().to_string(),
            message: redact::secrets(&message),
        });
    }
}

/// The capture filter for a level. The ladder is: `Off` captures nothing; `Trace`
/// captures *everything* (all crates, the full firehose including dependencies); and
/// every level in between captures Cathode's own crates at that level while capping
/// dependencies at WARN — so their genuine errors/warnings still surface, but their
/// debug/trace spam (hyper/reqwest/tao) does not.
pub fn targets(level: LogLevel) -> Targets {
    let lf = match level {
        LogLevel::Off => return Targets::new(),
        LogLevel::Trace => return Targets::new().with_default(LevelFilter::TRACE),
        LogLevel::Error => LevelFilter::ERROR,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Debug => LevelFilter::DEBUG,
    };
    Targets::new()
        .with_target("cathode", lf)
        .with_target("cathode_lib", lf)
        .with_target("cathode_core", lf)
        .with_default(LevelFilter::WARN)
}

/// A runtime switch for the capture filter. Wraps the boxed reload closure so the
/// (verbose) tracing handle type never leaks into the managed-state signature.
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
