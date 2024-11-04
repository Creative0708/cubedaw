//! Like cubedaw-command, but with `UiState` support.

use cubedaw_command::{StateCommand, StateCommandWrapper};
use egui::util::id_type_map::TypeId;

use crate::{EphemeralState, UiState};

pub mod misc;
pub mod node;
pub mod note;
pub mod section;
pub mod track;

pub trait UiStateCommand: 'static + Send {
    // renamed functions to prevent name collisions when doing .execute/.rollback on a normal StateCommand
    // TODO should there be an immutable state parameter to these?
    fn ui_execute(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState);
    fn ui_rollback(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState);

    fn try_merge(&mut self, _other: &Self) -> bool {
        false
    }

    // TODO should there be a default impl? kinda seems like a footgun if you forget to implement it
    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        None
    }
}

pub trait UiStateCommandWrapper: 'static + Send {
    fn ui_execute(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState);
    fn ui_rollback(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState);

    fn try_merge(&mut self, other: &dyn UiStateCommandWrapper) -> bool;

    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper>;

    fn type_id(&self) -> TypeId;
}
impl<T: UiStateCommand> UiStateCommandWrapper for T {
    fn ui_execute(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState) {
        UiStateCommand::ui_execute(self, ui_state, ephemeral_state)
    }
    fn ui_rollback(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState) {
        UiStateCommand::ui_rollback(self, ui_state, ephemeral_state)
    }

    fn try_merge(&mut self, other: &dyn UiStateCommandWrapper) -> bool {
        if let Some(other) = other.downcast_ref() {
            UiStateCommand::try_merge(self, other)
        } else {
            false
        }
    }

    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        UiStateCommand::inner(self)
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl dyn UiStateCommandWrapper {
    pub fn downcast_ref<T: UiStateCommandWrapper + Sized>(&self) -> Option<&T> {
        if UiStateCommandWrapper::type_id(self) == TypeId::of::<T>() {
            Some(unsafe { &*(self as *const dyn UiStateCommandWrapper as *const T) })
        } else {
            None
        }
    }
    pub fn downcast_mut<T: UiStateCommandWrapper + Sized>(&mut self) -> Option<&T> {
        if UiStateCommandWrapper::type_id(self) == TypeId::of::<T>() {
            Some(unsafe { &*(self as *mut dyn UiStateCommandWrapper as *mut T) })
        } else {
            None
        }
    }
}

pub struct UiStateCommandNoop<T: StateCommand>(pub T);

impl<T: StateCommand> UiStateCommand for UiStateCommandNoop<T> {
    fn ui_execute(&mut self, _ui_state: &mut UiState, _ephemeral_state: &mut EphemeralState) {}
    fn ui_rollback(&mut self, _ui_state: &mut UiState, _ephemeral_state: &mut EphemeralState) {}

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
    F: FnMut(&mut UiState, &mut EphemeralState, UiActionType) + Send + 'static,
>(F);

impl<F: FnMut(&mut UiState, &mut EphemeralState, UiActionType) + Send + 'static> UiStateCommand
    for FunctionUiStateCommand<F>
{
    fn ui_execute(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState) {
        (self.0)(ui_state, ephemeral_state, UiActionType::Execute);
    }
    fn ui_rollback(&mut self, ui_state: &mut UiState, ephemeral_state: &mut EphemeralState) {
        (self.0)(ui_state, ephemeral_state, UiActionType::Execute);
    }

    fn try_merge(&mut self, _other: &Self) -> bool {
        false
    }
    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        None
    }
}

// convenience impl for "inline" state commands
impl<F: FnMut(&mut UiState, &mut EphemeralState, UiActionType) + Send + 'static>
    IntoUiStateCommand<FunctionUiStateCommand<F>> for F
{
    fn into_ui_state_command(self) -> FunctionUiStateCommand<F> {
        FunctionUiStateCommand(self)
    }
}

pub enum UiActionType {
    Execute,
    Rollback,
}

pub struct Noop;

impl UiStateCommand for Noop {
    fn ui_execute(&mut self, _ui_state: &mut UiState, _ephemeral_state: &mut EphemeralState) {}
    fn ui_rollback(&mut self, _ui_state: &mut UiState, _ephemeral_state: &mut EphemeralState) {}
}
