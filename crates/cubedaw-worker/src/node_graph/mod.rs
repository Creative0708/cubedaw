// mod group_track;
// mod note;
// mod clip_track;

use ahash::HashSetExt;
use anyhow::Context;
use cubedaw_lib::{Buffer, Id, IdMap, IdSet, InternalBufferType, Node, Patch};
use resourcekey::ResourceKey;

use crate::{WorkerOptions, WorkerState, plugin::AttributeMap, util};

mod synth_note;
pub use synth_note::NoteNodeGraph;
mod synth_track;
pub use synth_track::TrackNodeGraph;

#[derive(Clone, Debug)]
/// A node graph. This is designed for fast updates and interactivity instead of performance.
pub struct PreparedNodeGraph {
    // TODO: tbh this would work better as a vec of input nodes for more generic node graphs
    input_node: Option<Id<Node>>,
    // TODO: same
    output_node: Id<Node>,

    id_to_index: IdMap<Node, u32>,
    nodes: Vec<NodeGraphEntry>,
}

impl PreparedNodeGraph {
    // pub fn new(
    //     patch: &Patch,
    //     options: &WorkerOptions,
    //     input_node: Option<Id<NodeEntry>>,
    //     output_node: Id<NodeEntry>,
    // ) -> Self {
    //     let mut this = Self::empty(input_node, output_node);

    //     this.sync_with(patch, options, input_node, output_node);

    //     this
    // }

