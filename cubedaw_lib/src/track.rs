use egui::Id;

use crate::synth::Synthesizer;

pub struct Track {
    pub id: Id,
    pub volume: f32,

    pub track_data: TrackData,
}

pub enum TrackData {
    ParentTrack(ParentTrack),
    SynthesizerTrack(SynthesizerTrack),
}

pub struct ParentTrack {
    pub child_tracks: Vec<Id>,
}

pub struct SynthesizerTrack {
    pub synth: Synthesizer,
}
