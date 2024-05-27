use cubedaw_lib::{Id, NodeState, NodeStateWrapper};

pub type DynNode = Box<dyn NodeWrapper>;

pub enum DataSource<'a> {
    Const(f32),
    NodeOutput(&'a [f32]),
}
impl<'a> DataSource<'a> {
    pub fn get(&self, i: u32) -> f32 {
        match self {
            Self::Const(val) => *val,
            Self::NodeOutput(buf) => buf[i as usize],
        }
    }
}
pub enum DataDrain<'a> {
    Disconnected,
    NodeInput(&'a mut [f32]),
}
impl<'a> DataDrain<'a> {
    pub fn set(&mut self, i: u32, val: f32) {
        match self {
            Self::Disconnected => (),
            Self::NodeInput(buf) => {
                buf[i as usize] = val;
            }
        }
    }
}

pub trait NodeContext<'a> {
    fn sample_rate(&self) -> u32;
    fn buffer_size(&self) -> u32;
    fn input(&self, index: u32) -> DataSource<'a>;
    fn output(&self, index: u32) -> DataDrain<'a>;
}

pub trait Node: 'static + Send {
    // the reason for the whole Self::Ui thing is to have a way to have the ui thread render without waiting for thread synchronization
    // (which could cause very bad ui delays.)
    // also we need to serialize the ui to disk and this provides a convenient struct to do so
    type State: NodeState;

    fn id(&self) -> Id<DynNode>;
    // -> (# of inputs, # of outputs)
    fn spec(&self) -> (u32, u32);

    fn process(&mut self, ctx: &mut dyn NodeContext<'_>);

    /// Creates a `Self::State` representing the current state
    fn create_state(&self) -> Self::State;
    /// Updates the current state from a `Self::State`
    fn update_from_state(&mut self, data: Self::State);
}

pub trait NodeWrapper: Send {
    fn id(&self) -> Id<DynNode>;
    fn spec(&self) -> (u32, u32);
    fn process(&mut self, ctx: &mut dyn NodeContext<'_>);

    fn create_state(&self) -> Box<dyn NodeStateWrapper>;

    fn update_from_state(&mut self, data: Box<dyn NodeStateWrapper>);
}

impl<T: Node> NodeWrapper for T {
    fn id(&self) -> Id<DynNode> {
        self.id()
    }

    fn spec(&self) -> (u32, u32) {
        self.spec()
    }

    fn process(&mut self, ctx: &mut dyn NodeContext<'_>) {
        self.process(ctx)
    }

    fn create_state(&self) -> Box<dyn NodeStateWrapper> {
        Box::new(Node::create_state(self))
    }

    fn update_from_state(&mut self, data: Box<dyn NodeStateWrapper>) {
        Node::update_from_state(
            self,
            *data
                .into_any()
                .downcast()
                .expect("update_from_state called with state of different type"),
        )
    }
}
