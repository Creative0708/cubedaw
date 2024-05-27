//! Command system for cubedaw. Separate from `cubedaw-lib` because it's not strictly needed to store state

use cubedaw_lib::State;

pub mod misc;
pub mod node;
pub mod note;
pub mod section;
pub mod track;

pub trait StateCommand: 'static + Send {
    fn execute(&mut self, state: &mut State);
    fn rollback(&mut self, state: &mut State);
}

pub trait Invertible: StateCommand {
    type Inverted: StateCommand;

    fn invert(&self) -> Self::Inverted;
}
