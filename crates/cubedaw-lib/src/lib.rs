mod note;
pub use note::Note;
mod section;
pub use section::Section;
mod range;
pub use range::Range;
pub mod id;
pub use id::{Id, IdMap, IdSet};
mod state;
pub use state::State;
mod track;
pub use resourcekey;
#[deprecated(note = "use resourcekey directly")]
pub use resourcekey::ResourceKey;
pub use track::{GroupTrack, SectionTrack, Track, TrackInner};
mod patch;
pub use patch::{
    Cable, CableConnection, CableTag, NodeData, Node, NodeInput, NodeOutput, NodeTag, Patch,
};
mod buffer;
pub use buffer::{Buffer, BufferType, InternalBufferType};
mod util;
// #[cfg(feature = "egui")]
// pub use node::{NodeContext, NodeInputUiOptions, NodeStateWrapper, NodeUiContext, ValueHandler};
pub use util::PreciseSongPos;

// TODO replace with more robust system (for scales other than 12TET, etc.)
pub fn pitch_to_hertz(pitch: f32) -> f32 {
    const MIDDLE_C_FREQUENCY: f32 = 261.62558f32; // 440 / 2**(9/12)
    const MULT_PER_PITCH_UNIT: f32 = 2.0; // cubedaw currently uses 1.0f32/octave

    MIDDLE_C_FREQUENCY * MULT_PER_PITCH_UNIT.powf(pitch)
}
