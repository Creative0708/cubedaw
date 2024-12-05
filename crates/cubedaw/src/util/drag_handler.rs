use std::fmt::Debug;

use ahash::{HashMap, HashMapExt};
use cubedaw_lib::Id;
use egui::Vec2;

pub trait SelectablePath: Sized + std::hash::Hash + Eq + PartialEq + 'static {}
impl<T: Sized + std::hash::Hash + Eq + PartialEq + 'static> SelectablePath for T {}

#[derive(Debug)]
pub struct DragHandler {
    dragged_id: Option<DraggedId>,
    raw_movement: Vec2,
    scale: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub struct DraggedId {
    id: Id,
    thing: Id,
}

impl DragHandler {
    pub fn new() -> Self {
        Self {
            dragged_id: None,
            raw_movement: Vec2::ZERO,
            scale: Vec2::new(1.0, 1.0),
        }
    }
    fn reset(&mut self) {
        self.dragged_id = None;
        self.raw_movement = Vec2::ZERO;
    }

    pub fn set_scale(&mut self, scale: impl Into<Vec2>) {
        self.scale = scale.into();
    }

    pub fn dragged_id(&self) -> Option<DraggedId> {
        self.dragged_id
    }
    pub fn is_something_being_dragged(&self) -> bool {
        self.dragged_id.is_some()
    }
    pub fn is_being_dragged(&self, id: Id) -> bool {
        self.dragged_id.is_some_and(|ids| ids.id == id)
    }

    pub fn raw_movement(&self) -> Option<Vec2> {
        self.is_something_being_dragged()
            .then_some(self.raw_movement)
    }
    pub fn raw_movement_x(&self) -> Option<f32> {
        self.is_something_being_dragged()
            .then_some(self.raw_movement.x)
    }
    pub fn raw_movement_y(&self) -> Option<f32> {
        self.is_something_being_dragged()
            .then_some(self.raw_movement.y)
    }

    pub fn raw_movement_for(&self, id: Id) -> Option<Vec2> {
        self.is_being_dragged(id).then_some(self.raw_movement)
    }
    pub fn raw_movement_x_for(&self, id: Id) -> Option<f32> {
        self.is_being_dragged(id).then_some(self.raw_movement.x)
    }
    pub fn raw_movement_y_for(&self, id: Id) -> Option<f32> {
        self.is_being_dragged(id).then_some(self.raw_movement.y)
    }

    pub fn handle<T: SelectablePath, R>(
        &mut self,
        id: Id,
        f: impl FnOnce(&mut Prepared<T, fn(Vec2) -> Vec2>) -> R,
    ) -> DragHandlerResult<T, R> {
        self.handle_snapped(id, |x| x, f)
    }
    pub fn handle_snapped<T: SelectablePath, R, F: Fn(Vec2) -> Vec2>(
        &mut self,
        id: Id,
        snap_fn: F,
        f: impl FnOnce(&mut Prepared<T, F>) -> R,
    ) -> DragHandlerResult<T, R> {
        let mut prepared = Prepared {
            id,
            drag_handler: self,
            selection_changes: HashMap::new(),
            should_deselect_everything: false,
            finished_movement: None,
            new_drag_movement: None,
            canceled: false,
            snap_fn,
        };

        let result = f(&mut prepared);

        prepared.end().with_inner(result)
    }
}

impl Default for DragHandler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Prepared<'a, T: SelectablePath, F: Fn(Vec2) -> Vec2 = fn(Vec2) -> Vec2> {
    id: Id,

    drag_handler: &'a mut DragHandler,
    // HashMap<changed path, whether it is selected>
    selection_changes: HashMap<T, bool>,
    should_deselect_everything: bool,
    finished_movement: Option<Vec2>,
    new_drag_movement: Option<Vec2>,
    canceled: bool,
    snap_fn: F,
}

