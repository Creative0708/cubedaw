use std::{
    fmt::{self, Debug},
    mem, ops,
};

use ahash::{HashMap, HashMapExt};
use egui::{Pos2, Vec2};

use super::Select;

// code reuse through generics!!1!!
pub trait SelectablePath: Sized + std::hash::Hash + Eq + PartialEq + 'static
where
    Diff<Self>: Clone + Copy + Debug + Default,
{
    type Id: Debug + Clone + Copy;
    type Pos: ops::Sub<Self::Pos>;

    type Extra: Default = ();
}
// what is this, typescript?
type Diff<T> = <<T as SelectablePath>::Pos as ops::Sub<<T as SelectablePath>::Pos>>::Output;

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

/// Drag handler. Unified selection/deselection/drag logic for all of cubedaw's stuff. Hooray!
///
/// TODO: due to tabs being rendered sequentially, worst-case scenario everything except the last selection on the last tab is delayed by one frame.
/// To solve this (and various other issues), this delays _everything_ by one frame, only applying movement at the end of each frame.
pub struct DragHandler<T: SelectablePath> {
    dragged_data: Option<DraggedData<T>>,

    // when the drag finishes, it's stored in here.
    result: DragHandlerResult<T>,

    // various other variables that are checked at the end of the frame
    marked_reset: bool,

    /// Extra data.
    pub extra: T::Extra,
}

impl<T: SelectablePath> fmt::Debug for DragHandler<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DragHandler { .. }")
    }
}

#[derive(Debug)]
pub struct DraggedData<T: SelectablePath> {
    thing: T::Id,

    raw_start_pos: Pos2,
    raw_current_pos: Pos2,

    snapped_movement: Diff<T>,
}
impl<T: SelectablePath> DraggedData<T> {
    fn raw_movement(&self) -> Vec2 {
        self.raw_current_pos - self.raw_start_pos
    }
    fn snapped_movement_with(&self, snap_fn: impl Fn(Pos2) -> T::Pos) -> Diff<T> {
        snap_fn(self.raw_current_pos) - snap_fn(self.raw_start_pos)
    }
    fn snapped_movement(&self) -> Diff<T> {
        self.snapped_movement
    }
}

impl<T: SelectablePath> DragHandler<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn dragged_id(&self) -> Option<T::Id> {
        self.dragged_data.as_ref().map(|data| data.thing)
    }
    pub fn is_being_dragged(&self) -> bool {
        self.dragged_data.is_some()
    }
    pub fn would_be_dragged(&self, select: Select) -> bool {
        self.is_being_dragged() && select.is()
    }

    pub fn raw_movement(&self) -> Option<Vec2> {
        self.dragged_data.as_ref().map(|data| data.raw_movement())
    }
    pub fn snapped_movement(&self) -> Option<Diff<T>> {
        self.dragged_data
            .as_ref()
            .map(|data| data.snapped_movement())
    }

    pub fn handle<F: Fn(Pos2) -> T::Pos, R>(
        &mut self,
        snap_fn: F,
        f: impl FnOnce(&mut Prepared<T, F>) -> R,
    ) -> R {
        let mut prepared = Prepared {
            handler: self,

            finished_movement: None,
            new_drag_movement: None,
            snap_fn,
        };

        let r = f(&mut prepared);

        prepared.end();

        r
    }

    pub fn on_frame_end(&mut self) -> DragHandlerResult<T> {
        let result = mem::take(&mut self.result);

        if self.marked_reset {
            self.marked_reset = false;
            self.dragged_data = None;
        }

        result
    }
}

impl<T: SelectablePath> Default for DragHandler<T> {
    fn default() -> Self {
        Self {
            dragged_data: None,

            extra: Default::default(),

            result: Default::default(),
            marked_reset: false,
        }
    }
}

