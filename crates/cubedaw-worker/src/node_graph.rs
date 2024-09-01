use std::cell::Cell;

use ahash::HashSetExt;
use cubedaw_lib::{DynNode, DynNodeState, Id, IdMap, IdSet, NodeEntry, NodeStateWrapper, Patch};

use crate::WorkerOptions;

#[derive(Clone, Debug)]
pub struct ProcessedNodeGraph {
    input_node: Option<Id<NodeEntry>>,
    output_node: Id<NodeEntry>,

    id_to_index: IdMap<NodeEntry, u32>,
    nodes: Vec<NodeGraphEntry>,
}

impl ProcessedNodeGraph {
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

        // simple topo sort algo. TODO possibly replace with a faster one

        let mut indegrees: IdMap<NodeEntry, u32> = IdMap::new();

        let mut zero_indegree_node_stack = Vec::new();

        // dfs nodes between the input and output nodes
        {
            let mut stack = vec![output_node];
            let mut visited = IdSet::new();
            while let Some(node_id) = stack.pop() {
                let node = patch
                    .node_entry(node_id)
                    .expect("cable connected to nonexistent node???");

                if node.inputs().is_empty() || Some(node_id) == input_node {
                    zero_indegree_node_stack.push(node_id);
                } else {
                    indegrees.insert(node_id, node.inputs().len() as u32);
                    for input in node.inputs() {
                        for (_cable_id, cable) in input
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
        }

        let mut node_id_to_vec_index_map = IdMap::new();

        let mut old_node_inners = IdMap::new();
        for graph_entry in self.nodes.drain(..) {
            old_node_inners.insert(graph_entry.node_id, graph_entry.inner);
        }

        while let Some(node_id) = zero_indegree_node_stack.pop() {
            let node = patch.node_entry(node_id).expect("unreachable");

            node_id_to_vec_index_map.insert(node_id, self.nodes.len() as u32);
            self.nodes.push(NodeGraphEntry {
                inner: old_node_inners
                    .remove(node_id)
                    .unwrap_or_else(|| options.registry.create_node(node.data.key_id)),
                inputs: node
                    .inputs()
                    .iter()
                    .map(|input| NodeGraphInput {
                        connections: {
                            input
                                .connections
                                .iter()
                                .map(|&cable_id| {
                                    let cable = patch
                                        .cable(cable_id)
                                        .expect("node connected to nonexistent cable");
                                    (
                                        *node_id_to_vec_index_map.force_get(cable.input_node),
                                        cable.input_output_index,
                                        cable.output_multiplier_fac,
                                    )
                                })
                                .collect()
                        },
                        bias: input.bias,
                        buffer: vec![0.0; options.buffer_size as usize].into_boxed_slice(),
                    })
                    .collect(),
                outputs: node
                    .outputs()
                    .iter()
                    .map(|output| {
                        if output.connections.is_empty() {
                            None
                        } else {
                            Some(
                                vec![Cell::new(0.0); options.buffer_size as usize]
                                    .into_boxed_slice(),
                            )
                        }
                    })
                    .collect(),

                node_id,
                state: node.data.inner.clone(),
            });
            // decrement outdegrees
            for output in node.outputs() {
                for (_cable_id, cable) in output
                    .get_connections(patch)
                    .filter(|(_, cable)| cable.tag.is_valid())
                {
                    let indegree = indegrees
                        .get_mut(cable.output_node)
                        .expect("cable connected to invalid node");
                    *indegree -= 1;
                    if *indegree == 0 {
                        zero_indegree_node_stack.push(cable.output_node);
                        indegrees.remove(cable.output_node);
                    }
                }
            }
        }
        assert!(indegrees.is_empty(), "cycle detected in node graph");
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

    pub fn process(&mut self, worker_options: &WorkerOptions) {
        // self.nodes has been topologically sorted so the all dependencies of a node appear before it in the vec
        for index in 0..self.nodes.len() {
            let (previous_nodes, [node, ..]) = self.nodes.split_at_mut(index) else {
                unreachable!()
            };
            let mut inputs_vec = Vec::with_capacity(node.inputs.len());

            struct CubedawNodeContext<'a> {
                worker_options: &'a WorkerOptions,
                previous_nodes: &'a [NodeGraphEntry],
                inputs: &'a [cubedaw_lib::DataSource<'a>],
                outputs: &'a mut [Option<Box<[Cell<f32>]>>],
            }

            impl<'a> cubedaw_lib::NodeContext<'a> for CubedawNodeContext<'a> {
                fn sample_rate(&self) -> u32 {
                    self.worker_options.sample_rate
                }
                fn buffer_size(&self) -> u32 {
                    self.worker_options.buffer_size
                }

                fn input(&self, index: u32) -> cubedaw_lib::DataSource<'_> {
                    self.inputs[index as usize]
                }
                fn output(&self, index: u32) -> cubedaw_lib::DataDrain<'_> {
                    match self.outputs[index as usize] {
                        None => cubedaw_lib::DataDrain::Disconnected,
                        Some(ref buf) => cubedaw_lib::DataDrain::NodeInput(buf),
                    }
                }
                fn property(&self, property: cubedaw_lib::NoteProperty) -> f32 {
                    match property {
                        cubedaw_lib::NoteProperty::PITCH => 1.0,
                        _ => 0.0,
                    }
                }
            }

            for input in &mut node.inputs {
                if input.connections.is_empty() {
                    inputs_vec.push(cubedaw_lib::DataSource::Const(input.bias));
                } else {
                    input.buffer.fill(0.0);
                    for &(connection, output_index, multiplier) in &input.connections {
                        let connected_node = &previous_nodes[connection as usize];
                        let zipped = connected_node.outputs[output_index as usize]
                            .as_ref()
                            .expect("???")
                            .iter()
                            .zip(&mut input.buffer);

                        match multiplier {
                            0.0 => (),
                            1.0 => {
                                for (conn_val, buf_val) in zipped {
                                    *buf_val += conn_val.get();
                                }
                            }
                            -1.0 => {
                                for (conn_val, buf_val) in zipped {
                                    *buf_val -= conn_val.get();
                                }
                            }
                            multiplier => {
                                for (conn_val, buf_val) in zipped {
                                    *buf_val += conn_val.get() * multiplier;
                                }
                            }
                        }
                    }

                    inputs_vec.push(cubedaw_lib::DataSource::Buffer(&input.buffer))
                }
            }

            node.inner.process(
                &*node.state,
                &mut CubedawNodeContext {
                    worker_options,
                    previous_nodes,
                    inputs: &inputs_vec,
                    outputs: &mut node.outputs,
                },
            );
        }
    }
}

#[derive(Clone)]
pub struct NodeGraphEntry {
    pub inner: DynNode,
    pub inputs: Vec<NodeGraphInput>,
    pub outputs: Vec<Option<Box<[Cell<f32>]>>>,

    pub node_id: Id<NodeEntry>,
    pub state: DynNodeState,
}
impl std::fmt::Debug for NodeGraphEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeGraphEntry")
            .field("inner", &self.inner)
            .field("inputs", &self.inputs)
            .field("outputs", &format_args!("<{} outputs>", self.outputs.len()))
            .field("node_id", &self.node_id)
            .field("state", &self.state)
            .finish()
    }
}

#[derive(Clone)]
pub struct NodeGraphInput {
    connections: Vec<(u32, u32, f32)>,
    bias: f32,
    buffer: Box<[f32]>,
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
