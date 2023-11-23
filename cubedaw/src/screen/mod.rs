use egui::{Id, WidgetText};

pub mod viewer;

pub mod test;
pub mod test2;

mod track;
pub use track::TrackScreen;

pub trait Screen {
    fn id(&self) -> Id;
    fn title(&self) -> WidgetText;
    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui);

    fn closeable(&self) -> bool {
        true
    }
}
