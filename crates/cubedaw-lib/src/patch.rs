use std::collections::VecDeque;

use ahash::HashSetExt;

use crate::{Id, IdMap, IdSet, NodeStateWrapper, ResourceKey};

#[derive(Debug, Default, Clone)]
pub struct Patch {
    nodes: IdMap<NodeData>,
    cables: IdMap<Cable>,
}

impl Patch {
    pub fn insert_node(&mut self, node_id: Id<NodeData>, node: NodeData) {
        self.nodes.insert(node_id, node);
    }
    pub fn take_node(&mut self, node_id: Id<NodeData>) -> NodeData {
        self.nodes.take(node_id)
    }
    pub fn nodes(&self) -> impl Iterator<Item = (Id<NodeData>, &NodeData)> {
        self.nodes.iter().map(|(&id, data)| (id, data))
    }
    pub fn node(&self, id: Id<NodeData>) -> Option<&NodeData> {
        self.nodes.get(id)
    }
    pub fn node_mut(&mut self, id: Id<NodeData>) -> Option<&mut NodeData> {
        self.nodes.get_mut(id)
    }

    pub fn cables(&self) -> impl Iterator<Item = (Id<Cable>, &Cable)> {
        self.cables.iter().map(|(&id, data)| (id, data))
    }

    /// Convenience function.
    pub fn set_cable_tag(&self, cable: &mut Cable) {
        cable.tag = self.get_cable_tag_if_added(cable);
    }
    pub fn get_cable_tag_if_added(&self, cable: &Cable) -> CableTag {
        let input_node = self.nodes.force_get(cable.input_node);
        let output_node = self.nodes.force_get(cable.output_node);

        if !input_node.tag.is_compatible_with(output_node.tag) {
            return CableTag::Invalid;
        }

        // check if output node is before input node.
        // this is just BFS on a graph. i told you competitive programming would come in handy someday!
        let mut visited = IdSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(cable.input_node);
        visited.insert(cable.input_node);
        while let Some(id) = queue.pop_front() {
            if id == cable.output_node {
                // cycle detected! cable is invalid
                return CableTag::Invalid;
            }
            for input in &self.nodes.force_get(id).inputs {
                for &cable_id in &input.connections {
                    let cable = self.cables.force_get(cable_id);
                    if !cable.tag.is_valid() {
                        continue;
                    }
                    if visited.insert(cable.input_node) {
                        queue.push_back(cable.input_node);
                    }
                }
            }
        }
        // output node is not before input node. there are no cycles and it is valid!
        if input_node.tag == NodeTag::Disconnected && output_node.tag == NodeTag::Disconnected {
            CableTag::Disconnected
        } else {
            CableTag::Valid
        }
    }
    pub fn insert_cable(&mut self, cable_id: Id<Cable>, mut cable: Cable) {
        self.set_cable_tag(&mut cable);

        let input_node = self.nodes.force_get_mut(cable.input_node);
        let input_output = &mut input_node.outputs[cable.input_output_index as usize];
        input_output.connections.push(cable_id);

        let output_node = self.nodes.force_get_mut(cable.output_node);
        let output_input = &mut output_node.inputs[cable.output_input_index as usize];
        output_input.connections.push(cable_id);

        self.cables.insert(cable_id, cable);
    }
    pub fn take_cable(&mut self, cable_id: Id<Cable>) -> Cable {
        let cable = self.cables.take(cable_id);

        let input_node = self.nodes.force_get_mut(cable.input_node);
        let input_output = &mut input_node.outputs[cable.input_output_index as usize];
        let cable_index = input_output
            .connections
            .iter()
            .position(|&x| x == cable_id)
            .expect("node output doesn't have an entry for connected cable");
        input_output.connections.remove(cable_index);
        for &cable in &input_output.connections[cable_index..] {
            self.cables.force_get_mut(cable).input_output_index -= 1;
        }

        let output_node = self.nodes.force_get_mut(cable.output_node);
        let output_input = &mut output_node.inputs[cable.output_input_index as usize];
        let cable_index = output_input
            .connections
            .iter()
            .position(|&x| x == cable_id)
            .expect("node input doesn't have an entry for connected cable");
        output_input.connections.remove(cable_index);
        for &cable in &output_input.connections[cable_index..] {
            self.cables.force_get_mut(cable).output_input_index -= 1;
        }

        cable
    }