    // this could be optimized to incrementally update the nodes when the graph is changed
    // but an O(n) change is fine because rendering is also O(n) and the performance impact is (probably) negligible.
    // if/when rendering gets optimized to use a texture or whatever and this becomes a bottleneck maybe we can optimize this.
    pub fn sync_with(
        &mut self,
        patch: &Patch,
        options: &WorkerOptions,
        input_node: Option<Id<Node>>,
        output_node: Id<Node>,
    ) {
        self.input_node = input_node;
        self.output_node = output_node;

        let mut node_id_to_vec_index_map: IdMap<Node, u32> = IdMap::new();

        let mut prev_entries: IdMap<Node, NodeGraphEntry> = IdMap::new();
        for graph_entry in self.nodes.drain(..) {
            prev_entries.insert(graph_entry.node_id, graph_entry);
        }

        // simple topo sort algo. TODO possibly replace with a faster one

        let mut indegrees: IdMap<Node, u32> = IdMap::new();

        let mut zero_indegree_node_stack = Vec::new();

        // dfs nodes between the input and output nodes
        {
            let mut stack = vec![output_node];
            let mut visited = IdSet::new();
            visited.insert(output_node);
            while let Some(node_id) = stack.pop() {
                let node = patch
                    .node_entry(node_id)
                    .expect("cable connected to nonexistent node???");

                let indegree = node
                    .inputs()
                    .iter()
                    .map(|input| input.connections.len())
                    .sum::<usize>() as u32;
                if indegree == 0 || Some(node_id) == input_node {
                    zero_indegree_node_stack.push(node_id);
                } else {
                    indegrees.insert(node_id, indegree);
                    for input in node.inputs() {
                        for (_, cable) in input
                            .get_connections(patch)
                            .filter(|(_, cable)| cable.tag.is_valid())
                        {
                            let new_node_id = cable.input_node;
                            if visited.insert(new_node_id) {
                                stack.push(new_node_id);
                            }
                        }
                    }
                }
            }

            if let Some(input_node) = input_node {
                if !visited.contains(&input_node) {
                    // the input node isn't connected to the output node! just insert a "dummy" entry in that case.
                    node_id_to_vec_index_map.insert(input_node, self.nodes.len() as u32);
                    self.nodes.push(NodeGraphEntry {
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                        key: patch.node(input_node).expect("unreachable").key.clone(),
                        node_id: input_node,
                        args: Default::default(),
                        state: Default::default(),
                        original_state: Default::default(),
                    });
                }
            }
        }

        while let Some(node_id) = zero_indegree_node_stack.pop() {
            let node = patch.node_entry(node_id).expect("unreachable");

            let mut entry = prev_entries.remove(node_id).unwrap_or_else(|| {
                let entry = options.registry.get(&node.data.key).expect("unreachable");
                let state: Box<Buffer> = (entry.node_factory)(node.data.inner.as_bytes())
                    .as_ref()
                    .into();
                NodeGraphEntry {
                    node_id,
                    key: node.data.key.clone(),

                    original_state: state.clone(),
                    state,

                    // these will be overwritten by the code below
                    args: Default::default(),
                    inputs: Default::default(),
                    outputs: Default::default(),
                }
            });

            let inputs = if Some(node_id) == input_node {
                // input nodes cannot have inputs by definition; prevent propagation to connected nodes
                &[]
            } else {
                node.inputs()
            };

            entry.inputs.resize_with(inputs.len(), || NodeGraphInput {
                connections: Default::default(),
                bias: Default::default(),
                buffer: Buffer::new_box_zeroed(options.buffer_size),
            });
            for (node_input, graph_input) in inputs.iter().zip(entry.inputs.iter_mut()) {
                graph_input.connections.resize_with(
                    node_input.connections.len(),
                    // dummy values
                    || NodeGraphCableConnection {
                        connection: u32::MAX,
                        output_index: u32::MAX,
                        multiplier: InterpolatedValue::default(),
                    },
                );
                for (cable, graph_connection) in node_input
                    .connected_cables()
                    .zip(graph_input.connections.iter_mut())
                {
                    let cable = patch.cable(cable).expect("unreachable");
                    graph_connection.connection = *node_id_to_vec_index_map.get(cable.input_node).with_context(||format!("node reachable with cables but not in map; this indicates an error in preprocessing\nnode: {:?}, index_map: {:?}", cable.input_node, node_id_to_vec_index_map)).unwrap();
                    graph_connection
                        .multiplier
                        .set_raw(cable.node_input_connection(patch).multiplier);
                    graph_connection.output_index = cable.input_output_index;
                }

                graph_input.bias.set_raw(node_input.bias);
            }

            entry
                .outputs
                .resize_with(node.outputs().len(), || NodeGraphOutput {
                    buffer: Buffer::new_box_zeroed(options.buffer_size),
                });

            entry.args.clone_from(&node.data.inner);

            node_id_to_vec_index_map.insert(node_id, self.nodes.len() as u32);
            self.nodes.push(entry);

            if node_id != output_node {
                // decrement indegrees
                for output in node.outputs() {
                    for (_cable_id, cable) in output
                        .get_connections(patch)
                        .filter(|(_, cable)| cable.tag.is_valid())
                    {
                        // if the node isn't in the indegrees map that means it's disconnected. just ignore it in that case
                        if let Some(indegree) = indegrees.get_mut(cable.output_node) {
                            *indegree -= 1;
                            if *indegree == 0 {
                                zero_indegree_node_stack.push(cable.output_node);
                                indegrees.remove(cable.output_node);
                            }
                        }
                    }
                }
            }
        }
        assert!(
            indegrees.is_empty() && zero_indegree_node_stack.is_empty(),
            "cycle detected in node graph"
        );
        assert!(
            self.nodes.len() <= u32::MAX as usize,
            "self.nodes.len() exceeds u32::MAX"
        );

        self.id_to_index = node_id_to_vec_index_map;
    }
    pub fn empty(input_node: Option<Id<Node>>, output_node: Id<Node>) -> Self {
        Self {
            input_node,
            output_node,

            id_to_index: IdMap::new(),
            nodes: Vec::new(),
        }
    }

    pub fn input_node(&self) -> Option<Id<Node>> {
        self.input_node
    }
    pub fn output_node(&self) -> Id<Node> {
        self.output_node
    }

    pub fn get_node(&self, note_id: Id<Node>) -> Option<&NodeGraphEntry> {
        self.id_to_index
            .get(note_id)
            .map(|&index| &self.nodes[index as usize])
    }
    pub fn get_node_mut(&mut self, note_id: Id<Node>) -> Option<&mut NodeGraphEntry> {
        self.id_to_index
            .get(note_id)
            .map(|&index| &mut self.nodes[index as usize])
    }