pub struct Prepared<'a, T: SelectablePath, F: Fn(Pos2) -> T::Pos> {
    handler: &'a mut DragHandler<T>,

    // store the movement in here (don't directly update self.handler.dragged_data.raw_current_pos) as that would result in some `T`s observing the new movement while some don't. instead, delay all movement to the end of the frame. we prefer synchronized movement over pure latency
    new_drag_movement: Option<Vec2>,

    finished_movement: Option<Diff<T>>,

    snap_fn: F,
}

impl<T: SelectablePath, F: Fn(Pos2) -> T::Pos> Prepared<'_, T, F> {
    pub fn dragged_thing(&self) -> Option<T::Id> {
        self.handler.dragged_id()
    }
    pub fn is_being_dragged(&self) -> bool {
        self.handler.is_being_dragged()
    }
    pub fn would_be_dragged(&self, select: Select) -> bool {
        self.is_being_dragged() && select.is()
    }
    pub fn raw_movement(&self) -> Option<Vec2> {
        self.handler.raw_movement()
    }
    pub fn movement(&self) -> Option<Diff<T>> {
        self.handler.snapped_movement()
    }

    /// If, by the end of this frame, there is a currently ongoing drag, cancel it.
    pub fn mark_reset(&mut self) {
        self.handler.marked_reset = true;
    }

    pub fn deselect_all(&mut self) {
        self.handler.result.global_selection_action = Some(Select::Deselect);
    }

    pub fn process_interaction(
        &mut self,
        id: T::Id,
        resp: &egui::Response,
        path: T,
        select: Select,
    ) {
        if resp.drag_started()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            assert!(
                !self.handler.is_being_dragged(),
                "two drags started at the same time, TODO"
            );

            self.handler.dragged_data = Some(DraggedData {
                thing: id,
                raw_start_pos: pos,
                raw_current_pos: pos,

                snapped_movement: Default::default(),
            });
            self.new_drag_movement = Some(Default::default());
        }
        if resp.clicked() || (resp.drag_started() && !select.is()) {
            if resp.ctx.input(|i| i.modifiers.shift) {
                // if user shift-clicks, toggle the selectedness without affecting anything else
                self.handler.result.selection_changes.insert(path, !select);
            } else {
                self.handler
                    .result
                    .selection_changes
                    .insert(path, Select::Select);
                // if user clicks without pressing shift, deselect everything else
                self.deselect_all();
            }
        }
        if resp.secondary_clicked() {
            self.mark_reset();
        }
        if resp.dragged() {
            self.new_drag_movement = Some(resp.drag_delta());
        }
        if resp.drag_stopped()
            && let Some(ref data) = self.handler.dragged_data
        {
            self.finished_movement = Some(data.snapped_movement_with(&self.snap_fn));
        }
    }

    // this probably should be a Drop impl
    pub fn end(self) {
        if let (Some(new_drag_movement), Some(data)) =
            (self.new_drag_movement, &mut self.handler.dragged_data)
        {
            data.raw_current_pos += new_drag_movement;
            data.snapped_movement = data.snapped_movement_with(&self.snap_fn);
        }

        if let Some(finished_movement) = self.finished_movement {
            self.handler.result.movement = Some(finished_movement);
            self.handler.marked_reset = true;
        }
    }
}

#[must_use = "You should handle this"]
pub struct DragHandlerResult<T: SelectablePath> {
    pub movement: Option<Diff<T>>,

    pub global_selection_action: Option<Select>,

    pub selection_changes: HashMap<T, Select>,
}

impl<T: SelectablePath> Default for DragHandlerResult<T> {
    fn default() -> Self {
        Self {
            movement: None,
            global_selection_action: None,
            selection_changes: HashMap::new(),
        }
    }
}

impl<T: SelectablePath> DragHandlerResult<T> {
    pub fn merge(&mut self, other: Self) {
        assert!(
            !(self.movement.is_some() && other.movement.is_some()),
            "multiple drags ended on the same frame, TODO"
        );
        if let Some(movement) = other.movement {
            self.movement = Some(movement);
        }
        self.global_selection_action = self
            .global_selection_action
            .or(other.global_selection_action);
        self.selection_changes.extend(other.selection_changes);
    }
}
