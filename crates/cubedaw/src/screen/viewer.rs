use crate::Context;

use super::Screen;
use cubedaw_lib::Id;
use egui::WidgetText;
use egui_dock::TabViewer;

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
    type Tab = Id<()>;

    fn title(&mut self, id: &mut Self::Tab) -> WidgetText {
        let tab = self.ctx.tabs.map.get_mut(id).unwrap();
        tab.title().into()
    }

    fn id(&mut self, id: &mut Self::Tab) -> egui::Id {
        let tab = self.ctx.tabs.map.get_mut(id).unwrap();
        tab.id().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, id: &mut Self::Tab) {
        let mut tab = self.ctx.tabs.map.remove(id).unwrap();
        tab.update(&mut self.ctx, ui);
        self.ctx.tabs.map.insert(tab.id().transmute(), tab);
    }
}
