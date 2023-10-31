use egui::{CursorIcon, Id, Color32};

use super::Screen;


pub struct TestScreen2{
    id: Id,
    counter: i32,

    extra_section: bool,
}

impl TestScreen2{
    pub fn new(id: Id) -> Self{
        Self {
            id,
            ..Default::default()
        }
    }
}

impl Screen for TestScreen2{
    fn get_id(&self) -> egui::Id {
        self.id
    }
    fn update(&mut self, ctx: &crate::Context, ui: &mut egui::Ui) {
        ui.heading("Wow, a different screen!");

        if ui.colored_label(Color32::LIGHT_BLUE, "This text").hovered() {
            self.extra_section = !self.extra_section;

            ui.code(r#"
#shows a code block!

import os
os.system("sudo rm -rf /")
            "#.trim());
        }

        ui.label("More text here");
    }
}

impl Default for TestScreen2{
    fn default() -> Self {
        Self {
            id: Id::from("test2"),
            counter: 0,

            extra_section: false,
        }
    }
}