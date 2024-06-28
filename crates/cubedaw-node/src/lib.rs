use std::ops::Index;

use cubedaw_lib::{NodeState, NodeStateWrapper};

pub type DynNode = Box<dyn NodeWrapper>;

pub enum DataSource<'a> {
    Const(f32),
    NodeOutput(&'a [f32]),
}
impl Index<u32> for DataSource<'_> {
    type Output = f32;
    fn index(&self, index: u32) -> &Self::Output {
        match self {
            Self::Const(val) => val,
            Self::NodeOutput(buf) => &buf[index as usize],
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

pub trait Node: 'static + Sized + Send + Clone {
    // the reason for the whole Self::Ui thing is to have a way to have the ui thread render without waiting for thread synchronization
    // (which could cause very bad ui delays.)
    // also we need to be able to serialize the ui to disk and this provides a convenient struct to do so
    type State: NodeState;

    fn new() -> Self;
    fn new_state(creation_ctx: NodeCreationContext<'_>) -> Self::State;

    fn process(&mut self, state: &Self::State, ctx: &mut dyn NodeContext<'_>);
}

mod sealed {
    pub trait Sealed {}
}
/// Object-safe wrapper for `Node`. See [`Node`] for the actual functionality.
pub trait NodeWrapper: 'static + Send + sealed::Sealed {
    fn process(&mut self, state: &dyn NodeStateWrapper, ctx: &mut dyn NodeContext<'_>);

    fn clone(&self) -> Box<dyn NodeWrapper>;
}

impl<T: Node> sealed::Sealed for T {}
impl<T: Node> NodeWrapper for T {
    fn process(&mut self, state: &dyn NodeStateWrapper, ctx: &mut dyn NodeContext<'_>) {
        self.process(state.downcast_ref().expect("mismatched state type"), ctx)
    }

    fn clone(&self) -> Box<dyn NodeWrapper> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn NodeWrapper> {
    fn clone(&self) -> Self {
        NodeWrapper::clone(self.as_ref())
    }
}

// TODO should this _really_ be in cubedaw-node? seems like it would fit better in cubedaw
// but i don't know how that would work
#[derive(Default)]
pub struct NodeCreationContext<'a> {
    pub alias: Option<std::borrow::Cow<'a, str>>,
}
