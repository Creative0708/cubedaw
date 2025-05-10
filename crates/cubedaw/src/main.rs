#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![feature(int_roundings)]
#![feature(let_chains)]
#![feature(portable_simd)]
#![feature(if_let_guard)]
#![feature(gen_blocks)]
#![feature(coroutines)]
#![feature(associated_type_defaults)]
#![allow(clippy::new_without_default)] // useless, cubedaw isn't a library so default impls aren't necessary
#![forbid(unsafe_op_in_unsafe_fn)]

pub mod app;
mod screen;
pub use app::{
    context::{self, Context},
    state::{self, ephemeral::EphemeralState, ui::UiState},
    util,
};

pub use screen::Screen;
mod command;
pub mod dbg;
mod node;
pub mod tab;
pub use node::registry;
pub use registry::NodeRegistry;
mod widget;
mod workerhost;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use app::CubedawApp;

    tracing_subscriber::fmt::init();

    eframe::run_native(
        "cubedaw",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(CubedawApp::new(cc)))),
    )
}
