//! Command system for cubedaw. Separate from `cubedaw-lib` because it's not strictly needed to store state

use std::any::TypeId;

use cubedaw_lib::State;
use cubedaw_workerlib::{NodeRegistry, WorkerState};

pub mod misc;
pub mod node;
pub mod note;
pub mod patch;
pub mod section;
pub mod track;
mod wrapper;
pub use wrapper::DontMerge;

mod tracker;
pub use tracker::StateTracker;

pub trait StateCommand: 'static + Send + Clone {
    fn execute(&mut self, state: &mut State);
    fn rollback(&mut self, state: &mut State);

    fn try_merge(&mut self, _other: &Self) -> bool {
        false
    }

    // TODO should these _really_ be in a shared cubedaw-command?
    fn worker_execute(&mut self, _worker_state: &mut WorkerState, _node_registry: &NodeRegistry) {}
    fn worker_rollback(&mut self, _worker_state: &mut WorkerState, _node_registry: &NodeRegistry) {}
}

pub trait StateCommandWrapper: 'static + Sealed + Send {
    fn execute(&mut self, state: &mut State);
    fn rollback(&mut self, state: &mut State);

    fn try_merge(&mut self, other: &dyn StateCommandWrapper) -> bool;

    fn worker_execute(&mut self, worker_state: &mut WorkerState, node_registry: &NodeRegistry);
    fn worker_rollback(&mut self, worker_state: &mut WorkerState, node_registry: &NodeRegistry);

    fn clone(&self) -> Box<dyn StateCommandWrapper>;

    fn type_id(&self) -> TypeId;
}

impl<T: StateCommand> Sealed for T {}
impl<T: StateCommand> StateCommandWrapper for T {
    fn execute(&mut self, state: &mut State) {
        StateCommand::execute(self, state)
    }
    fn rollback(&mut self, state: &mut State) {
        StateCommand::rollback(self, state)
    }

    fn try_merge(&mut self, other: &dyn StateCommandWrapper) -> bool {
        if let Some(other) = other.downcast_ref() {
            dbg!();
            StateCommand::try_merge(self, other)
        } else {
            false
        }
    }

    fn worker_execute(&mut self, worker_state: &mut WorkerState, node_registry: &NodeRegistry) {
        StateCommand::worker_execute(self, worker_state, node_registry)
    }
    fn worker_rollback(&mut self, worker_state: &mut WorkerState, node_registry: &NodeRegistry) {
        StateCommand::worker_rollback(self, worker_state, node_registry)
    }

    fn clone(&self) -> Box<dyn StateCommandWrapper> {
        Box::new(Clone::clone(self))
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

// Copy of `std::any::Any`. Replace this when trait upcasting is stabilized
// https://doc.rust-lang.org/beta/unstable-book/language-features/trait-upcasting.html
impl dyn StateCommandWrapper {
    pub fn downcast_ref<T: StateCommandWrapper + Sized>(&self) -> Option<&T> {
        if StateCommandWrapper::type_id(self) == TypeId::of::<T>() {
            Some(unsafe { &*(self as *const dyn StateCommandWrapper as *const T) })
        } else {
            None
        }
    }
    pub fn downcast_mut<T: StateCommandWrapper + Sized>(&mut self) -> Option<&T> {
        if StateCommandWrapper::type_id(self) == TypeId::of::<T>() {
            Some(unsafe { &*(self as *mut dyn StateCommandWrapper as *mut T) })
        } else {
            None
        }
    }
}

impl Clone for Box<dyn StateCommandWrapper> {
    fn clone(&self) -> Self {
        StateCommandWrapper::clone(self.as_ref())
    }
}

mod sealed {
    pub trait Sealed {}
}
pub(crate) use sealed::Sealed;
