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
pub use resourcekey::ResourceKey;
pub use track::{GroupTrack, SectionTrack, Track, TrackInner};
mod patch;
pub use patch::{Cable, CableTag, NodeData, NodeEntry, NodeInput, NodeOutput, NodeTag, Patch};
// mod node;
// pub use node::{
//     DataDrain, DataSource, DynNode, DynNodeState, Node, NodeCreationContext, NodeState,
//     NoteProperty,
// };
mod registry;
pub use registry::{DynNodeFactory, NodeRegistry, NodeRegistryEntry, NodeStateFactory};
mod buffer;
pub use buffer::{Buffer, BufferOwned, BufferType};
mod util;
// #[cfg(feature = "egui")]
// pub use node::{NodeContext, NodeInputUiOptions, NodeStateWrapper, NodeUiContext, ValueHandler};
pub use util::PreciseSongPos;

// TODO replace with more robust system (for scales other than 12TET, etc.)
pub fn pitch_to_hertz(pitch: f32) -> f32 {
    const MIDDLE_C_FREQUENCY: f32 = 261.62558f32; // 440 / 2**(9/12)
    const MULT_PER_PITCH: f32 = 1.0594631f32; // 2**(1/12)

    MIDDLE_C_FREQUENCY * MULT_PER_PITCH.powf(pitch)
}
