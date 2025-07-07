use std::any::Any;

use cubedaw_lib::State;

pub trait StateCommand: 'static + Send + Clone {
    fn run(&mut self, state: &mut State, action: ActionDirection);

    fn try_merge(&mut self, _other: &Self) -> bool {
        false
    }
}

pub trait StateCommandWrapper: 'static + Sealed + Send + Any {
    fn run(&mut self, state: &mut State, action: ActionDirection);

    fn try_merge(&mut self, other: &dyn StateCommandWrapper) -> bool;

    fn clone(&self) -> Box<dyn StateCommandWrapper>;
}

impl<T: StateCommand> Sealed for T {}
impl<T: StateCommand> StateCommandWrapper for T {
    fn run(&mut self, state: &mut State, action: ActionDirection) {
        StateCommand::run(self, state, action)
    }

    fn try_merge(&mut self, other: &dyn StateCommandWrapper) -> bool {
        if let Some(other) = (other as &dyn Any).downcast_ref() {
            StateCommand::try_merge(self, other)
        } else {
            false
        }
    }

    fn clone(&self) -> Box<dyn StateCommandWrapper> {
        Box::new(Clone::clone(self))
    }
}

impl dyn StateCommandWrapper {
    pub fn execute(&mut self, state: &mut State) {
        self.run(state, ActionDirection::Forward)
    }

    pub fn rollback(&mut self, state: &mut State) {
        self.run(state, ActionDirection::Reverse)
    }
}

impl Clone for Box<dyn StateCommandWrapper> {
    fn clone(&self) -> Self {
        StateCommandWrapper::clone(self.as_ref())
    }
}

impl std::fmt::Debug for dyn StateCommandWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StateCommandWrapper { .. }")
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ActionDirection {
    Forward,
    Reverse,
}
impl ActionDirection {
    pub fn is_execute(self) -> bool {
        matches!(self, ActionDirection::Forward)
    }
    pub fn is_rollback(self) -> bool {
        matches!(self, ActionDirection::Reverse)
    }
}

mod sealed {
    pub trait Sealed {}
}
pub(crate) use sealed::Sealed;
