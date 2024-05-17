// Like cubedaw-command, but allowing

use cubedaw_command::StateCommand;

use crate::UiState;

pub mod misc;
pub mod note;
pub mod section;
pub mod track;

pub trait UiStateCommand: 'static + Send {
    // renamed functions to prevent name collisions when doing .execute/.rollback on a normal StateCommand
    // TODO should there be an immutable state parameter to these?
    fn ui_execute(&mut self, ui_state: &mut UiState);
    fn ui_rollback(&mut self, ui_state: &mut UiState);

    fn inner(&mut self) -> Option<&mut dyn StateCommand> {
        None
    }
}

impl<T: StateCommand> UiStateCommand for T {
    fn ui_execute(&mut self, _ui_state: &mut UiState) {}
    fn ui_rollback(&mut self, _ui_state: &mut UiState) {}

    fn inner(&mut self) -> Option<&mut dyn StateCommand> {
        Some(self)
    }
}
