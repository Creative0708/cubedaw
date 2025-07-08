use std::{collections::VecDeque, ops};

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};

use crate::{Buffer, Id, IdMap, IdSet, ResourceKey};

#[derive(Debug, Default, Clone)]
pub struct Patch {
    nodes: IdMap<Node>,
    cables: IdMap<Cable>,
}

impl Patch {
    pub fn new() -> Self {
        Self::default()
    }

    /// If the provided node was added, what would its tag be?
    pub fn get_node_tag_if_added(&self, node: &NodeData) -> NodeTag {
        // nodes have no tag on their own but certain special nodes have their own NodeTag
        static SPECIAL_NODES: std::sync::LazyLock<HashMap<ResourceKey, NodeTag>> =
            std::sync::LazyLock::new(|| {
                let mut map = HashMap::new();
                map.insert(resourcekey::literal!("builtin:input"), NodeTag::Monophonic);
                map.insert(resourcekey::literal!("builtin:downmix"), NodeTag::Downmix);
                map.insert(resourcekey::literal!("builtin:output"), NodeTag::Monophonic);
                map
            });

        SPECIAL_NODES.get(&node.key).copied().unwrap_or_default()
    }

    pub fn insert_node(
        &mut self,
        node_id: Id<Node>,
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
            Node {
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
    pub fn remove_node(&mut self, node_id: Id<Node>) -> Option<NodeData> {
        Some(self.remove_entry(node_id)?.data)
    }
    pub fn remove_entry(&mut self, node_id: Id<Node>) -> Option<Node> {
        let entry = self.nodes.remove(node_id)?;
        assert!(
            entry.connected_cables().next().is_none(),
            "unimplemented :("
        );
        Some(entry)
    }
    pub fn nodes(&self) -> impl Iterator<Item = (Id<Node>, &Node)> {
        self.nodes.iter().map(|(id, data)| (id, data))
    }
    pub fn node(&self, id: Id<Node>) -> Option<&NodeData> {
        self.nodes.get(id).map(|entry| &entry.data)
    }
    pub fn node_mut(&mut self, id: Id<Node>) -> Option<&mut NodeData> {
        self.nodes.get_mut(id).map(|entry| &mut entry.data)
    }
    pub fn node_entry(&self, id: Id<Node>) -> Option<&Node> {
        self.nodes.get(id)
    }
    pub fn node_entry_mut(&mut self, id: Id<Node>) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    pub fn cables(&self) -> impl Iterator<Item = (Id<Cable>, &Cable)> {
        self.cables.iter().map(|(id, data)| (id, data))
    }
    pub fn cable(&self, id: Id<Cable>) -> Option<&Cable> {
        self.cables.get(id)
    }
    pub fn cable_mut(&mut self, id: Id<Cable>) -> Option<&mut Cable> {
        self.cables.get_mut(id)
    }

    pub fn get_nodes(&self, key: &ResourceKey) -> Vec<Id<Node>> {
        self.nodes()
            .filter_map(|(id, val)| (&val.data.key == key).then_some(id))
            .collect()
    }
    pub fn get_active_node(&self, key: &ResourceKey) -> Option<Id<Node>> {
        let nodes = self.get_nodes(key);
        match *nodes {
            [] => None,
            [id] => Some(id),
            _ => todo!("multiple nodes aren't implemented yet. (found multiple {key:?} nodes)"),
        }
    }

    /// If the provided cable was added, what would its tag be?
    pub fn get_cable_tag_if_added(&self, cable: &Cable) -> CableTag {
        let input_node = self.nodes.force_get(cable.input_node);
        let output_node = self.nodes.force_get(cable.output_node);

        let Some(cable_tag) = input_node.tag.get_cable_tag_between(output_node.tag) else {
            return CableTag::Invalid;
        };

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
                for cable_id in input.connected_cables() {
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
        cable_tag
    }

    pub fn insert_cable(
        &mut self,
        cable_id: Id<Cable>,
        mut cable: Cable,
        conn: CableConnection,
    ) -> &mut CableConnection {
        cable.tag = self.get_cable_tag_if_added(&cable);

        let Cable {
            input_node: input_node_id,
            input_output_index,
            output_node: output_node_id,
            output_input_index,
            output_cable_index,
            ..
        } = cable;

        let input_node = self.nodes.force_get_mut(input_node_id);
        let input_output = &mut input_node.outputs[input_output_index as usize];
        input_output.connections.push(cable_id);

        let output_node = self.nodes.force_get_mut(output_node_id);
        let output_input = &mut output_node.inputs[output_input_index as usize];

        for &(node_id, _) in &output_input.connections[output_cable_index as usize..] {
            self.cables.force_get_mut(node_id).output_cable_index += 1;
        }
        output_input
            .connections
            .insert(output_cable_index as usize, (cable_id, conn));

        self.cables.insert(cable_id, cable);

        self.recalculate_tags();

        let output_node = self.nodes.force_get_mut(output_node_id);
        let output_input = &mut output_node.inputs[output_input_index as usize];
        &mut output_input.connections[output_cable_index as usize].1
    }
    pub fn remove_cable(&mut self, cable_id: Id<Cable>) -> Option<(Cable, CableConnection)> {
        let cable = self.cables.remove(cable_id)?;

        let input_node = self.nodes.force_get_mut(cable.input_node);
        let input_output = &mut input_node.outputs[cable.input_output_index as usize];
        let cable_index = input_output
            .connections
            .iter()
            .position(|&x| x == cable_id)
            .expect("node output doesn't have an entry for connected cable");
        input_output.connections.remove(cable_index);

        let output_node = self.nodes.force_get_mut(cable.output_node);
        let output_input = &mut output_node.inputs[cable.output_input_index as usize];
        let cable_index = output_input
            .connections
            .iter()
            .position(|&(id, _)| id == cable_id)
            .expect("node input doesn't have an entry for connected cable");
        let (_, conn) = output_input.connections.remove(cable_index);
        for &(conn_id, _) in &output_input.connections[cable_index..] {
            self.cables.force_get_mut(conn_id).output_cable_index -= 1;
        }

        self.recalculate_tags();

        Some((cable, conn))
    }
    pub fn take_cable(&mut self, cable_id: Id<Cable>) -> (Cable, CableConnection) {
        self.remove_cable(cable_id)
            .expect("take_cable() failed: cable doesn't exist in patch")
    }

    // TODO: this is O(n). possibly change to incremental updates later?
    // again, rendering is also O(n) so this isn't needed in most cases. optimization also isn't needed if the number of nodes is like < 10000.
    fn recalculate_tags(&mut self) {
        // also TODO: multiple of the same special nodes aren't implemented yet. we should be like blender where you can choose which node is actually "active"
        // or: make it so that adding a special node deletes the previous one

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum VisitedState {
            // the node is currently being visited
            Active,
            // the node has been visited already and has this tag
            Inactive(NodeTag),
        }

        // reset everything
        for node in self.nodes.values_mut() {
            node.tag = NodeTag::Disconnected;
        }
        for cable in self.cables.values_mut() {
            cable.tag = CableTag::Disconnected;
        }

        fn do_dfs_backwards(
            patch: &mut Patch,
            visited: &mut IdMap<Node, VisitedState>,

            start_id: Id<Node>,
            node_tag: NodeTag,
            cable_tag: CableTag,
        ) {
            visited.insert(start_id, VisitedState::Active);
            let node = patch.nodes.force_get_mut(start_id);
            node.tag = node_tag;

            let connections: Vec<(Id<Cable>, CableConnection)> = node
                .inputs
                .iter()
                .flat_map(|input| input.connections.iter().cloned())
                .collect();

            for (cable_id, _conn) in connections {
                let cable = &mut patch.cables[cable_id];
                let other_node = cable.input_node;

                match visited.get(other_node).copied() {
                    None => {
                        // new node!
                        cable.tag = cable_tag;
                        do_dfs_backwards(patch, visited, other_node, node_tag, cable_tag);
                    }
                    Some(VisitedState::Inactive(other))
                        if other.cable_tag_for_output() == cable_tag =>
                    {
                        // cycle-less node connection in the same graph
                        cable.tag = cable_tag;
                    }
                    Some(VisitedState::Active | VisitedState::Inactive(_)) => {
                        // we either found a cycle (VisitedState::Active) or a visited node from another node type. this cable is invalid
                        cable.tag = CableTag::Invalid;
                    }
                }
            }

            assert_eq!(
                visited.replace(start_id, VisitedState::Inactive(node_tag)),
                Some(VisitedState::Active)
            );
        }

        let mut visited = IdMap::new();

        if let Some(node) = self.get_active_node(&resourcekey::literal!("builtin:input")) {
            visited.insert(node, VisitedState::Inactive(NodeTag::Monophonic));
            self[node].tag = NodeTag::Monophonic;
        }
        if let Some(node) = self.get_active_node(&resourcekey::literal!("builtin:downmix")) {
            do_dfs_backwards(
                self,
                &mut visited,
                node,
                NodeTag::Multiphonic,
                CableTag::Multiphonic,
            );
            visited.replace(node, VisitedState::Inactive(NodeTag::Downmix));
        }
        if let Some(node) = self.get_active_node(&resourcekey::literal!("builtin:output")) {
            do_dfs_backwards(
                self,
                &mut visited,
                node,
                NodeTag::Monophonic,
                CableTag::Monophonic,
            );
        }
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

impl ops::Index<Id<Node>> for Patch {
    type Output = Node;
    fn index(&self, id: Id<Node>) -> &Self::Output {
        &self.nodes[id]
    }
}
impl ops::IndexMut<Id<Node>> for Patch {
    fn index_mut(&mut self, id: Id<Node>) -> &mut Self::Output {
        &mut self.nodes[id]
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeInput {
    pub bias: f32,
    // connections are additive to the value
    pub connections: Vec<(Id<Cable>, CableConnection)>,
}
impl NodeInput {
    pub fn connected_cables(&self) -> impl Iterator<Item = Id<Cable>> + '_ {
        self.connections.iter().map(|(id, _)| *id)
    }
    /// Convenience function.
    pub fn get_connections<'a>(
        &'a self,
        patch: &'a Patch,
    ) -> impl Iterator<Item = (&'a CableConnection, &'a Cable)> {
        self.connections.iter().map(move |(id, conn)| {
            (
                conn,
                patch.cable(*id).expect("cable doesn't exist on patch??"),
            )
        })
    }
}

#[derive(Debug, Clone)]
pub struct CableConnection {
    pub multiplier: f32,
}
impl Default for CableConnection {
    fn default() -> Self {
        Self { multiplier: 0.2 }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeOutput {
    pub connections: Vec<Id<Cable>>,
}
impl NodeOutput {
    pub fn connected_cables(&self) -> impl Iterator<Item = Id<Cable>> + '_ {
        self.connections.iter().copied()
    }
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
    // fyi the "input node" is the node to which its _output_ is connected to this cable.
    // it's called this way because it's the node which is the input to this cable. confusing
    pub input_node: Id<Node>,
    pub input_output_index: u32,

    pub output_node: Id<Node>,
    pub output_input_index: u32,
    pub output_cable_index: u32,

    pub tag: CableTag,
}
impl Cable {
    pub fn new(
        input_node: Id<Node>,
        input_output_index: u32,
        output_node: Id<Node>,
        output_input_index: u32,
        output_cable_index: u32,
    ) -> Self {
        Self {
            input_node,
            input_output_index,

            output_node,
            output_input_index,
            output_cable_index,

            tag: CableTag::Disconnected,
        }
    }
    pub fn one(input_node: Id<Node>, output_node: Id<Node>) -> Self {
        Self::new(input_node, 0, output_node, 0, 0)
    }

    pub fn input_node<'a>(&self, patch: &'a Patch) -> &'a Node {
        patch
            .node_entry(self.input_node)
            .expect("cable doesn't belong to patch")
    }
    pub fn output_node<'a>(&self, patch: &'a Patch) -> &'a Node {
        patch
            .node_entry(self.output_node)
            .expect("cable doesn't belong to patch")
    }
    pub fn node_input<'a>(&self, patch: &'a Patch) -> &'a NodeInput {
        &self.output_node(patch).inputs[self.output_input_index as usize]
    }
    pub fn node_input_connection<'a>(&self, patch: &'a Patch) -> &'a CableConnection {
        let (_id, ref cable) = self.node_input(patch).connections[self.output_cable_index as usize];
        cable
    }

    pub fn assert_valid(&self, patch: &Patch) {
        let input_node = &patch[self.input_node];
        let output_node = &patch[self.output_node];

        match (self.tag, input_node.tag(), output_node.tag()) {
            (CableTag::Monophonic, NodeTag::Monophonic | NodeTag::Downmix, NodeTag::Monophonic)
            | (
                CableTag::Multiphonic,
                NodeTag::Multiphonic,
                NodeTag::Multiphonic | NodeTag::Downmix,
            )
            | (CableTag::Disconnected, _, NodeTag::Disconnected) => (),
            _ => panic!(
                "incompatible node tags for cable {self:?}\ninput: {input_node:#?}\noutput: {output_node:#?}"
            ),
        }
    }
}

