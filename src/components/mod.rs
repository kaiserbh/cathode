//! Reusable UI building blocks. Components stay thin: they render shared
//! `cathode_core` types and raise events; state lives in the views.

pub mod category_list;
pub mod channel_list;
pub mod channel_pane;
pub mod connect_form;
pub mod epg_guide;
pub mod icons;
pub mod logs_panel;
pub mod panel;
pub mod player_overlay;
pub mod search_results;
pub mod series_detail;
pub mod settings_panel;
pub mod sources_panel;
pub mod spinner;
pub mod stream_grid;
pub mod tab_bar;
pub mod title_bar;
pub mod toast;
pub mod toggle;

pub use category_list::CategoryList;
pub use channel_list::ChannelList;
pub use channel_pane::ChannelPane;
pub use connect_form::ConnectForm;
pub use epg_guide::EpgGuide;
pub use logs_panel::LogsPanel;
pub use panel::PanelDialog;
pub use player_overlay::PlayerOverlay;
pub use search_results::SearchResults;
pub use series_detail::SeriesDetail;
pub use settings_panel::SettingsPanel;
pub use sources_panel::SourcesPanel;
pub use spinner::Spinner;
pub use stream_grid::StreamGrid;
pub use tab_bar::{Tab, TabBar};
pub use title_bar::TitleBar;
pub use toast::Toast;
pub use toggle::Toggle;
