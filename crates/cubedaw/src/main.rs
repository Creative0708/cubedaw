#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![feature(trait_upcasting)]
#![feature(int_roundings)]
#![feature(let_chains)]
#![feature(portable_simd)]
#![feature(float_next_up_down)]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(clippy::new_without_default)] // useless, cubedaw isn't a library so default impls aren't necessary

pub mod app;
mod screen;
pub use screen::Screen;
mod context;
pub use context::Context;
mod state;
pub mod tab;
pub mod util;
pub use state::{ephemeral::EphemeralState, ui::UiState};
mod command;
mod node;
mod widget;
mod workerhost;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use app::CubedawApp;

    env_logger::init();

    eframe::run_native(
        "cubedaw",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(CubedawApp::new(cc)))),
    )
}
