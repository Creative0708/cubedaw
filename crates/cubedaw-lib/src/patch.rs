use std::sync::Arc;

use egui::mutex::Mutex;

use crate::{Id, IdMap, ResourceKey};

#[derive(Debug)]
pub struct Patch {
    nodes: Vec<Id<NodeData>>,
}

impl Patch {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
    pub fn add(&mut self, nodes: &mut IdMap<NodeData>, node: NodeData) {
        self.nodes.push(nodes.create(node));
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
}

impl NodeData {
    pub fn disconnected(node_type: Id<ResourceKey>) -> Self {
        Self {
            node_type,
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }
}