/// What status a cable can be in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CableTag {
    /// The cable is multiphonic (each note has one instance of this cable).
    Multiphonic,
    /// The cable is monophonic (each track has one instance of this cable).
    Monophonic,

    /// The cable, if added, would result in an invalid patch (i.e. having cycles or connecting multiphonic and monophonic channels).
    Invalid,
    /// The cable doesn't cause an invalid patch but is unused when processing audio.
    Disconnected,
}
impl CableTag {
    /// Whether the cable is in one of the valid states.
    pub fn is_valid(self) -> bool {
        self != Self::Invalid
    }
}

#[derive(Debug, Clone)]
pub struct NodeData {
    pub key: ResourceKey,
    /// Node args. _Not_ state, which can change over time. This stays static.
    pub inner: Box<Buffer>,
}

impl NodeData {
    pub fn new_disconnected(node_type: ResourceKey, inner: Box<Buffer>) -> Self {
        Self {
            key: node_type,
            inner,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub data: NodeData,

    inputs: Vec<NodeInput>,
    outputs: Vec<NodeOutput>,
    tag: NodeTag,
}

impl Node {
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
            for cable_id in input.connected_cables() {
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

    pub fn connected_cables(&self) -> impl Iterator<Item = Id<Cable>> + '_ {
        self.inputs()
            .iter()
            .flat_map(|input| input.connections.iter().map(|conn| conn.0))
            .chain(
                self.outputs()
                    .iter()
                    .flat_map(|output| output.connections.iter().copied()),
            )
    }

    pub fn tag(&self) -> NodeTag {
        self.tag
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum NodeTag {
    #[default]
    /// The node is disconnected from the rest of the patch and doesn't contribute anything.
    Disconnected,
    /// The node is multiphonic and has one instance per note. This only applies to clip tracks.
    Multiphonic,
    /// The node is monophonic and has one instance per track. This applies to both clip and group tracks.
    Monophonic,
    /// The node is a downmix node (the input is multiphonic and the output is monophonic).
    Downmix,
}

impl NodeTag {
    pub fn cable_tag_for_input(self) -> CableTag {
        match self {
            Self::Disconnected => CableTag::Disconnected,
            Self::Multiphonic | Self::Downmix => CableTag::Multiphonic,
            Self::Monophonic => CableTag::Monophonic,
        }
    }
    pub fn cable_tag_for_output(self) -> CableTag {
        match self {
            Self::Disconnected => CableTag::Disconnected,
            Self::Multiphonic => CableTag::Multiphonic,
            Self::Monophonic | Self::Downmix => CableTag::Monophonic,
        }
    }

    fn get_cable_tag_between(self, other: Self) -> Option<CableTag> {
        let this_cable_tag = self.cable_tag_for_output();
        let other_cable_tag = other.cable_tag_for_input();
        if other_cable_tag == CableTag::Disconnected || this_cable_tag == other_cable_tag {
            Some(this_cable_tag)
        } else {
            None
        }
    }
}
