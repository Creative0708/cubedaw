use std::any::Any;

use cubedaw_lib::Id;
use egui::WidgetText;

pub mod viewer;

mod track;
pub use track::TrackScreen;
mod pianoroll;
pub use pianoroll::PianoRollScreen;

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum ScreenType {
//     Track,
//     PianoRoll,
// }

pub trait Screen: Any {
    fn id(&self) -> Id<()>;

    fn title(&self) -> WidgetText;
    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui);

    fn closeable(&self) -> bool {
        true
    }

    fn create<'a>(ctx: &'a mut crate::Context) -> Self
    where
        Self: Sized;
}

impl PartialEq for dyn Screen {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for dyn Screen {}
