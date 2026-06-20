//! Reusable UI building blocks. Components stay thin: they render shared
//! `cathode_core` types and raise events; state lives in the views.

pub mod category_list;
pub mod connect_form;
pub mod player_overlay;
pub mod stream_grid;

pub use category_list::CategoryList;
pub use connect_form::ConnectForm;
pub use player_overlay::PlayerOverlay;
pub use stream_grid::StreamGrid;
