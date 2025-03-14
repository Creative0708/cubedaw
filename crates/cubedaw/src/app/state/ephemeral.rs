use cubedaw_lib::{Id, IdMap, Node, Note, Section, State, Track};
use egui::Vec2;

use crate::{
    UiState,
    context::UiStateTracker,
    util::{DragHandler, NodeSearch, SelectionRect},
};

#[derive(Debug)]
pub struct EphemeralState {
    pub note_drag: DragHandler<(Id<Track>, Id<Section>, Id<Note>)>,
    pub section_drag: DragHandler<(Id<Track>, Id<Section>)>,
    pub track_drag: DragHandler<Id<Track>>,

    pub tracks: IdMap<Track, TrackEphemeralState>,

    pub selection_rect: SelectionRect,

    pub node_search: NodeSearch,

    _private: private::Private,
}
mod private {
    #[derive(Clone, Copy, Debug)]
    pub struct Private;
}

impl EphemeralState {
    pub(in crate::app) fn new() -> Self {
        Self {
            note_drag: Default::default(),
            section_drag: Default::default(),
            track_drag: Default::default(),
            tracks: Default::default(),
            selection_rect: Default::default(),
            node_search: Default::default(),

            _private: private::Private,
        }
    }

    pub fn on_frame_end(
        &mut self,
        state: &State,
        ui_state: &UiState,
        tracker: &mut UiStateTracker,
    ) {
        use crate::command::{note::UiNoteSelect, section::UiSectionSelect, track::UiTrackSelect};
        use cubedaw_command::{note::NoteMove, section::SectionMove};

        self.note_drag.on_frame_end();
        self.track_drag.on_frame_end();

        // roughly the same template for everything:
        // - if there's a global selection action:
        //   - iterate through every existing "thing" and set it to whether it needs to be selected
        // - otherwise:
        //   - iterate through every thing in the selection changes
        {
            let result = self.section_drag.on_frame_end();

            let selection_changes = result.selection_changes;
            if let Some(target_state) = result.global_selection_action {
                for (track_id, track) in &ui_state.tracks {
                    for (section_id2, section_ui) in &track.sections {
                        let target_state_for_this = selection_changes
                            .get(&(track_id, section_id2))
                            .copied()
                            .unwrap_or(target_state);

                        if section_ui.selected != target_state_for_this {
                            tracker.add(UiSectionSelect::new(
                                track_id,
                                section_id2,
                                target_state_for_this,
                            ));
                        }
                    }
                }
            } else {
                for (&(track_id, section_id), &selected) in &selection_changes {
                    tracker.add(UiSectionSelect::new(track_id, section_id, selected));
                }
            }
            if let Some(finished_drag_offset) = result.movement {
                for (track_id, track) in &state.tracks {
                    if let Some(track) = track.inner.section() {
                        let track_ui = ui_state.tracks.force_get(track_id);
                        for (section_range, section_id, _section) in track.sections() {
                            let section_ui = track_ui.sections.force_get(section_id);
                            if section_ui.selected.is() {
                                // tracker.add(SectionMove::new(
                                //     track_id,
                                //     ,
                                //     section_range,
                                //     section_range.start + finished_drag_offset.time,
                                // ));
                                todo!();
                            }
                        }
                    }
                }
            }
        }

        self.selection_rect.on_frame_end();
    }
}

#[derive(Debug, Default)]
pub struct TrackEphemeralState {
    pub patch: PatchEphemeralState,
}

#[derive(Debug, Default)]
pub struct PatchEphemeralState {
    pub node_drag: DragHandler<Id<Node>>,
    pub nodes: IdMap<Node, NodeEphemeralState>,
}

#[derive(Debug, Default)]
pub struct NodeEphemeralState {
    pub size: Vec2,
    pub input_state: Vec<InputEphemeralState>,
}
#[derive(Debug)]
pub struct InputEphemeralState {
    pub num_connected: u32,
}