    pub fn process(
        &mut self,
        options: &WorkerOptions,
        state: &mut WorkerState,
        attribute_map: &mut dyn AttributeMap,
    ) -> anyhow::Result<()> {
        // self.nodes has been topologically sorted so the all dependencies of a node appear before it in the vec
        for index in 0..self.nodes.len() {
            let (previous_nodes, [node, ..]) = self.nodes.split_at_mut(index) else {
                unreachable!(
                    "index <= self.nodes.len() - 1 at all times so the right side is nonempty"
                );
            };

            if Some(node.node_id) == self.input_node {
                // skip input node if it exists
                continue;
            }

            for input in &mut node.inputs {
                input.bias.fill_buffer(&mut input.buffer);
                for &mut NodeGraphCableConnection {
                    connection,
                    output_index,
                    ref mut multiplier,
                } in &mut input.connections
                {
                    let connected_node = &previous_nodes[connection as usize];

                    for ((conn_val, buf_val), multiplier) in connected_node.outputs
                        [output_index as usize]
                        .buffer
                        .iter()
                        .zip(input.buffer.iter_mut())
                        .zip(multiplier.iter())
                    {
                        *buf_val += conn_val * multiplier;
                    }
                }
            }

            let registry_entry = options
                .registry
                .get(&node.key)
                .expect("desynced node graph");
            match registry_entry.plugin_data {
                Some(ref _plugin_data) => {
                    let mut plugin = state
                        .standalone_instances
                        .get(&node.key)
                        .expect("desynced node graph")
                        .borrow_mut();
                    let data = plugin.store_mut().data_mut();

                    data.inputs.resize_with(node.inputs.len(), Default::default);
                    data.outputs
                        .resize_with(node.outputs.len(), Default::default);

                    for sample_idx in 0..options.buffer_size as usize / InternalBufferType::N {
                        let data = plugin.store_mut().data_mut();

                        for (input_idx, input) in node.inputs.iter().enumerate() {
                            data.inputs[input_idx] = input.buffer.as_internal()[sample_idx];
                        }

                        plugin.run(
                            &node.key,
                            node.args.as_bytes(),
                            node.state.as_bytes_mut(),
                            attribute_map,
                        )?;

                        let data = plugin.store_mut().data_mut();

                        for (output_idx, output) in node.outputs.iter_mut().enumerate() {
                            output.buffer.as_internal_mut()[sample_idx] = data.outputs[output_idx];
                        }
                    }
                }
                None => {
                    // special passthrough logic
                    for (input, output) in node.inputs.iter().zip(node.outputs.iter_mut()) {
                        output.buffer.copy_from(&input.buffer);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        for node in &mut self.nodes {
            node.reset();
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeGraphEntry {
    state: Box<Buffer>,
    inputs: Vec<NodeGraphInput>,
    outputs: Vec<NodeGraphOutput>,

    key: ResourceKey,
    node_id: Id<Node>,
    args: Box<Buffer>,
    original_state: Box<Buffer>,
}
impl NodeGraphEntry {
    fn reset(&mut self) {
        self.state.copy_from_slice(&self.original_state);
    }

    pub fn add_dummy_output(&mut self, options: &WorkerOptions) {
        self.outputs = vec![NodeGraphOutput {
            buffer: Buffer::new_box_zeroed(options.buffer_size),
        }];
    }
}

#[derive(Clone, Copy, Debug)]
struct InterpolatedValue {
    raw_value: f32,
    interpolated_value: f32,
}
impl Default for InterpolatedValue {
    fn default() -> Self {
        Self {
            raw_value: f32::NAN,
            interpolated_value: f32::NAN,
        }
    }
}
impl InterpolatedValue {
    pub fn set_raw(&mut self, val: f32) {
        self.raw_value = val;
        if self.interpolated_value.is_nan() {
            self.interpolated_value = val;
        }
    }
    pub fn fill_buffer(&mut self, buf: &mut [f32]) {
        for (val, dst) in self.iter().zip(buf) {
            *dst = val;
        }
    }
    pub fn iter(&mut self) -> impl Iterator<Item = f32> {
        let is_raw = if (self.raw_value - self.interpolated_value).abs() < f32::EPSILON {
            self.interpolated_value = self.raw_value;
            true
        } else {
            false
        };
        gen move {
            loop {
                if is_raw {
                    yield self.raw_value;
                } else {
                    yield self.interpolated_value;
                    // TODO: not hardcode this. this should also be dependent on the sample rate
                    self.interpolated_value =
                        util::lerp(self.interpolated_value, self.raw_value, 0.005);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct NodeGraphInput {
    connections: Vec<NodeGraphCableConnection>,
    bias: InterpolatedValue,
    buffer: Box<Buffer>,
}
#[derive(Clone, Debug)]
struct NodeGraphOutput {
    buffer: Box<Buffer>,
}

#[derive(Clone, Debug)]
struct NodeGraphCableConnection {
    connection: u32,
    output_index: u32,
    multiplier: InterpolatedValue,
}
