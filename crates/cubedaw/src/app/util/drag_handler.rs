use std::{fmt::Debug, ops};

use ahash::{HashMap, HashMapExt};
use egui::{Pos2, Vec2};

// the associated types are kinda messy but no way in hell am i adding _more_ generics to DragHandler
pub trait SelectablePath: Sized + std::hash::Hash + Eq + PartialEq + 'static {
    type Id: Debug + Clone + Copy;
    type Pos: ops::Sub<Self::Pos>;
}
// what is this, typescript?
type Diff<T: SelectablePath> = <T::Pos as ops::Sub<T::Pos>>::Output;

mod impls {
    use super::SelectablePath;
    use cubedaw_lib::{Id, Node, Note, Section, Track};
    use egui::Pos2;

    impl SelectablePath for (Id<Track>, Id<Section>, Id<Note>) {
        type Id = Id<Note>;
        type Pos = crate::tab::pianoroll::Note2DPos;
    }
    impl SelectablePath for (Id<Track>, Id<Section>) {
        type Id = Id<Section>;
        type Pos = crate::tab::track::Track2DPos;
    }
    impl SelectablePath for Id<Track> {
        type Id = Id<Track>;
        type Pos = Pos2;
    }
    // DragHandler<Id<Node>>s are per-track and thus don't need the track id
    impl SelectablePath for Id<Node> {
        type Id = Id<Track>;
        type Pos = Pos2;
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
}
impl<T: SelectablePath> DraggedData<T> {
    fn raw_movement(&self) -> Vec2 {
        self.raw_current_pos - self.raw_start_pos
    }
    fn snapped_movement(&self, snap_fn: impl Fn(Pos2) -> T::Pos) -> Diff<T> {
        snap_fn(self.raw_current_pos) - snap_fn(self.raw_start_pos)
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

    pub fn raw_movement(&self) -> Option<Vec2> {
        self.dragged_data.as_ref().map(|data| data.raw_movement())
    }
    pub fn snapped_movement(&self, snap_fn: impl Fn(Pos2) -> T::Pos) -> Option<Diff<T>> {
        self.dragged_data
            .as_ref()
            .map(|data| data.snapped_movement(snap_fn))
    }

    pub fn handle<F: Fn(Pos2) -> T::Pos, R>(
        &mut self,
        snap_fn: F,
        f: impl FnOnce(&mut Prepared<T, F>) -> R,
    ) -> R {
        let mut prepared = Prepared {
            handler: self,
            selection_changes: HashMap::new(),
            should_deselect_everything: false,
            movement_when_drag_stopped: None,
            new_drag_movement: None,
            canceled: false,
            snap_fn,
        };

        let result = f(&mut prepared);

        prepared.end().with_inner(result).1
    }

    pub fn on_frame_end(&mut self) -> DragHandlerResult<T> {}
}

impl<T: SelectablePath> Default for DragHandler<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Prepared<'drag, T: SelectablePath, F: Fn(Pos2) -> T::Pos> {
    handler: &'drag mut DragHandler<T>,
    // HashMap<changed path, whether it is selected>
    selection_changes: HashMap<T, bool>,
    should_deselect_everything: bool,
    movement_when_drag_stopped: Option<Diff<T>>,
    new_drag_movement: Option<Vec2>,
    canceled: bool,
    snap_fn: F,
}

impl<T: SelectablePath, F: Fn(Pos2) -> T::Pos> Prepared<'_, T, F> {
    pub fn dragged_thing(&self) -> Option<T::Id> {
        self.handler.dragged_id()
    }
    pub fn is_being_dragged(&self) -> bool {
        self.handler.is_being_dragged()
    }
    pub fn raw_movement(&self) -> Option<Vec2> {
        self.handler.raw_movement()
    }
    pub fn movement(&self) -> Option<Diff<T>> {
        self.handler.snapped_movement(&self.snap_fn)
    }

    pub fn deselect_all(&mut self) {
        self.should_deselect_everything = true;
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
            && let Some(ref data) = self.handler.dragged_data
        {
            self.movement_when_drag_stopped = Some(data.snapped_movement(&self.snap_fn));
        }
    }

    pub fn end(self) -> DragHandlerResult<T> {
        if let (Some(new_drag_movement), Some(data)) =
            (self.new_drag_movement, &mut self.handler.dragged_data)
        {
            data.raw_current_pos += new_drag_movement;
        }

        if self.movement_when_drag_stopped.is_some() || self.canceled {
            self.handler.reset();
        }

        DragHandlerResult {
            movement: self.movement_when_drag_stopped,
            should_deselect_everything: self.should_deselect_everything,
            selection_changes: self.selection_changes,
        }
    }
}

#[must_use = "You should handle this"]
pub struct DragHandlerResult<T: SelectablePath> {
    pub movement: Option<Diff<T>>,
    pub should_deselect_everything: bool,
    pub selection_changes: HashMap<T, bool>,
}
