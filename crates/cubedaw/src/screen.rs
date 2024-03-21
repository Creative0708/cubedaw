use cubedaw_lib::Id;

use crate::{app::Tab, Context};

pub trait Screen: 'static {
    fn create(ctx: &mut Context) -> Self
    where
        Self: Sized;

    fn id(&self) -> Id<Tab>;

    fn title(&self) -> egui::WidgetText;

    fn update(&mut self, ctx: &mut Context, ui: &mut egui::Ui);
}
