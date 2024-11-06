// mod group_track;
// mod note;
// mod section_track;

use ahash::HashSetExt;
use cubedaw_lib::{Buffer, Id, IdMap, IdSet, InternalBufferType, NodeEntry, Patch};
use resourcekey::ResourceKey;

use crate::{WorkerOptions, WorkerState};

mod group;
pub use group::GroupNodeGraph;
mod synth_note;
pub use synth_note::SynthNoteNodeGraph;
mod synth_track;
pub use synth_track::SynthTrackNodeGraph;

#[derive(Clone, Debug)]
pub struct PreparedNodeGraph {
    input_node: Option<Id<NodeEntry>>,
    output_node: Id<NodeEntry>,

    id_to_index: IdMap<NodeEntry, u32>,
    nodes: Vec<NodeGraphEntry>,
}

impl PreparedNodeGraph {
    pub fn new(
        patch: &Patch,
        options: &WorkerOptions,
        input_node: Option<Id<NodeEntry>>,
        output_node: Id<NodeEntry>,
    ) -> Self {
        let mut this = Self::empty(input_node, output_node);

        this.sync_with(patch, options, input_node, output_node);

        this
    }

    // this could be optimized to incrementally update the nodes when the graph is changed
    // but an O(n) change is fine because rendering is also O(n) and the performance impact is (probably) negligible.
    // if/when rendering gets optimized to use a texture or whatever and this becomes a bottleneck maybe we can optimize this.
    pub fn sync_with(
        &mut self,
        patch: &Patch,
        options: &WorkerOptions,
        input_node: Option<Id<NodeEntry>>,
        output_node: Id<NodeEntry>,
    ) {
        self.input_node = input_node;
        self.output_node = output_node;

        let mut node_id_to_vec_index_map = IdMap::new();

        let mut old_node_inners = IdMap::new();
        for graph_entry in self.nodes.drain(..) {
            old_node_inners.insert(graph_entry.node_id, graph_entry.state);
        }

        // simple topo sort algo. TODO possibly replace with a faster one

        let mut indegrees: IdMap<NodeEntry, u32> = IdMap::new();

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
                    node_id_to_vec_index_map.insert(input_node, 0);
                    self.nodes.push(NodeGraphEntry {
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                        key: patch.node(input_node).expect("unreachable").key.clone(),
                        node_id: input_node,
                        args: Box::new([]),
                        state: Box::new([]),
                    });
                }
            }
        }

        while let Some(node_id) = zero_indegree_node_stack.pop() {
            let node = patch.node_entry(node_id).expect("unreachable");

            node_id_to_vec_index_map.insert(node_id, self.nodes.len() as u32);
            self.nodes.push(NodeGraphEntry {
                node_id,
                args: node.data.inner.clone(),
                key: node.data.key.clone(),

                state: old_node_inners.remove(node_id).unwrap_or_else(|| {
                    let entry = options.registry.get(&node.data.key).expect("unreachable");
                    (entry.node_factory)(&node.data.inner)
                }),
                inputs: node
                    .inputs()
                    .iter()
                    .map(|input| NodeGraphInput {
                        connections: {
                            input
                                .connected_cables()
                                .map(|cable_id| {
                                    let cable = patch
                                        .cable(cable_id)
                                        .expect("node connected to nonexistent cable");
                                    (
                                        *node_id_to_vec_index_map
                                            .get(cable.input_node)
                                            .unwrap_or_else(|| {
                                                panic!(
                                                    "{:?} {:?}",
                                                    cable, &node_id_to_vec_index_map
                                                )
                                            }),
                                        cable.input_output_index,
                                        cable.node_input_connection(patch).multiplier,
                                    )
                                })
                                .collect()
                        },
                        bias: input.bias,
                        buffer: Buffer::new_box_zeroed(options.buffer_size),
                    })
                    .collect(),
                outputs: node
                    .outputs()
                    .iter()
                    .map(|_output| Buffer::new_box_zeroed(options.buffer_size))
                    .collect(),
            });

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
    pub fn empty(input_node: Option<Id<NodeEntry>>, output_node: Id<NodeEntry>) -> Self {
        Self {
            input_node,
            output_node,

            id_to_index: IdMap::new(),
            nodes: Vec::new(),
        }
    }

    pub fn input_node(&self) -> Option<Id<NodeEntry>> {
        self.input_node
    }
    pub fn output_node(&self) -> Id<NodeEntry> {
        self.output_node
    }

    pub fn get_node(&self, note_id: Id<NodeEntry>) -> Option<&NodeGraphEntry> {
        self.id_to_index
            .get(note_id)
            .map(|&index| &self.nodes[index as usize])
    }
    pub fn get_node_mut(&mut self, note_id: Id<NodeEntry>) -> Option<&mut NodeGraphEntry> {
        self.id_to_index
            .get(note_id)
            .map(|&index| &mut self.nodes[index as usize])
    }

    pub fn process(
        &mut self,
        options: &WorkerOptions,
        state: &mut WorkerState,
    ) -> anyhow::Result<()> {
        // self.nodes has been topologically sorted so the all dependencies of a node appear before it in the vec
        for index in 0..self.nodes.len() {
            let (previous_nodes, [node, ..]) = self.nodes.split_at_mut(index) else {
                unreachable!(
                    "index <= self.nodes.len() - 1 at all times so the right side is nonempty"
                );
            };

            for input in &mut node.inputs {
                input.buffer.fill(input.bias);
                for &(connection, output_index, multiplier) in &input.connections {
                    let connected_node = &previous_nodes[connection as usize];
                    let zipped = connected_node.outputs[output_index as usize]
                        .iter()
                        .zip(input.buffer.iter_mut());

                    match multiplier {
                        0.0 => (),
                        1.0 => {
                            for (conn_val, buf_val) in zipped {
                                *buf_val += conn_val;
                            }
                        }
                        -1.0 => {
                            for (conn_val, buf_val) in zipped {
                                *buf_val -= conn_val;
                            }
                        }
                        multiplier => {
                            for (conn_val, buf_val) in zipped {
                                *buf_val += conn_val * multiplier;
                            }
                        }
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

                        plugin.run(&node.key, &node.args, &mut node.state)?;

                        let data = plugin.store_mut().data_mut();

                        for (output_idx, output) in node.outputs.iter_mut().enumerate() {
                            output.as_internal_mut()[sample_idx] = data.outputs[output_idx];
                        }
                    }
                }
                None => {
                    // special passthrough logic
                    for (input, output) in node.inputs.iter().zip(node.outputs.iter_mut()) {
                        output.copy_from(&input.buffer);
                    }
                }
            }

            // node.inner.process(
            //     &*node.state,
            //     &mut CubedawNodeContext {
            //         worker_options,
            //         previous_nodes,
            //         inputs: &inputs_vec,
            //         outputs: &mut node.outputs,
            //     },
            // );
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct NodeGraphEntry {
    pub state: Box<[u8]>,
    pub inputs: Vec<NodeGraphInput>,
    pub outputs: Vec<Box<Buffer>>,

    pub key: ResourceKey,
    pub node_id: Id<NodeEntry>,
    pub args: Box<[u8]>,
}
impl std::fmt::Debug for NodeGraphEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeGraphEntry")
            .field("key", &self.key)
            .field("inner", &format_args!("<len {}>", self.state.len()))
            .field("inputs", &self.inputs)
            .field("outputs", &format_args!("<{} outputs>", self.outputs.len()))
            .field("node_id", &self.node_id)
            .field("state", &format_args!("<len {}>", self.args.len()))
            .finish()
    }
}

#[derive(Clone)]
pub struct NodeGraphInput {
    connections: Vec<(u32, u32, f32)>,
    bias: f32,
    buffer: Box<Buffer>,
}

impl std::fmt::Debug for NodeGraphInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeGraphInput")
            .field("connections", &self.connections)
            .field("bias", &self.bias)
            .field("buffer", &format_args!("<len {}>", self.buffer.len()))
            .finish()
    }
}
