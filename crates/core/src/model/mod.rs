//! The normalized domain model.
//!
//! One model is the contract shared by both ends of the app. Both the Xtream
//! client and the M3U parser emit these exact types, and the Dioxus UI consumes
//! them directly. Nothing downstream branches on which source produced a record.

pub mod category;
pub mod id;
pub mod log_level;
pub mod log_line;
pub mod programme;
pub mod settings;
pub mod stream;

pub use category::{Category, CategoryId};
pub use id::{derive_stream_id, StreamId};
pub use log_level::LogLevel;
pub use log_line::LogLine;
pub use programme::{NowNext, Programme};
pub use settings::{ChannelView, Settings};
pub use stream::{Stream, StreamKind};
