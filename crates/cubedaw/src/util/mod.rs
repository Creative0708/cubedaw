use std::fmt::Debug;

use ahash::{HashMap, HashMapExt};
use egui::Vec2;

mod selection_rect;
pub use selection_rect::SelectionRect;
mod node_search;
pub use node_search::NodeSearch;

mod private {
    use cubedaw_lib::Id;

    // TODO why do these trait impls exist
    pub trait SelectablePath: Sized + std::hash::Hash + Eq + PartialEq + 'static {}
    impl SelectablePath for Id<cubedaw_lib::Track> {}
    impl SelectablePath for (Id<cubedaw_lib::Track>, Id<cubedaw_lib::Section>) {}
    impl SelectablePath
        for (
            Id<cubedaw_lib::Track>,
            Id<cubedaw_lib::Section>,
            Id<cubedaw_lib::Note>,
        )
    {
    }
    impl SelectablePath for (Id<cubedaw_lib::Track>, Id<cubedaw_lib::NodeData>) {}

    // pub trait SelectablePath: Sized + std::hash::Hash + Eq + PartialEq + 'static {
    //     type Ui: SelectableUi;
    //     type Command: Sized + 'static;
    //     fn create_command(&self, selected: bool) -> Self::Command;
    // }
    // pub trait SelectableUi: Sized + 'static {
    //     fn selected(&self) -> bool;
    // }

    // impl SelectablePath for Id<cubedaw_lib::Track> {
    //     type Ui = crate::state::ui::TrackUiState;
    //     type Command = crate::command::track::UiTrackSelect;

    //     fn create_command(&self, selected: bool) -> Self::Command {
    //         Self::Command::new(*self, selected)
    //     }
    // }
    // impl SelectableUi for crate::state::ui::TrackUiState {
    //     fn selected(&self) -> bool {
    //         self.selected
    //     }
    // }
    // impl SelectablePath for (Id<cubedaw_lib::Track>, Id<cubedaw_lib::Section>) {
    //     type Ui = crate::state::ui::SectionUiState;
    //     type Command = crate::command::section::UiSectionSelect;

    //     fn create_command(&self, selected: bool) -> Self::Command {
    //         Self::Command::new(self.0, self.1, selected)
    //     }
    // }
    // impl SelectableUi for crate::state::ui::SectionUiState {
    //     fn selected(&self) -> bool {
    //         self.selected
    //     }
    // }
    // impl SelectablePath
    //     for (
    //         Id<cubedaw_lib::Track>,
    //         Id<cubedaw_lib::Section>,
    //         Id<cubedaw_lib::Note>,
    //     )
    // {
    //     type Ui = crate::state::ui::NoteUiState;
    //     type Command = crate::command::note::UiNoteSelect;

    //     fn create_command(&self, selected: bool) -> Self::Command {
    //         Self::Command::new(self.0, self.1, self.2, selected)
    //     }
    // }
    // impl SelectableUi for crate::state::ui::NoteUiState {
    //     fn selected(&self) -> bool {
    //         self.selected
    //     }
    // }
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

    pub fn handle<T: private::SelectablePath, R>(
        &mut self,
        f: impl FnOnce(&mut Prepared<T, fn(Vec2) -> Vec2>) -> R,
    ) -> DragHandlerResult<T, R> {
        self.handle_snapped(|x| x, f)
    }
    pub fn handle_snapped<T: private::SelectablePath, R, F: Fn(Vec2) -> Vec2>(
        &mut self,
        snap_fn: F,
        f: impl FnOnce(&mut Prepared<T, F>) -> R,
    ) -> DragHandlerResult<T, R> {
        let mut prepared = Prepared {
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

pub struct Prepared<'a, T: private::SelectablePath, F: Fn(Vec2) -> Vec2> {
    drag_handler: &'a mut DragHandler,
    // Vec<(changed id, whether it is selected)>
    selection_changes: HashMap<T, bool>,
    should_deselect_everything: bool,
    finished_movement: Option<Vec2>,
    new_drag_movement: Option<Vec2>,
    canceled: bool,
    snap_fn: F,
}

impl<'a, T: private::SelectablePath, F: Fn(Vec2) -> Vec2> Prepared<'a, T, F> {
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

    pub fn process_interaction(
        &mut self,
        resp: egui::Response,
        path: T,
        is_currently_selected: bool,
    ) {
        if resp.drag_started() {
            self.new_drag_movement = Some(Vec2::ZERO);
        }
        if resp.clicked() || (resp.drag_started() && !is_currently_selected) {
            dbg!(resp.drag_started() && !is_currently_selected);
            if resp.ctx.input(|i| i.modifiers.shift) {
                // if user shift-clicks, toggle the selectedness without affecting anything else
                self.selection_changes.insert(path, !is_currently_selected);
            } else {
                // if user clicks without pressing shift,
                self.should_deselect_everything = true;
                self.selection_changes.insert(path, true);
            }
        }
        if resp.dragged() {
            self.new_drag_movement = Some(resp.drag_delta());
        }
        if resp.drag_stopped() {
            if self.drag_handler.is_dragging {
                self.finished_movement = Some(self.drag_handler.raw_movement);
            } else {
                unreachable!();
            }
        } else if resp.ctx.input(|i| i.pointer.primary_released()) {
            self.canceled = true;
        }
    }

    fn end(self) -> DragHandlerResult<T, ()> {
        if let Some(new_drag_movement) = self.new_drag_movement {
            self.drag_handler.is_dragging = true;
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

pub struct DragHandlerResult<T: private::SelectablePath, R> {
    movement: Option<Vec2>,
    should_deselect_everything: bool,
    selection_changes: HashMap<T, bool>,
    inner: R,
}

impl<T: private::SelectablePath, R> DragHandlerResult<T, R> {
    pub fn selection_changes(&self) -> &HashMap<T, bool> {
        &self.selection_changes
    }
    pub fn should_deselect_everything(&self) -> bool {
        self.should_deselect_everything
    }
    pub fn movement(&self) -> Option<Vec2> {
        self.movement
    }
    pub fn inner(self) -> R {
        self.inner
    }

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
