mod app;
pub use app::TestApp;
mod screen;
mod compat;

use compat::Compat;

pub struct Context<'a>{
    compat: &'a dyn Compat,
    egui_ctx: &'a egui::Context,

}