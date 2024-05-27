use ahash::HashSet;

use crate::{Id, IdMap, NodeStateWrapper, ResourceKey};

#[derive(Debug, Default)]
pub struct Patch {
    nodes: HashSet<Id<NodeData>>,
}

impl Patch {
    pub fn insert_node(
        &mut self,
        nodes: &mut IdMap<NodeData>,
        node_id: Id<NodeData>,
        node: NodeData,
    ) {
        nodes.insert(node_id, node);
        self.nodes.insert(node_id);
    }

    pub fn remove_node(&mut self, nodes: &mut IdMap<NodeData>, node_id: Id<NodeData>) -> NodeData {
        self.nodes.remove(&node_id);
        nodes.take(node_id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = Id<NodeData>> + '_ {
        self.nodes.iter().copied()
    }
}

#[derive(Debug)]
pub struct NodeInput {
    pub unconnected_value: f32,
    // TODO are two-way links the best way to do this? seems messy
    pub connection: Option<Id<NodeData>>,
}

#[derive(Debug)]
pub struct NodeOutput {
    pub connection: Option<Id<NodeData>>,
}

#[derive(Debug)]
pub struct NodeData {
    pub node_type: Id<ResourceKey>,
    pub inputs: Vec<NodeInput>,
    pub outputs: Vec<NodeOutput>,

    pub inner: Box<dyn NodeStateWrapper>,
}

impl NodeData {
    pub fn new_disconnected(node_type: Id<ResourceKey>, inner: Box<dyn NodeStateWrapper>) -> Self {
        Self {
            node_type,
            inputs: Vec::new(),
            outputs: Vec::new(),
            inner,
        }
    }
}
