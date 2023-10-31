use egui::{CursorIcon, Id};

use super::Screen;


pub struct TestScreen{
    id: Id,
    counter: i32,

    checkbox_values: [bool; 32],
}

impl TestScreen{
    pub fn new(id: Id) -> Self{
        Self {
            id,
            ..Default::default()
        }
    }
}

impl Screen for TestScreen{
    fn get_id(&self) -> egui::Id {
        self.id
    }
    fn update(&mut self, ctx: &crate::Context, ui: &mut egui::Ui) {
        if ui.heading("Lorem Ipsum").hovered() {
            ctx.egui_ctx.set_cursor_icon(CursorIcon::AllScroll);
        }

        let lorem_ipsum = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Phasellus a fermentum urna, sed tempor lorem. Mauris aliquet nisl a purus imperdiet hendrerit. Quisque vel magna orci. Phasellus fermentum consequat massa, et condimentum sem pellentesque ut. Suspendisse a velit erat. Nullam eget velit at eros porta luctus vel non nunc. Curabitur eget tempus metus.";

        ui.label(lorem_ipsum);

        if ui.button("Click me").clicked() {
            self.counter += 1;
        }

        ui.label(format!("You have clicked {} times", self.counter));

        ui.label("Right click on me!").context_menu(|ui| { ui.label("You right clicked on me!"); });
        ui.add_space(12.0);
        ui.heading("Here are some checkboxes to make this screen taller");

        for (text, checked) in std::iter::zip(lorem_ipsum.split(' '), self.checkbox_values.iter_mut()) {
            ui.checkbox(checked, text);
        }
    }
}

impl Default for TestScreen{
    fn default() -> Self {
        Self {
            id: Id::from("test1"),
            counter: 0,

            checkbox_values: [false; 32],
        }
    }
}