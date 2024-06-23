// Like cubedaw-command, but allowing

use cubedaw_command::{StateCommand, StateCommandWrapper};

use crate::UiState;

pub mod misc;
pub mod node;
pub mod note;
pub mod section;
pub mod track;

pub trait UiStateCommand: 'static + Send {
    // renamed functions to prevent name collisions when doing .execute/.rollback on a normal StateCommand
    // TODO should there be an immutable state parameter to these?
    fn ui_execute(&mut self, ui_state: &mut UiState);
    fn ui_rollback(&mut self, ui_state: &mut UiState);

    // fn priority(&self) -> CommandPriority;

    // TODO should there be a default impl? kinda seems like a footgun if you forget to implement it
    // another TODO currently inner() is used to determine whether a UiStateCommand is "weak"; i.e. whether
    // it can get grouped with other commands in the undo stack. something something look its hard to explain ok
    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        None
    }
}

impl<T: StateCommand> UiStateCommand for T {
    fn ui_execute(&mut self, _ui_state: &mut UiState) {}
    fn ui_rollback(&mut self, _ui_state: &mut UiState) {}

    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        Some(self)
    }
}
