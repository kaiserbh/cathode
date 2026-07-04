mod app;
mod bindings;
mod clipboard;
mod components;
mod epg;
mod filter;
mod format;
mod ui;
mod views;

use app::App;
use dioxus::prelude::*;
use dioxus_logger::tracing::Level;

fn main() {
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    launch(App);
}
