use cubedaw_lib::{Id, IdMap, Node, Note, Section, Track};
use egui::Vec2;

use crate::util::{DragHandler, NodeSearch, SelectionRect};

#[derive(Debug, Default)]
pub struct EphemeralState {
    pub note_drag: DragHandler<(Id<Track>, Id<Section>, Id<Note>)>,
    pub section_drag: DragHandler<(Id<Track>, Id<Section>)>,
    pub track_drag: DragHandler<Id<Track>>,

    pub tracks: IdMap<Track, TrackEphemeralState>,

    pub selection_rect: SelectionRect,

    pub node_search: NodeSearch,
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
