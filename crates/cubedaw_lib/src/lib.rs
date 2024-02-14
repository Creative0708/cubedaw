#![feature(entry_insert)]

pub mod buffer;
pub mod math;
pub mod misc;
pub mod synth;
pub mod track;

mod range;
pub use range::Range;

mod state;
pub use state::State;

mod id;
pub use id::{Id, IdCorrespondenceMap, IdHasher, IdMap, IdSet};
