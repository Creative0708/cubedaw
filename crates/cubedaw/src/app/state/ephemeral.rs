use std::cell::LazyCell;

use ahash::HashSetExt;
use cubedaw_lib::{Clip, Id, IdMap, IdSet, Node, Note, State, Track};
use egui::Vec2;

use crate::{
    UiState,
    command::{
        clip::{ClipAddOrRemove, ClipMove},
        node::{NodeAddOrRemove, NodeSelect, UiNodeMove},
        note::{NoteAddOrRemove, NoteMove},
        patch::CableAddOrRemove,
        track::TrackAddOrRemove,
    },
    context::UiStateTracker,
    util::{DragHandler, NodeSearch, SelectionRect},
};

#[derive(Debug)]
pub struct EphemeralState {
    pub note_drag: DragHandler<(Id<Track>, Id<Clip>, Id<Note>)>,
    pub clip_drag: DragHandler<(Id<Track>, Id<Clip>)>,
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
            clip_drag: Default::default(),
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
        use crate::command::{clip::UiClipSelect, note::NoteSelect, track::TrackSelect};
        // use cubedaw_command::{clip::ClipMove, note::NoteMove};

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
        //   - iterate through every existing "thing":
        //     - set it to whether it needs to be selected
        // - otherwise:
        //   - iterate through every thing in the selection changes and set the selection state
        // - handle movement
        // - handle deletions

