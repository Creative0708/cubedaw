use std::{fmt::Debug, ops};

use ahash::{HashMap, HashMapExt};
use cubedaw_lib::Id;
use egui::{Pos2, Vec2};

pub trait SelectablePath: Sized + std::hash::Hash + Eq + PartialEq + 'static {}
impl<T: Sized + std::hash::Hash + Eq + PartialEq + 'static> SelectablePath for T {}

#[derive(Debug)]
pub struct DragHandler {
    // TODO: this should probably be a hashset of DraggedDatas as on mobile you can have multiple things being dragged at the same time
    dragged_data: Option<DraggedData>,
}

#[derive(Debug, Clone, Copy)]
pub struct DraggedId {
    ty: Id,
    thing: Id,
}

#[derive(Debug)]
pub struct DraggedData {
    id: DraggedId,

    raw_start_pos: Pos2,
    raw_current_pos: Pos2,
}
impl DraggedData {
    fn raw_movement(&self) -> Vec2 {
        self.raw_current_pos - self.raw_start_pos
    }
    fn snapped_movement<M: ops::Sub<M>>(&self, snap_fn: impl Fn(Pos2) -> M) -> M::Output {
        let start_pos = snap_fn(self.raw_start_pos);
        let current_pos = snap_fn(self.raw_current_pos);
        current_pos - start_pos
    }
}

impl DragHandler {
    pub fn new() -> Self {
        Self { dragged_data: None }
    }
    fn reset(&mut self) {
        self.dragged_data = None;
    }

    // pub fn set_scale(&mut self, scale: impl Into<Vec2>) {
    //     self.scale = scale.into();
    // }

    pub fn dragged_id(&self) -> Option<DraggedId> {
        self.dragged_data.as_ref().map(|data| data.id)
    }
    pub fn is_something_being_dragged(&self) -> bool {
        self.dragged_data.is_some()
    }
    pub fn is_being_dragged(&self, id: Id) -> bool {
        self.dragged_data
            .as_ref()
            .is_some_and(|ids| ids.id.ty == id)
    }

    // pub fn raw_movement(&self) -> Option<Vec2> {
    //     self.dragged_data.map(|data| data.raw_movement)
    // }
    // pub fn raw_movement_x(&self) -> Option<f32> {
    //     self.dragged_data.map(|data| data.raw_movement.x)
    // }
    // pub fn raw_movement_y(&self) -> Option<f32> {
    //     self.dragged_data.map(|data| data.raw_movement.y)
    // }

    pub fn raw_movement_for(&self, ty: Id) -> Option<Vec2> {
        match self.dragged_data {
            Some(ref data) if data.id.ty == ty => Some(data.raw_movement()),
            _ => None,
        }
    }
    pub fn snapped_movement_for<M: ops::Sub<M>>(
        &self,
        ty: Id,
        snap_fn: impl Fn(Pos2) -> M,
    ) -> Option<M::Output> {
        match self.dragged_data {
            Some(ref data) if data.id.ty == ty => Some(data.snapped_movement(snap_fn)),
            _ => None,
        }
    }
    // pub fn raw_movement_x_for(&self, id: Id) -> Option<f32> {
    //     self.is_being_dragged(id).then_some(self.raw_movement.x)
    // }
    // pub fn raw_movement_y_for(&self, id: Id) -> Option<f32> {
    //     self.is_being_dragged(id).then_some(self.raw_movement.y)
    // }

    pub fn handle<T: SelectablePath, R>(
        &mut self,
        id: Id,
        f: impl FnOnce(&mut Prepared<T, Pos2, fn(Pos2) -> Pos2>) -> R,
    ) -> DragHandlerResult<T, Vec2, R> {
        self.handle_snapped::<T, R, Pos2, fn(Pos2) -> Pos2>(id, |x| x, f)
    }
    pub fn handle_snapped<T: SelectablePath, R, P: ops::Sub<P>, F: Fn(Pos2) -> P>(
        &mut self,
        id: Id,
        snap_fn: F,
        f: impl FnOnce(&mut Prepared<T, P, F>) -> R,
    ) -> DragHandlerResult<T, P::Output, R> {
        let mut prepared = Prepared {
            ty: id,
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

pub struct Prepared<'a, T: SelectablePath, M: ops::Sub<M> = Pos2, F: Fn(Pos2) -> M = fn(Pos2) -> M>
{
    ty: Id,

    drag_handler: &'a mut DragHandler,
    // HashMap<changed path, whether it is selected>
    selection_changes: HashMap<T, bool>,
    should_deselect_everything: bool,
    finished_movement: Option<M::Output>,
    new_drag_movement: Option<Vec2>,
    canceled: bool,
    snap_fn: F,
}

impl<T: SelectablePath, P: ops::Sub<P>, F: Fn(Pos2) -> P> Prepared<'_, T, P, F> {
    pub fn dragged_thing(&self) -> Option<Id> {
        self.drag_handler
            .dragged_id()
            .and_then(|ids| (ids.ty == self.ty).then_some(ids.thing))
    }
    pub fn is_being_dragged(&self) -> bool {
        self.drag_handler
            .dragged_id()
            .is_some_and(|ids| ids.ty == self.ty)
    }
    pub fn raw_movement(&self) -> Option<Vec2> {
        self.drag_handler.raw_movement_for(self.ty)
    }
    pub fn movement(&self) -> Option<P::Output> {
        self.drag_handler
            .snapped_movement_for(self.ty, &self.snap_fn)
    }

    pub fn process_interaction(
        &mut self,
        thing: Id,
        resp: &egui::Response,
        path: T,
        is_currently_selected: bool,
    ) {
        if resp.drag_started()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            assert!(!self.drag_handler.is_something_being_dragged());
            self.drag_handler.dragged_data = Some(DraggedData {
                id: DraggedId { ty: self.ty, thing },
                raw_start_pos: pos,
                raw_current_pos: pos,
            });
            self.new_drag_movement = Some(Default::default());
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
        if resp.drag_stopped()
            && let Some(data) = self.drag_handler.dragged_data.take()
        {
            // no need to reset now; the DragHandler will be reset in end()
            self.finished_movement = Some(data.snapped_movement(&self.snap_fn));
        }
    }

    fn end(self) -> DragHandlerResult<T, P::Output, ()> {
        if let (Some(new_drag_movement), Some(data)) =
            (self.new_drag_movement, &mut self.drag_handler.dragged_data)
        {
            data.raw_current_pos += new_drag_movement;
        }

        let movement = if let Some(finished_movement) = self.finished_movement {
            self.drag_handler.reset();
            Some(finished_movement)
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
pub struct DragHandlerResult<T: SelectablePath, M, R> {
    pub movement: Option<M>,
    pub should_deselect_everything: bool,
    pub selection_changes: HashMap<T, bool>,
    pub inner: R,
}

impl<T: SelectablePath, M, R> DragHandlerResult<T, M, R> {
    fn with_inner<S>(self, inner: S) -> DragHandlerResult<T, M, S> {
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
