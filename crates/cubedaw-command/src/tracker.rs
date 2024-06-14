use crate::{StateCommand, StateCommandWrapper};

#[derive(Default)]
pub struct StateTracker(Vec<Box<dyn StateCommandWrapper>>);

impl StateTracker {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn add(&mut self, command: impl StateCommand) {
        self.0.push(Box::new(command));
    }
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }
    pub fn take(&mut self) -> StateTracker {
        core::mem::take(self)
    }
    pub fn finish(self) -> Vec<Box<dyn StateCommandWrapper>> {
        self.0
    }
}
