#![feature(int_roundings)]
#![feature(iter_map_windows)]
#![feature(impl_trait_in_assoc_type)]

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
pub use track::Track;