        {
            let result = self.track_drag.on_frame_end();

            if let Some(global_select) = result.global_selection_action {
                for (track_id, track_ui) in &ui_state.tracks {
                    let new_select = result
                        .selection_changes
                        .get(&track_id)
                        .copied()
                        .unwrap_or(global_select);

                    if track_ui.select != new_select {
                        tracker.add(TrackSelect::new(track_id, new_select));
                    }
                }
            } else {
                for (&track_id, &selected) in &result.selection_changes {
                    tracker.add(TrackSelect::new(track_id, selected));
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
            if result.delete_selected {
                // usually you'd have a link from a child back to the parent but doubly linked structures would be awful to debug
                // and this is gonna be O(n) anyways so this doesn't really matter
                let parent_map = {
                    let mut parent_map = IdMap::new();
                    for (track_id, track_state) in &state.tracks {
                        for &child_id in &track_state.children {
                            parent_map.insert(child_id, track_id);
                        }
                    }
                    parent_map
                };
                for (track_id, track_ui) in &ui_state.tracks {
                    if track_ui.select.is() {
                        // parent_map.get(_) can return None if track_id is the root track, in which case don't remove it lol
                        if let Some(&parent_track) = parent_map.get(track_id) {
                            tracker.add(TrackAddOrRemove::removal(track_id, Some(parent_track)));
                        }
                    }
                }
            }
        }

        {
            let result = self.clip_drag.on_frame_end();

            if let Some(target_select) = result.global_selection_action {
                for (track_id, track_ui) in &ui_state.tracks {
                    for (clip_id2, clip_ui) in &track_ui.clips {
                        let target_select_for_this = result
                            .selection_changes
                            .get(&(track_id, clip_id2))
                            .copied()
                            .unwrap_or(target_select);

                        if clip_ui.select != target_select_for_this {
                            tracker.add(UiClipSelect::new(
                                track_id,
                                clip_id2,
                                target_select_for_this,
                            ));
                        }
                    }
                }
            } else {
                for (&(track_id, clip_id), &selected) in &result.selection_changes {
                    tracker.add(UiClipSelect::new(track_id, clip_id, selected));
                }
            }
            if let Some(finished_drag_offset) = result.movement
                && (finished_drag_offset.idx != 0 || finished_drag_offset.time != 0)
            {
                for (track_id, track) in &state.tracks {
                    let new_track_id = if finished_drag_offset.idx == 0 {
                        track_id
                    } else {
                        // optimize with a hashmap if necessary, currently O(num tracks^2)
                        let Some(curr_idx) = track_list.iter().position(|&i| i == track_id) else {
                            continue;
                        };
                        let new_idx = curr_idx
                            .saturating_add_signed(finished_drag_offset.idx as isize)
                            .min(track_list.len() - 1);

                        track_list[new_idx]
                    };

                    let track_ui = ui_state.tracks.force_get(track_id);
                    for (clip_range, clip_id, _clip) in track.clips() {
                        let clip_ui = track_ui.clips.force_get(clip_id);
                        if clip_ui.select.is() {
                            tracker.add(ClipMove::new(
                                clip_id,
                                track_id,
                                new_track_id,
                                clip_range,
                                clip_range.start + finished_drag_offset.time,
                            ));
                        }
                    }
                }
            }

            if result.delete_selected {
                for (track_id, track) in &state.tracks {
                    let track_ui = ui_state.tracks.force_get(track_id);
                    for (clip_range, clip_id, _clip) in track.clips() {
                        let clip_ui = track_ui.clips.force_get(clip_id);
                        if clip_ui.select.is() {
                            tracker.add(ClipAddOrRemove::removal(
                                clip_id,
                                clip_range.start,
                                track_id,
                            ));
                        }
                    }
                }
            }
        }

        // notes
        {
            let result = self.note_drag.on_frame_end();

            if let Some(target_select) = result.global_selection_action {
                for (track_id, track_ui) in &ui_state.tracks {
                    for (clip_id, clip_ui) in &track_ui.clips {
                        for (note_id, note_ui) in &clip_ui.notes {
                            let target_select_for_this = result
                                .selection_changes
                                .get(&(track_id, clip_id, note_id))
                                .copied()
                                .unwrap_or(target_select);
                            if note_ui.select != target_select_for_this {
                                tracker.add(NoteSelect::new(
                                    track_id,
                                    clip_id,
                                    note_id,
                                    target_select_for_this,
                                ));
                            }
                        }
                    }
                }
            } else {
                for (&(track_id, clip_id, note_id), &selected) in &result.selection_changes {
                    tracker.add(NoteSelect::new(track_id, clip_id, note_id, selected));
                }
            }
            if let Some(offset) = result.movement {
                for (track_id, track_ui) in &ui_state.tracks {
                    for (clip_id, clip_ui) in &track_ui.clips {
                        for (note_id, note_ui) in &clip_ui.notes {
                            if note_ui.select.is() {
                                tracker.add(NoteMove::new(
                                    track_id,
                                    clip_id,
                                    note_id,
                                    offset.time,
                                    offset.pitch,
                                ));
                            }
                        }
                    }
                }
            }
            if result.delete_selected {
                for (track_id, track_ui) in &ui_state.tracks {
                    for (clip_id, clip_ui) in &track_ui.clips {
                        for (note_id, note_ui) in &clip_ui.notes {
                            if note_ui.select.is() {
                                tracker.add(NoteAddOrRemove::removal(track_id, clip_id, note_id));
                            }
                        }
                    }
                }
            }
        }

        {
            // nodes
            for (track_id, track_ephem) in &mut self.tracks {
                let patch = &state.tracks.force_get(track_id).patch;
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
                            tracker.add(NodeSelect::new(track_id, node_id, target_select_for_this));
                        }
                    }
                } else {
                    for (&node_id, &selected) in &result.selection_changes {
                        tracker.add(NodeSelect::new(track_id, node_id, selected));
                    }
                }

                if let Some(finished_drag_offset) = result.movement {
                    for (node_id, node_ui) in &patch_ui.nodes {
                        if node_ui.select.is() {
                            tracker.add(UiNodeMove::new(node_id, track_id, finished_drag_offset));
                        }
                    }
                }
                if result.delete_selected {
                    let mut deleted_cables = IdSet::new();
                    for (node_id, node_ui) in &patch_ui.nodes {
                        if node_ui.select.is() {
                            let node = patch.node_entry(node_id).expect("state/ui state desync");

                            for cable_id in node.connected_cables() {
                                if deleted_cables.insert(cable_id) {
                                    tracker.add(CableAddOrRemove::removal(cable_id, track_id));
                                }
                            }
                            tracker.add(NodeAddOrRemove::removal(node_id, track_id));
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
