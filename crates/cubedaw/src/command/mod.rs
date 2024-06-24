// Like cubedaw-command, but allowing

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
impl<T: StateCommand> UiStateCommand for T {
    fn ui_execute(&mut self, _ui_state: &mut UiState, _ephemeral_state: &mut EphemeralState) {}
    fn ui_rollback(&mut self, _ui_state: &mut UiState, _ephemeral_state: &mut EphemeralState) {}

    fn try_merge(&mut self, other: &Self) -> bool {
        StateCommand::try_merge(self, other)
    }

    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        Some(self)
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
