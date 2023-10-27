use egui::CursorIcon;

use super::Screen;


pub struct TestScreen{
    counter: i32,
}

impl Screen for TestScreen{
    fn get_id(&self) -> egui::Id {
        egui::Id::new("loremipsum")
    }
    fn update(&mut self, ctx: &crate::Context, ui: &mut egui::Ui) {
        if ui.heading("Lorem Ipsum").hovered() {
            ctx.egui_ctx.set_cursor_icon(CursorIcon::AllScroll);
        }
        ui.label("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Phasellus a fermentum urna, sed tempor lorem. Mauris aliquet nisl a purus imperdiet hendrerit. Quisque vel magna orci. Phasellus fermentum consequat massa, et condimentum sem pellentesque ut. Suspendisse a velit erat. Nullam eget velit at eros porta luctus vel non nunc. Curabitur eget tempus metus.");

        if ui.button("Click me").clicked() {
            self.counter += 1;
        }

        ui.label(format!("You have clicked {} times", self.counter)).context_menu(|ui| { ui.label("context menu"); });
    }
}

impl Default for TestScreen{
    fn default() -> Self {
        Self {
            counter: 0,
        }
    }
}