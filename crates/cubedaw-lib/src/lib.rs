#![feature(int_roundings)]
#![feature(iter_map_windows)]

mod note;
pub use note::Note;
mod section;
pub use section::Section;
mod range;
pub use range::Range;
mod id;
pub use id::{Id, IdMap};
mod state;
pub use state::State;
mod track;
pub use track::Track;
