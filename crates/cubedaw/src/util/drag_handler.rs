use std::{fmt::Debug, ops};

use ahash::{HashMap, HashMapExt};
use egui::{Pos2, Vec2};

pub trait SelectablePath: Sized + std::hash::Hash + Eq + PartialEq + 'static {
    type Id: Debug + Clone + Copy;
}
mod impls {
    use super::SelectablePath;
    use cubedaw_lib::{Id, Node, Note, Section, Track};

    impl SelectablePath for (Id<Track>, Id<Section>, Id<Note>) {
        type Id = Id<Note>;
    }
    impl SelectablePath for (Id<Track>, Id<Section>) {
        type Id = Id<Section>;
    }
    impl SelectablePath for Id<Track> {
        type Id = Id<Track>;
    }
    /// DragHandler<Id<Node>>s are per-track and thus don't need the track id
    impl SelectablePath for Id<Node> {
        type Id = Id<Track>;
    }
}

#[derive(Debug)]
pub struct DragHandler<T: SelectablePath> {
    dragged_data: Option<DraggedData<T>>,
}

#[derive(Debug)]
pub struct DraggedData<T: SelectablePath> {
    id: T::Id,

    raw_start_pos: Pos2,
    raw_current_pos: Pos2,

    state: DraggedDataState,
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DraggedDataState {
    Dragging,
    /// The `DraggedData` finished the frame. If we immediately reset the `DragHandler` after we finish, the tabs that render after this tab will observe the unmodified state (because the commands haven't been processed yet) for a single frame, which looks, in scientific terms, "stupid as hell". So hold the `dragged_data` as `Some(_)` until the frame ends.
    Finished,
}
impl<T: SelectablePath> DraggedData<T> {
    fn raw_movement(&self) -> Vec2 {
        self.raw_current_pos - self.raw_start_pos
    }
    fn snapped_movement<M: ops::Sub<M>>(&self, snap_fn: impl Fn(Pos2) -> M) -> M::Output {
        let start_pos = snap_fn(self.raw_start_pos);
        let current_pos = snap_fn(self.raw_current_pos);
        current_pos - start_pos
    }
}

impl<T: SelectablePath> DragHandler<T> {
    pub fn new() -> Self {
        Self { dragged_data: None }
    }
    fn reset(&mut self) {
        if let Some(ref mut data) = self.dragged_data {
            data.state = DraggedDataState::Finished;
        }
    }

    // pub fn set_scale(&mut self, scale: impl Into<Vec2>) {
    //     self.scale = scale.into();
    // }

    pub fn dragged_id(&self) -> Option<T::Id> {
        self.dragged_data.as_ref().map(|data| data.id)
    }
    pub fn is_being_dragged(&self) -> bool {
        self.dragged_data.is_some()
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

    pub fn raw_movement(&self) -> Option<Vec2> {
        self.dragged_data.as_ref().map(|data| data.raw_movement())
    }
    pub fn snapped_movement<M: ops::Sub<M>>(
        &self,
        snap_fn: impl Fn(Pos2) -> M,
    ) -> Option<M::Output> {
        self.dragged_data
            .as_ref()
            .map(|data| data.snapped_movement(snap_fn))
    }
    // pub fn raw_movement_x_for(&self, id: Id) -> Option<f32> {
    //     self.is_being_dragged(id).then_some(self.raw_movement.x)
    // }
    // pub fn raw_movement_y_for(&self, id: Id) -> Option<f32> {
    //     self.is_being_dragged(id).then_some(self.raw_movement.y)
    // }

    pub fn handle<R>(
        &mut self,
        f: impl FnOnce(&mut Prepared<T, Pos2, fn(Pos2) -> Pos2>) -> R,
    ) -> DragHandlerResult<T, Vec2, R> {
        self.handle_snapped::<R, Pos2, fn(Pos2) -> Pos2>(|x| x, f)
    }
    pub fn handle_snapped<R, P: ops::Sub<P>, F: Fn(Pos2) -> P>(
        &mut self,
        snap_fn: F,
        f: impl FnOnce(&mut Prepared<T, P, F>) -> R,
    ) -> DragHandlerResult<T, P::Output, R> {
        let mut prepared = Prepared {
            handler: self,
            selection_changes: HashMap::new(),
            should_deselect_everything: false,
            finished_movement: None,
            new_drag_movement: None,
            canceled: false,
            snap_fn,
        };

        let result = f(&mut prepared);

        prepared.end().with_inner(result).1
    }

    pub fn end_of_frame(&mut self) {
        self.dragged_data
            .take_if(|data| data.state == DraggedDataState::Finished);
    }
}

impl<T: SelectablePath> Default for DragHandler<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Prepared<
    'drag,
    T: SelectablePath,
    M: ops::Sub<M> = Pos2,
    F: Fn(Pos2) -> M = fn(Pos2) -> M,
> {
    handler: &'drag mut DragHandler<T>,
    // HashMap<changed path, whether it is selected>
    selection_changes: HashMap<T, bool>,
    should_deselect_everything: bool,
    finished_movement: Option<M::Output>,
    new_drag_movement: Option<Vec2>,
    canceled: bool,
    snap_fn: F,
}

impl<T: SelectablePath, P: ops::Sub<P>, F: Fn(Pos2) -> P> Prepared<'_, T, P, F> {
    pub fn dragged_thing(&self) -> Option<T::Id> {
        self.handler.dragged_id()
    }
    pub fn is_being_dragged(&self) -> bool {
        self.handler.is_being_dragged()
    }
    pub fn raw_movement(&self) -> Option<Vec2> {
        self.handler.raw_movement()
    }
    pub fn movement(&self) -> Option<P::Output> {
        self.handler.snapped_movement(&self.snap_fn)
    }

    pub fn process_interaction(
        &mut self,
        id: T::Id,
        resp: &egui::Response,
        path: T,
        is_currently_selected: bool,
    ) {
        if resp.drag_started()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            assert!(!self.handler.is_being_dragged());
            self.handler.dragged_data = Some(DraggedData {
                id,
                raw_start_pos: pos,
                raw_current_pos: pos,

                state: DraggedDataState::Dragging,
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
            && let Some(data) = self.handler.dragged_data.take()
        {
            // no need to reset now; the DragHandler will be reset in end()
            self.finished_movement = Some(data.snapped_movement(&self.snap_fn));
        }
    }

    fn end(self) -> DragHandlerResult<T, P::Output, ()> {
        if let (Some(new_drag_movement), Some(data)) =
            (self.new_drag_movement, &mut self.handler.dragged_data)
        {
            data.raw_current_pos += new_drag_movement;
        }

        let movement = if let Some(finished_movement) = self.finished_movement {
            self.handler.reset();
            Some(finished_movement)
        } else {
            if self.canceled {
                self.handler.reset();
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
    fn with_inner<S>(self, new_inner: S) -> (R, DragHandlerResult<T, M, S>) {
        let Self {
            movement,
            should_deselect_everything,
            selection_changes,
            inner,
        } = self;

        (inner, DragHandlerResult {
            movement,
            should_deselect_everything,
            selection_changes,
            inner: new_inner,
        })
    }
}
