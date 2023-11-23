use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    // TODO
}

impl State {
    pub fn new() -> Self {
        Self {}
    }
}

/// State changes used to efficiently broadcast changes to workers and also for the undo system
#[derive(Serialize, Deserialize)]
pub enum StateChange {
    DUMMY,
}

impl StateChange {
    pub fn apply(&self, state: &mut State) {
        todo!()
    }
    pub fn revert(&self, state: &mut State) {
        todo!()
    }
}
