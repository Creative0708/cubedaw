use std::cell::LazyCell;

use cubedaw_lib::{Id, IdMap, Node, Note, Section, State, Track};
use egui::Vec2;

use crate::{
    UiState,
    command::node::{UiNodeMove, UiNodeSelect},
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

        // track list for calculating y movement
        // this duplicates code with the track tab but the behavior is so different idt it's worth it
        let track_list = LazyCell::new(|| {
            let mut track_list = vec![];
            let mut track_stack = vec![state.root_track];

            while let Some(track_id) = track_stack.pop() {
                if !(track_id == state.root_track && !ui_state.show_root_track) {
                    track_list.push(track_id);
                }
                let track_ui = ui_state.tracks.force_get(track_id);
                if !track_ui.closed {
                    for &track_id in &track_ui.track_list {
                        track_stack.push(track_id);
                    }
                }
            }

            track_list
        });

        // roughly the same template for everything:
        // - if there's a global selection action:
        //   - iterate through every existing "thing" and set it to whether it needs to be selected
        // - otherwise:
        //   - iterate through every thing in the selection changes and set the selection state
        // - finally, handle movement

        {
            let result = self.track_drag.on_frame_end();

            if let Some(target_select) = result.global_selection_action {
                for (track_id, track_ui) in &ui_state.tracks {
                    let target_select_for_this = result
                        .selection_changes
                        .get(&track_id)
                        .copied()
                        .unwrap_or(target_select);

                    if track_ui.select != target_select_for_this {
                        tracker.add(UiTrackSelect::new(track_id, target_select_for_this));
                    }
                }
            } else {
                for (&track_id, &selected) in &result.selection_changes {
                    tracker.add(UiTrackSelect::new(track_id, selected));
                }
            }
            if let Some(finished_drag_offset) = result.movement {
                // for (&node_id, node_ui) in &track_ui.patch.nodes {
                //     if node_ui.selected {
                //         ctx.tracker.add(UiNodeMove::new(
                //             node_id,
                //             track_id,
                //             finished_drag_offset,
                //         ));
                //     }
                // }

                // TODO
                let _ = finished_drag_offset;
            }
        }

        {
            let result = self.section_drag.on_frame_end();

            if let Some(target_select) = result.global_selection_action {
                for (track_id, track_ui) in &ui_state.tracks {
                    for (section_id2, section_ui) in &track_ui.sections {
                        let target_select_for_this = result
                            .selection_changes
                            .get(&(track_id, section_id2))
                            .copied()
                            .unwrap_or(target_select);

                        if section_ui.select != target_select_for_this {
                            tracker.add(UiSectionSelect::new(
                                track_id,
                                section_id2,
                                target_select_for_this,
                            ));
                        }
                    }
                }
            } else {
                for (&(track_id, section_id), &selected) in &result.selection_changes {
                    tracker.add(UiSectionSelect::new(track_id, section_id, selected));
                }
            }
            if let Some(finished_drag_offset) = result.movement {
                for (track_id, track) in &state.tracks {
                    let new_track_id = if finished_drag_offset.idx == 0 {
                        track_id
                    } else {
                        // optimize with a hashmap if necessary, currently O(num tracks^2)
                        let Some(curr_idx) = track_list.iter().position(|&i| i == track_id) else {
                            continue;
                        };
                        dbg!(finished_drag_offset);
                        let new_idx = curr_idx
                            .saturating_add_signed(finished_drag_offset.idx as isize)
                            .min(track_list.len() - 1);

                        track_list[new_idx]
                    };

                    let track_ui = ui_state.tracks.force_get(track_id);
                    for (section_range, section_id, _section) in track.sections() {
                        let section_ui = track_ui.sections.force_get(section_id);
                        if section_ui.select.is() {
                            tracker.add(SectionMove::new(
                                track_id,
                                new_track_id,
                                section_range,
                                section_range.start + finished_drag_offset.time,
                            ));
                        }
                    }
                }
            }
        }

        {
            let result = self.note_drag.on_frame_end();

            if let Some(target_select) = result.global_selection_action {
                for (track_id, track_ui) in &ui_state.tracks {
                    for (section_id, section_ui) in &track_ui.sections {
                        for (note_id, note_ui) in &section_ui.notes {
                            let target_select_for_this = result
                                .selection_changes
                                .get(&(track_id, section_id, note_id))
                                .copied()
                                .unwrap_or(target_select);
                            if note_ui.select != target_select_for_this {
                                tracker.add(UiNoteSelect::new(
                                    track_id,
                                    section_id,
                                    note_id,
                                    target_select_for_this,
                                ));
                            }
                        }
                    }
                }
            } else {
                for (&(track_id, section_id, note_id), &selected) in &result.selection_changes {
                    tracker.add(UiNoteSelect::new(track_id, section_id, note_id, selected));
                }
            }
            if let Some(offset) = result.movement {
                for (track_id, track_ui) in &ui_state.tracks {
                    for (section_id, section_ui) in &track_ui.sections {
                        for (note_id, note_ui) in &section_ui.notes {
                            if note_ui.select.is() {
                                tracker.add(NoteMove::new(
                                    track_id,
                                    section_id,
                                    note_id,
                                    offset.time,
                                    offset.pitch,
                                ));
                            }
                        }
                    }
                }
            }
        }

        for (track_id, track_ephem) in &mut self.tracks {
            let patch_ui = &ui_state.tracks.force_get(track_id).patch;
            let result = track_ephem.patch.node_drag.on_frame_end();

            if let Some(target_select) = result.global_selection_action {
                for (node_id, node_ui) in &patch_ui.nodes {
                    let target_select_for_this = result
                        .selection_changes
                        .get(&node_id)
                        .copied()
                        .unwrap_or(target_select);

                    if node_ui.select != target_select_for_this {
                        tracker.add(UiNodeSelect::new(track_id, node_id, target_select_for_this));
                    }
                }
            } else {
                for (&node_id, &selected) in &result.selection_changes {
                    tracker.add(UiNodeSelect::new(track_id, node_id, selected));
                }
            }

            if let Some(finished_drag_offset) = result.movement {
                for (node_id, node_ui) in &patch_ui.nodes {
                    if node_ui.select.is() {
                        tracker.add(UiNodeMove::new(node_id, track_id, finished_drag_offset));
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
