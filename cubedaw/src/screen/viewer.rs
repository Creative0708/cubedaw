use crate::Context;

use super::Screen;
use egui::{Id, RichText, WidgetText};
use egui_dock::{DockState, TabViewer};

pub type Tab = Box<dyn Screen>;

pub struct CubedawTabViewer<'a> {
    ctx: Context<'a>,
}

impl<'a> CubedawTabViewer<'a> {
    pub fn new(ctx: Context<'a>) -> Self {
        Self { ctx }
    }
}

impl<'a> TabViewer for CubedawTabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        let str = tab.title();
        str.into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        tab.id()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.update(&mut self.ctx, ui);
    }
}
