use std::fmt::Debug;

use cubedaw_lib::{Id, IdMap};
use egui::Vec2;

use crate::context::StateTracker;

mod selection_rect;
pub use selection_rect::SelectionRect;

pub trait SelectableUiData<T> {
    type SelectEvent: SelectableUiEvent<T>;

    fn set_selected(&mut self, selected: bool);
    fn selected(&self) -> bool;
}
pub trait SelectableUiEvent<T>: crate::command::UiStateCommand {
    fn new(id: Id<T>, selected: bool) -> Self
    where
        Self: Sized;
}

mod impls {
    use cubedaw_lib::{Id, Note, Section, Track};

    use crate::{
        command::{note::UiNoteSelect, section::UiSectionSelect, track::UiTrackSelect},
        ui_state::{NoteUiState, SectionUiState, TrackUiState},
    };

    use super::{SelectableUiData, SelectableUiEvent};

    impl SelectableUiEvent<Section> for UiSectionSelect {
        fn new(id: Id<Section>, selected: bool) -> Self
        where
            Self: Sized,
        {
            UiSectionSelect::new(id, selected)
        }
    }
    impl SelectableUiData<Section> for SectionUiState {
        type SelectEvent = UiSectionSelect;
        fn selected(&self) -> bool {
            self.selected
        }
        fn set_selected(&mut self, selected: bool) {
            self.selected = selected;
        }
    }
    impl SelectableUiEvent<Track> for UiTrackSelect {
        fn new(id: Id<Track>, selected: bool) -> Self
        where
            Self: Sized,
        {
            UiTrackSelect::new(id, selected)
        }
    }
    impl SelectableUiData<Track> for TrackUiState {
        type SelectEvent = UiTrackSelect;
        fn selected(&self) -> bool {
            self.selected
        }
        fn set_selected(&mut self, selected: bool) {
            self.selected = selected;
        }
    }
    impl SelectableUiEvent<Note> for UiNoteSelect {
        fn new(id: Id<Note>, selected: bool) -> Self
        where
            Self: Sized,
        {
            UiNoteSelect::new(id, selected)
        }
    }
    impl SelectableUiData<Note> for NoteUiState {
        type SelectEvent = UiNoteSelect;
        fn selected(&self) -> bool {
            self.selected
        }
        fn set_selected(&mut self, selected: bool) {
            self.selected = selected;
        }
    }
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
        ui_data: &IdMap<T, U>,
        f: impl FnOnce(&mut Prepared<T, U, F>) -> R,
        snap_fn: F,
    ) -> DragHandlerResult<R> {
        let mut prepared = Prepared {
            drag_handler: self,
            ui_data,
            tracker: StateTracker::new(),
            single_thing_clicked: None,
            finished_movement: None,
            new_drag_movement: None,
            canceled: false,
            snap_fn,
        };

        let result = f(&mut prepared);

        prepared.end().with_inner(result)
    }
}

pub struct Prepared<'a, 'b, T, U: SelectableUiData<T>, F: Fn(Vec2) -> Vec2> {
    drag_handler: &'a mut DragHandler,
    ui_data: &'b IdMap<T, U>,
    tracker: StateTracker,
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
        let Some(ui_data) = self.ui_data.get(id) else {
            // TODO handle this better
            eprintln!("DragHandler::process_interaction called with nonexistent id: {id:?}");
            return;
        };
        if resp.drag_started() {
            self.new_drag_movement = Some(Vec2::ZERO);
        }
        if resp.clicked() || (resp.drag_started() && !ui_data.selected()) {
            self.tracker.add(U::SelectEvent::new(id, true));
            if !resp.ctx.input(|i| i.modifiers.shift) {
                self.single_thing_clicked = Some(id);
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

    fn end(mut self) -> DragHandlerResult<()> {
        if let Some(new_drag_movement) = self.new_drag_movement {
            self.drag_handler.is_dragging = true;
            self.drag_handler.raw_movement += new_drag_movement * self.drag_handler.scale;
        }

        if let Some(single_thing_clicked) = self.single_thing_clicked {
            for &id in self.ui_data.keys() {
                if id != single_thing_clicked {
                    self.tracker.add(U::SelectEvent::new(id, false));
                }
            }
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
            tracker: self.tracker,
            inner: (),
        }
    }
}

pub struct DragHandlerResult<R> {
    movement: Option<Vec2>,
    tracker: StateTracker,
    inner: R,
}

impl<R> DragHandlerResult<R> {
    pub fn apply(mut self, tracker: &mut StateTracker) -> Self {
        tracker.extend(self.tracker.take());
        self
    }
    pub fn movement(&self) -> Option<Vec2> {
        self.movement
    }
    pub fn inner(self) -> R {
        self.inner
    }

    fn with_inner<S>(self, inner: S) -> DragHandlerResult<S> {
        let Self {
            movement,
            tracker,
            inner: _,
        } = self;

        DragHandlerResult {
            movement,
            tracker,
            inner,
        }
    }
}
