use crate::StateCommand;

#[derive(Debug, Default, Clone, Copy)]
pub struct DontMerge<T: StateCommand>(T);

impl<T: StateCommand> DontMerge<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }
}

impl<T: StateCommand> StateCommand for DontMerge<T> {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        self.0.execute(state)
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        self.0.rollback(state)
    }
    fn worker_execute(
        &mut self,
        worker_state: &mut cubedaw_workerlib::WorkerState,
        node_registry: &cubedaw_workerlib::NodeRegistry,
    ) {
        self.0.worker_execute(worker_state, node_registry)
    }
    fn worker_rollback(
        &mut self,
        worker_state: &mut cubedaw_workerlib::WorkerState,
        node_registry: &cubedaw_workerlib::NodeRegistry,
    ) {
        self.0.worker_rollback(worker_state, node_registry)
    }
}
