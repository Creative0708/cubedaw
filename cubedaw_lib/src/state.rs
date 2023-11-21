
use serde::{Serialize, Deserialize};

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    // TODO
}

impl State {
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