impl<T: SelectablePath, F: Fn(Vec2) -> Vec2> Prepared<'_, T, F> {
    pub fn set_scale(&mut self, scale: impl Into<Vec2>) {
        self.drag_handler.set_scale(scale)
    }
    pub fn dragged_thing(&self) -> Option<Id> {
        self.drag_handler
            .dragged_id
            .and_then(|ids| (ids.id == self.id).then_some(ids.thing))
    }
    pub fn is_being_dragged(&self) -> bool {
        self.drag_handler
            .dragged_id
            .is_some_and(|ids| ids.id == self.id)
    }
    pub fn raw_movement(&self) -> Option<Vec2> {
        self.is_being_dragged()
            .then_some(self.drag_handler.raw_movement)
    }
    pub fn raw_movement_x(&self) -> Option<f32> {
        self.is_being_dragged()
            .then_some(self.drag_handler.raw_movement.x)
    }
    pub fn raw_movement_y(&self) -> Option<f32> {
        self.is_being_dragged()
            .then_some(self.drag_handler.raw_movement.y)
    }
    pub fn movement(&self) -> Option<Vec2> {
        self.raw_movement().map(|m| (self.snap_fn)(m))
    }
    pub fn movement_x(&self) -> Option<f32> {
        self.raw_movement_x()
            .map(|x| (self.snap_fn)(Vec2::new(x, 0.0)).x)
    }
    pub fn movement_y(&self) -> Option<f32> {
        self.raw_movement_y()
            .map(|y| (self.snap_fn)(Vec2::new(0.0, y)).y)
    }

    pub fn process_interaction(
        &mut self,
        thing: Id,
        resp: &egui::Response,
        path: T,
        is_currently_selected: bool,
    ) {
        if resp.drag_started() {
            self.drag_handler.dragged_id = Some(DraggedId { id: self.id, thing });
            self.new_drag_movement = Some(Vec2::ZERO);
        }
        if resp.clicked() || (resp.drag_started() && !is_currently_selected) {
            if resp.ctx.input(|i| i.modifiers.shift) {
                // if user shift-clicks, toggle the selectedness without affecting anything else
                self.selection_changes.insert(path, !is_currently_selected);
            } else {
                // if user clicks without pressing shift, deselect everything else
                self.should_deselect_everything = true;
                self.selection_changes.insert(path, true);
            }
        }
        if resp.dragged() {
            self.new_drag_movement = Some(resp.drag_delta());
        }
        if resp.drag_stopped() {
            if self.drag_handler.is_something_being_dragged() {
                self.finished_movement = Some(self.drag_handler.raw_movement);
            }
        }
        // else if resp.ctx.input(|i| i.pointer.primary_released()) {
        //     self.canceled = true;
        // }
    }

    fn end(self) -> DragHandlerResult<T, ()> {
        if let Some(new_drag_movement) = self.new_drag_movement {
            self.drag_handler.raw_movement += new_drag_movement * self.drag_handler.scale;
        }

        let movement = if let Some(finished_movement) = self.finished_movement {
            self.drag_handler.reset();
            Some((self.snap_fn)(finished_movement))
        } else {
            if self.canceled {
                self.drag_handler.reset();
            }
            None
        };

        DragHandlerResult {
            movement,
            should_deselect_everything: self.should_deselect_everything,
            selection_changes: self.selection_changes,
            inner: (),
        }
    }
}

#[must_use = "You should handle this"]
pub struct DragHandlerResult<T: SelectablePath, R> {
    pub movement: Option<Vec2>,
    pub should_deselect_everything: bool,
    pub selection_changes: HashMap<T, bool>,
    pub inner: R,
}

impl<T: SelectablePath, R> DragHandlerResult<T, R> {
    fn with_inner<S>(self, inner: S) -> DragHandlerResult<T, S> {
        let Self {
            movement,
            should_deselect_everything,
            selection_changes,
            inner: _,
        } = self;

        DragHandlerResult {
            movement,
            should_deselect_everything,
            selection_changes,
            inner,
        }
    }
}
