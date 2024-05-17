#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![feature(trait_upcasting)]
#![feature(int_roundings)]
#![feature(option_get_or_insert_default)]
#![feature(let_chains)]

pub mod app;
mod screen;
pub use screen::Screen;
mod context;
pub use context::Context;
mod ephemeral_state;
pub mod tab;
mod ui_state;
pub mod util;
pub use ephemeral_state::EphemeralState;
pub use ui_state::UiState;
mod command;
mod node;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use app::CubedawApp;

    env_logger::init();

    eframe::run_native(
        "cubedaw",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(|cc| Box::new(CubedawApp::new(cc))),
    )
}
