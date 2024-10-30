use std::collections::VecDeque;

use ahash::{HashSet, HashSetExt};

use crate::{Id, IdMap, IdSet, ResourceKey};

#[derive(Debug, Default, Clone)]
pub struct Patch {
    nodes: IdMap<NodeEntry>,
    cables: IdMap<Cable>,
}

impl Patch {
    pub fn new() -> Self {
        Self::default()
    }

    /// If the provided node was added, what would its tag be?
    pub fn get_node_tag_if_added(&self, node: &NodeData) -> NodeTag {
        // nodes have no tag on their own but certain special nodes have their own NodeTag
        static SPECIAL_NODES: std::sync::LazyLock<HashSet<ResourceKey>> =
            std::sync::LazyLock::new(|| {
                let mut map = HashSet::new();
                map.insert(ResourceKey::new("builtin:note_output").unwrap());
                map.insert(ResourceKey::new("builtin:track_input").unwrap());
                map.insert(ResourceKey::new("builtin:track_output").unwrap());
                map
            });

        if SPECIAL_NODES.contains(&node.key) {
            NodeTag::Special
        } else {
            NodeTag::Disconnected
        }
    }

    pub fn insert_node(
        &mut self,
        node_id: Id<NodeEntry>,
        node: NodeData,
        inputs: Vec<f32>,
        num_outputs: u32,
    ) {
        assert!(
            inputs.len() <= u32::MAX as usize,
            "# of inputs exceeds {}",
            u32::MAX
        );

        self.nodes.insert(
            node_id,
            NodeEntry {
                tag: self.get_node_tag_if_added(&node),

                data: node,
                inputs: inputs
                    .into_iter()
                    .map(|bias| NodeInput {
                        bias,
                        connections: Vec::new(),
                    })
                    .collect(),
                outputs: vec![
                    NodeOutput {
                        connections: Vec::new(),
                    };
                    num_outputs as usize
                ],
            },
        );
    }
    pub fn remove_node(&mut self, node_id: Id<NodeEntry>) -> Option<NodeData> {
        Some(self.remove_entry(node_id)?.data)
    }
    pub fn remove_entry(&mut self, node_id: Id<NodeEntry>) -> Option<NodeEntry> {
        let entry = self.nodes.remove(node_id)?;
        // TODO what do we do in this scenario
        assert!(
            entry.inputs.is_empty() && entry.outputs.is_empty(),
            "unimplemented :("
        );
        Some(entry)
    }
    pub fn nodes(&self) -> impl Iterator<Item = (Id<NodeEntry>, &NodeEntry)> {
        self.nodes.iter().map(|(&id, data)| (id, data))
    }
    pub fn node(&self, id: Id<NodeEntry>) -> Option<&NodeData> {
        self.nodes.get(id).map(|entry| &entry.data)
    }
    pub fn node_mut(&mut self, id: Id<NodeEntry>) -> Option<&mut NodeData> {
        self.nodes.get_mut(id).map(|entry| &mut entry.data)
    }
    pub fn node_entry(&self, id: Id<NodeEntry>) -> Option<&NodeEntry> {
        self.nodes.get(id)
    }
    pub fn node_entry_mut(&mut self, id: Id<NodeEntry>) -> Option<&mut NodeEntry> {
        self.nodes.get_mut(id)
    }

    pub fn cables(&self) -> impl Iterator<Item = (Id<Cable>, &Cable)> {
        self.cables.iter().map(|(&id, data)| (id, data))
    }
    pub fn cable(&self, id: Id<Cable>) -> Option<&Cable> {
        self.cables.get(id)
    }
    pub fn cable_mut(&mut self, id: Id<Cable>) -> Option<&mut Cable> {
        self.cables.get_mut(id)
    }

