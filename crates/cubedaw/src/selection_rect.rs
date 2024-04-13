use cubedaw_lib::Id;
use egui::Pos2;

use crate::app::Tab;

pub struct SelectionRect {
    drag_start_pos: Option<Pos2>,
    tab_id: Id<Tab>,
    rect: Option<egui::Rect>,
    released: bool,
    // process_interaction is usually called at the very end, so if we reset at the end of every
    // frame the screen can't access the selection rect. This field adds a 1-frame delay.
    // TODO is there a better way to do this?
    should_reset: bool,
}

impl SelectionRect {
    pub fn new() -> Self {
        Self {
            drag_start_pos: None,
            tab_id: Id::invalid(),
            rect: None,
            released: false,
            should_reset: false,
        }
    }

    pub fn process_interaction(&mut self, interaction: egui::Response, tab_id: Id<Tab>) {
        if interaction.drag_started() {
            self.drag_start_pos = interaction.ctx.input(|i| i.pointer.interact_pos());
            self.tab_id = tab_id;
        }
        if interaction.drag_released() {
            self.released = true;
        }
    }

    pub fn released_rect(&mut self, tab_id: Id<Tab>) -> Option<egui::Rect> {
        if self.released && self.tab_id == tab_id {
            self.rect
        } else {
            None
        }
    }

    pub fn rect(&self) -> egui::Rect {
        self.rect.unwrap_or(egui::Rect::NOTHING)
    }

    pub fn draw(&mut self, ui: &mut egui::Ui, tab_id: Id<Tab>) {
        if self.tab_id != tab_id {
            return;
        }
        if let Some(drag_start_pos) = self.drag_start_pos {
            if let Some(pointer_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                const SELECTION_COLOR: egui::Color32 = egui::Color32::from_rgb(0, 32, 200);
                let rect = egui::Rect::from_two_pos(drag_start_pos, pointer_pos);
                ui.painter().rect(
                    rect,
                    egui::Rounding::ZERO,
                    SELECTION_COLOR.gamma_multiply(0.3),
                    (2.0, SELECTION_COLOR.gamma_multiply(0.7)),
                );
                self.rect = Some(rect);
            }
        }
    }

    pub fn finish(&mut self) {
        if self.should_reset {
            self.should_reset = false;
            self.released = false;
            self.rect = None;
            self.drag_start_pos = None;
        }
        if self.released {
            self.should_reset = true;
        }
    }
}
