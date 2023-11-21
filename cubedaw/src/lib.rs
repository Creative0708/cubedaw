mod app;
pub use app::TestApp;
pub mod compat;
mod screen;
pub mod widget;

pub struct Context<'a> {
    egui_frame: &'a eframe::Frame,

    paused: bool,
}