    /// If the provided cable was added, what would its tag be?
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
        cable.tag = self.get_cable_tag_if_added(&cable);

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
    pub bias: f32,
    // the connections are additive to the value
    pub connections: Vec<Id<Cable>>,
}
impl NodeInput {
    pub fn get_connections<'a>(
        &'a self,
        patch: &'a Patch,
    ) -> impl Iterator<Item = (Id<Cable>, &'a Cable)> {
        self.connections.iter().map(move |&cable_id| {
            (
                cable_id,
                patch
                    .cable(cable_id)
                    .expect("cable doesn't exist on patch??"),
            )
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeOutput {
    pub connections: Vec<Id<Cable>>,
}
impl NodeOutput {
    pub fn get_connections<'a>(
        &'a self,
        patch: &'a Patch,
    ) -> impl Iterator<Item = (Id<Cable>, &'a Cable)> {
        self.connections.iter().map(move |&cable_id| {
            (
                cable_id,
                patch
                    .cable(cable_id)
                    .expect("cable doesn't exist on patch??"),
            )
        })
    }
}

#[derive(Debug, Clone)]
pub struct Cable {
    // fyi the "input node" is the node to which the _output_ is connected to this cable.
    // it's called this way because it's the node which is the input to this cable. confusing
    pub input_node: Id<NodeEntry>,
    // TODO rename
    pub input_output_index: u32,

    pub output_node: Id<NodeEntry>,
    pub output_input_index: u32,

    pub output_multiplier_fac: f32,

    pub tag: CableTag,
}
impl Cable {
    pub fn new(
        input_node: Id<NodeEntry>,
        input_output_index: u32,
        output_node: Id<NodeEntry>,
        output_input_index: u32,
    ) -> Self {
        Self {
            input_node,
            input_output_index,

            output_node,
            output_input_index,

            output_multiplier_fac: 1.0,

            tag: CableTag::Disconnected,
        }
    }
    pub fn assert_valid(&self, patch: &Patch) {
        let (input_node, output_node) = (
            patch
                .node_entry(self.input_node)
                .expect("nonexistent input node"),
            patch
                .node_entry(self.output_node)
                .expect("nonexistent output node"),
        );

        if self.tag.is_valid() {
            assert_eq!(
                input_node.tag, output_node.tag,
                "node tags connected to the same valid cable should be equal\nleft: {input_node:#?}\nright: {output_node:#?}"
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
    pub key: ResourceKey,
    /// Node args. _Not_ state, which can change over time. This stays static.
    pub inner: Box<[u8]>,
}

impl NodeData {
    pub fn new_disconnected(node_type: ResourceKey, inner: Box<[u8]>) -> Self {
        Self {
            key: node_type,
            inner,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeEntry {
    pub data: NodeData,

    inputs: Vec<NodeInput>,
    outputs: Vec<NodeOutput>,
    tag: NodeTag,
}

impl NodeEntry {
    pub fn new(data: NodeData, num_inputs: u32, num_outputs: u32) -> Self {
        Self {
            data,
            inputs: {
                let mut vec = Vec::with_capacity(num_inputs as usize);
                for _ in 0..num_inputs {
                    vec.push(NodeInput {
                        bias: 1.0,
                        connections: Vec::new(),
                    });
                }
                vec
            },
            outputs: {
                let mut vec = Vec::with_capacity(num_outputs as usize);
                for _ in 0..num_outputs {
                    vec.push(NodeOutput {
                        connections: Vec::new(),
                    });
                }
                vec
            },
            tag: NodeTag::Disconnected,
        }
    }

    pub fn assert_valid(&self, patch: &Patch) {
        for input in &self.inputs {
            assert!(
                input.bias.is_finite(),
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

    pub fn inputs(&self) -> &[NodeInput] {
        &self.inputs
    }
    pub fn inputs_mut(&mut self) -> &mut [NodeInput] {
        &mut self.inputs
    }
    pub fn pop_input(&mut self) -> Option<NodeInput> {
        let last = self.inputs.pop()?;
        if !last.connections.is_empty() {
            // can't delete node or bad desyncs will happen
            self.inputs.push(last);
            return None;
        }
        Some(last)
    }
    pub fn push_input(&mut self, bias: f32) {
        self.inputs.push(NodeInput {
            bias,
            connections: Vec::new(),
        });

        assert!(
            self.inputs.len() <= u32::MAX as usize,
            "you got 4 billion inputs on your node there"
        );
    }

    pub fn outputs(&self) -> &[NodeOutput] {
        &self.outputs
    }
    pub fn outputs_mut(&mut self) -> &mut [NodeOutput] {
        &mut self.outputs
    }
    pub fn pop_output(&mut self) -> Option<NodeOutput> {
        let last = self.outputs.pop()?;
        if !last.connections.is_empty() {
            // can't delete node or bad desyncs will happen
            self.outputs.push(last);
            return None;
        }
        Some(last)
    }
    pub fn push_output(&mut self) {
        self.outputs.push(NodeOutput {
            connections: Vec::new(),
        });

        assert!(
            self.outputs.len() <= u32::MAX as usize,
            "you got 4 billion outputs on your node there"
        );
    }

    pub fn tag(&self) -> NodeTag {
        self.tag
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
