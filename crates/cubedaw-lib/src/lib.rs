mod note;
pub use note::Note;
mod section;
pub use section::Section;
mod range;
pub use range::Range;
mod id;
pub use id::{Id, IdMap, IdSet};
mod state;
pub use state::State;
mod track;
pub use track::{GroupTrack, SectionTrack, Track, TrackInner};
mod resource_key;
pub use resource_key::ResourceKey;
mod patch;
pub use patch::{Cable, CableTag, NodeData, NodeInput, NodeOutput, NodeTag, Patch};
mod node;
pub use node::{DynNodeState, NodeState};
#[cfg(feature = "egui")]
pub use node::{NodeInputUiOptions, NodeStateWrapper, NodeUiContext, ValueHandler};

// TODO replace with more robust system (for scales other than 12TET, etc.)
pub fn pitch_to_hertz(pitch: f32) -> f32 {
    const MIDDLE_C_FREQUENCY: f32 = 261.62558f32; // 440 / 2**(9/12)
    const MULT_PER_PITCH: f32 = 1.0594631f32; // 2**(1/12)

    MIDDLE_C_FREQUENCY * MULT_PER_PITCH.powf(pitch)
}
