#![feature(trait_upcasting)]
#![feature(int_roundings)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
mod screen;
pub use screen::Screen;
mod context;
pub use context::Context;
mod state;
pub mod tab;
pub mod util;
pub use state::UiState;

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
