use std::fmt::Debug;

use cubedaw_lib::{Id, IdMap};
use egui::Vec2;

pub trait SelectableUiData<T>: Debug {
    fn set_selected(&mut self, selected: bool);
    fn selected(&self) -> bool;
}

#[derive(Debug)]
pub struct DragHandler {
    is_dragging: bool,
    raw_movement: Vec2,
    scale: Vec2,
}

impl DragHandler {
    pub fn new() -> Self {
        Self {
            is_dragging: false,
            raw_movement: Vec2::ZERO,
            scale: Vec2::new(1.0, 1.0),
        }
    }
    fn reset(&mut self) {
        self.is_dragging = false;
        self.raw_movement = Vec2::ZERO;
    }

    pub fn set_scale(&mut self, scale: impl Into<Vec2>) {
        self.scale = scale.into();
    }

    pub fn raw_movement(&self) -> Option<Vec2> {
        self.is_dragging.then_some(self.raw_movement)
    }
    pub fn raw_movement_x(&self) -> Option<f32> {
        self.is_dragging.then_some(self.raw_movement.x)
    }
    pub fn raw_movement_y(&self) -> Option<f32> {
        self.is_dragging.then_some(self.raw_movement.y)
    }

    pub fn handle_snapped<T, U: SelectableUiData<T>, R, F: Fn(Vec2) -> Vec2>(
        &mut self,
        ui_data: &mut IdMap<T, U>,
        f: impl FnOnce(&mut Prepared<T, U, F>) -> R,
        snap_fn: F,
    ) -> (Option<Vec2>, R) {
        let mut prepared = Prepared {
            drag_handler: self,
            ui_data,
            single_thing_clicked: None,
            finished_movement: None,
            new_drag_movement: None,
            canceled: false,
            snap_fn,
        };

        let result = f(&mut prepared);

        (prepared.end(), result)
    }
}

pub struct Prepared<'a, 'b, T, U: SelectableUiData<T>, F: Fn(Vec2) -> Vec2> {
    drag_handler: &'a mut DragHandler,
    ui_data: &'b mut IdMap<T, U>,
    single_thing_clicked: Option<Id<T>>,
    finished_movement: Option<Vec2>,
    new_drag_movement: Option<Vec2>,
    canceled: bool,
    snap_fn: F,
}

impl<'a, 'b, T, U: SelectableUiData<T>, F: Fn(Vec2) -> Vec2> Prepared<'a, 'b, T, U, F> {
    pub fn data(&self) -> &'_ IdMap<T, U> {
        self.ui_data
    }
    pub fn data_mut(&mut self) -> &'_ mut IdMap<T, U> {
        self.ui_data
    }
    pub fn set_scale(&mut self, scale: impl Into<Vec2>) {
        self.drag_handler.set_scale(scale)
    }
    pub fn movement(&self) -> Option<Vec2> {
        self.drag_handler.raw_movement().map(|m| (self.snap_fn)(m))
    }
    pub fn movement_x(&self) -> Option<f32> {
        self.drag_handler
            .raw_movement_x()
            .map(|x| (self.snap_fn)(Vec2::new(x, 0.0)).x)
    }
    pub fn movement_y(&self) -> Option<f32> {
        self.drag_handler
            .raw_movement_y()
            .map(|y| (self.snap_fn)(Vec2::new(0.0, y)).y)
    }

    pub fn process_interaction(&mut self, resp: egui::Response, id: Id<T>) {
        let ui_data = self.ui_data.get_mut(id);
        if resp.drag_started() {
            self.new_drag_movement = Some(Vec2::ZERO);
        }
        if resp.clicked() || resp.drag_started() {
            if resp.clicked() || !ui_data.selected() {
                ui_data.set_selected(true);
                if !resp.ctx.input(|i| i.modifiers.shift) {
                    self.single_thing_clicked = Some(id);
                }
            }
        }
        if resp.dragged() {
            self.new_drag_movement = Some(resp.drag_delta());
        }
        if resp.drag_released() {
            if self.drag_handler.is_dragging {
                self.finished_movement = Some(self.drag_handler.raw_movement);
            } else {
                unreachable!();
            }
        } else if resp.ctx.input(|i| i.pointer.primary_released()) {
            self.canceled = true;
        }
    }

    fn end(self) -> Option<Vec2> {
        if let Some(new_drag_movement) = self.new_drag_movement {
            self.drag_handler.is_dragging = true;
            self.drag_handler.raw_movement += new_drag_movement * self.drag_handler.scale;
        }
        if let Some(single_thing_clicked) = self.single_thing_clicked {
            for (&id, ui_data) in self.ui_data.iter_mut() {
                if id != single_thing_clicked {
                    ui_data.set_selected(false);
                }
            }
        }
        if let Some(finished_movement) = self.finished_movement {
            self.drag_handler.reset();
            Some((self.snap_fn)(finished_movement))
        } else {
            if self.canceled {
                self.drag_handler.reset();
            }
            None
        }
    }
}