    pub fn debug_assert_valid(&self) {
        if cfg!(debug_assertions) {
            self.assert_valid();
        }
    }
    pub fn assert_valid(&self) {
        for (_, node) in self.nodes() {
            node.assert_valid(self);
        }
        for (_, cable) in &self.cables {
            cable.assert_valid(self);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeInput {
    pub value: f32,
    // the connections are additive to the value
    pub connections: Vec<Id<Cable>>,
}

#[derive(Debug, Clone, Default)]
pub struct NodeOutput {
    pub connections: Vec<Id<Cable>>,
}
#[derive(Debug, Clone)]
pub struct Cable {
    // fyi the "input node" is the node to which the _output_ is connected to this cable.
    // it's called this way bc it makes more sense (it's the node which is the input to
    // this cable). confusing
    pub input_node: Id<NodeData>,
    // TODO rename
    pub input_output_index: u32,

    pub output_node: Id<NodeData>,
    pub output_input_index: u32,

    pub output_multiplier_fac: f32,

    pub tag: CableTag,
}
impl Cable {
    pub fn assert_valid(&self, patch: &Patch) {
        let (input_node, output_node) = (
            patch.node(self.input_node).expect("nonexistent input node"),
            patch
                .node(self.output_node)
                .expect("nonexistent output node"),
        );

        if self.tag.is_valid() {
            assert_eq!(
                input_node.tag, output_node.tag,
                "node tags connected to the same valid cable should be equal"
            );
        }
    }
}

/// What status a cable can be in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CableTag {
    /// Nothing's wrong with the cable! :D
    Valid,
    /// The cable, if added, would result in an invalid patch (i.e. having cycles or the like).
    Invalid,
    /// The cable doesn't cause an invalid patch but is unused when processing audio.
    Disconnected,
}
impl CableTag {
    /// Whether the cable is in one of the valid states.
    pub fn is_valid(self) -> bool {
        match self {
            Self::Valid | Self::Disconnected => true,
            Self::Invalid => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeData {
    pub key: Id<ResourceKey>,
    pub inputs: Vec<NodeInput>,
    pub outputs: Vec<NodeOutput>,
    pub tag: NodeTag,

    pub inner: Box<dyn NodeStateWrapper>,
}

impl NodeData {
    pub fn new_disconnected(node_type: Id<ResourceKey>, inner: Box<dyn NodeStateWrapper>) -> Self {
        Self {
            key: node_type,
            inputs: Vec::new(),
            outputs: Vec::new(),
            tag: NodeTag::Disconnected,

            inner,
        }
    }

    pub fn assert_valid(&self, patch: &Patch) {
        for input in &self.inputs {
            assert!(
                input.value.is_finite(),
                "i'm impressed you got this panic tbh. (node input value is infinite or NaN)"
            );
            for &cable_id in &input.connections {
                assert!(
                    patch.cables.has(cable_id),
                    "node connected with nonexistent cable"
                );
            }
        }
        for output in &self.outputs {
            for &cable_id in &output.connections {
                assert!(
                    patch.cables.has(cable_id),
                    "node connected with nonexistent cable"
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum NodeTag {
    #[default]
    Disconnected,
    Note,
    Track,
    Special,
}

impl NodeTag {
    fn is_compatible_with(self, other: Self) -> bool {
        match (self, other) {
            (Self::Special, _) | (_, Self::Special) => true,
            (a, b) if a == b => true,
            _ => false,
        }
    }
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum NodeRelation {
//     Ancestor,
//     Descendant,
//     Disconnected
// }
