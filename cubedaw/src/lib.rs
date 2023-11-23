mod app;

pub use app::TestApp;
pub mod compat;
mod screen;
pub mod widget;

pub mod resources;

pub struct Context<'a> {
    paused: bool,

    state: &'a cubedaw_lib::State,
}
