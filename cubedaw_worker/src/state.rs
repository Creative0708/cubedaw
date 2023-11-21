use std::sync::{OnceLock, RwLock};

use crate::State;

pub fn worker_state() -> &'static RwLock<State>{
    static WORKER_STATE: OnceLock<RwLock<State>> = OnceLock::new();
    WORKER_STATE.get_or_init(|| RwLock::new(State::default()))
}
