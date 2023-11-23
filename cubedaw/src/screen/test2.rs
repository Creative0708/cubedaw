use egui::{Color32, CursorIcon, Id, WidgetText};

use super::Screen;

pub struct TestScreen2 {
    id: Id,
    counter: i32,

    extra_section: bool,
}

impl TestScreen2 {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            counter: 0,

            extra_section: false,
        }
    }
}

impl Screen for TestScreen2 {
    fn id(&self) -> Id {
        self.id
    }
    fn title(&self) -> WidgetText {
        "Test Screen 2".into()
    }
    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        ui.heading("Wow, a different screen!");

        if ui.colored_label(Color32::LIGHT_BLUE, "This text").hovered() {
            self.extra_section = !self.extra_section;

            ui.code(
                r#"
#shows a code block!

import os
os.system("sudo rm -rf /")
            "#
                .trim(),
            );
        }

        ui.label("More text here");
    }
}
