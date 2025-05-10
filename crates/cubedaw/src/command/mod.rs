//! Like cubedaw-command, but with `UiState` support.

use std::any::Any;

use cubedaw_worker::command::{ActionType, StateCommand, StateCommandWrapper};

use crate::{EphemeralState, UiState};

pub mod clip;
pub mod misc;
pub mod node;
pub mod note;
pub mod patch;
pub mod track;

pub trait UiStateCommand: 'static + Send {
    fn run_ui(
        &mut self,
        ui_state: &mut UiState,
        ephemeral_state: &mut EphemeralState,
        action: ActionType,
    );

    fn try_merge(&mut self, _other: &Self) -> bool {
        false
    }

    // TODO should there be a default impl? kinda seems like a footgun if you forget to implement it
    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        None
    }
}

pub trait UiStateCommandWrapper: 'static + Send + Any {
    fn run_ui(
        &mut self,
        ui_state: &mut UiState,
        ephemeral_state: &mut EphemeralState,
        action: ActionType,
    );

    fn try_merge(&mut self, other: &dyn UiStateCommandWrapper) -> bool;

    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper>;
}
impl<T: UiStateCommand> UiStateCommandWrapper for T {
    fn run_ui(
        &mut self,
        ui_state: &mut UiState,
        ephemeral_state: &mut EphemeralState,
        action: ActionType,
    ) {
        UiStateCommand::run_ui(self, ui_state, ephemeral_state, action)
    }

    fn try_merge(&mut self, other: &dyn UiStateCommandWrapper) -> bool {
        if let Some(other) = (other as &dyn Any).downcast_ref() {
            UiStateCommand::try_merge(self, other)
        } else {
            false
        }
    }

    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        UiStateCommand::inner(self)
    }
}

impl dyn UiStateCommandWrapper {
    pub fn execute_ui(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState) {
        self.run_ui(ui_state, ephemeral_state, ActionType::Execute)
    }

    pub fn rollback_ui(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState) {
        self.run_ui(ui_state, ephemeral_state, ActionType::Rollback)
    }
}

pub struct UiStateCommandNoop<T: StateCommand>(pub T);

impl<T: StateCommand> UiStateCommand for UiStateCommandNoop<T> {
    fn run_ui(&mut self, _: &mut UiState, _: &mut EphemeralState, _: ActionType) {}

    fn try_merge(&mut self, other: &Self) -> bool {
        StateCommand::try_merge(&mut self.0, &other.0)
    }

    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        Some(&mut self.0)
    }
}

pub trait IntoUiStateCommand<T: UiStateCommand> {
    fn into_ui_state_command(self) -> T;
}

impl<T: StateCommand> IntoUiStateCommand<UiStateCommandNoop<T>> for T {
    fn into_ui_state_command(self) -> UiStateCommandNoop<T> {
        UiStateCommandNoop(self)
    }
}

impl<T: UiStateCommand> IntoUiStateCommand<T> for T {
    fn into_ui_state_command(self) -> Self {
        self
    }
}

pub struct FunctionUiStateCommand<
    F: FnMut(&mut UiState, &mut EphemeralState, ActionType) + Send + 'static,
>(F);

impl<F: FnMut(&mut UiState, &mut EphemeralState, ActionType) + Send + 'static> UiStateCommand
    for FunctionUiStateCommand<F>
{
    fn run_ui(
        &mut self,
        ui_state: &mut UiState,
        ephemeral_state: &mut EphemeralState,
        action: ActionType,
    ) {
        (self.0)(ui_state, ephemeral_state, action);
    }

    fn try_merge(&mut self, _other: &Self) -> bool {
        false
    }
    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        None
    }
}

// convenience impl for "inline" state commands
impl<F: FnMut(&mut UiState, &mut EphemeralState, ActionType) + Send + 'static>
    IntoUiStateCommand<FunctionUiStateCommand<F>> for F
{
    fn into_ui_state_command(self) -> FunctionUiStateCommand<F> {
        FunctionUiStateCommand(self)
    }
}

pub struct Noop;

impl UiStateCommand for Noop {
    fn run_ui(&mut self, _: &mut UiState, _: &mut EphemeralState, _: ActionType) {}
}
