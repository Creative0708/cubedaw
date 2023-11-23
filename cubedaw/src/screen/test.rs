use egui::{CursorIcon, Id, Label, RichText, Sense, WidgetText};
use log::info;

use super::Screen;

pub struct TestScreen {
    id: Id,
    counter: i32,

    checkbox_values: [bool; 32],
}

impl TestScreen {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            counter: 0,

            checkbox_values: [false; 32],
        }
    }
}

impl Screen for TestScreen {
    fn id(&self) -> Id {
        self.id
    }
    fn title(&self) -> WidgetText {
        "Test Screen 1".into()
    }
    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        if ui.heading("Lorem Ipsum").hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::AllScroll);
        }

        let lorem_ipsum = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Phasellus a fermentum urna, sed tempor lorem. Mauris aliquet nisl a purus imperdiet hendrerit. Quisque vel magna orci. Phasellus fermentum consequat massa, et condimentum sem pellentesque ut. Suspendisse a velit erat. Nullam eget velit at eros porta luctus vel non nunc. Curabitur eget tempus metus.";

        ui.label(RichText::new(lorem_ipsum).weak());

        if ui.button("Click me").clicked() {
            self.counter += 1;
        }

        ui.label(format!("You have clicked {} times", self.counter));

        ui.add(Label::new("Right click on me!").sense(Sense::click()))
            .context_menu(|ui| {
                ui.label("You right clicked on me!");
            });
        ui.add_space(12.0);
        ui.heading("Here are some checkboxes to make this screen taller");

        for (text, checked) in
            std::iter::zip(lorem_ipsum.split(' '), self.checkbox_values.iter_mut())
        {
            ui.checkbox(checked, text);
        }
    }
}
