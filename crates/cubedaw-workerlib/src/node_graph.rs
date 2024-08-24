use ahash::HashSetExt;
use cubedaw_lib::{Buffer, DynNode, Id, IdMap, IdSet, NodeData, Patch};

use crate::WorkerOptions;

#[derive(Clone, Debug, Default)]
pub struct ProcessedNodeGraph {
    id_to_index: IdMap<NodeData, u32>,
    nodes: Vec<NodeGraphEntry>,
}

impl ProcessedNodeGraph {
    // this could be optimized to incrementally update the nodes when the graph is changed
    // but an O(n) change is fine because rendering is also O(n) and the performance impact is (probably) negligible.
    // if/when rendering gets optimized to use a texture or whatever and this becomes a bottleneck maybe we can optimize this.
    pub fn new(
        patch: &Patch,
        options: &WorkerOptions,
        input_node: Option<Id<NodeData>>,
        output_node: Id<NodeData>,
    ) -> Self {
        // simple topo sort algo. TODO possibly replace with a faster one?

        let mut indegrees: IdMap<NodeData, u32> = IdMap::new();

        let mut zero_indegree_node_stack = Vec::new();

        // dfs nodes between the input and output nodes
        {
            let mut stack = vec![output_node];
            let mut visited = IdSet::new();
            while let Some(node_id) = stack.pop() {
                let node = patch
                    .node(node_id)
                    .expect("cable connected to nonexistent node???");

                if node.inputs.is_empty() || Some(node_id) == input_node {
                    zero_indegree_node_stack.push(node_id);
                } else {
                    indegrees.insert(node_id, node.inputs.len() as u32);
                    for input in &node.inputs {
                        for (_cable_id, cable) in input
                            .get_connections(patch)
                            .filter(|(_, cable)| cable.tag.is_valid())
                        {
                            let new_node_id = cable.output_node;
                            if visited.insert(new_node_id) {
                                stack.push(new_node_id);
                            }
                        }
                    }
                }
            }
        }

        let mut node_vec = Vec::new();
        let mut node_id_to_vec_index_map = IdMap::new();

        while let Some(node_id) = zero_indegree_node_stack.pop() {
            let node = patch.node(node_id).expect("unreachable");

            node_id_to_vec_index_map.insert(node_id, node_vec.len() as u32);
            node_vec.push(NodeGraphEntry {
                inner: options.registry.create_node(node.key),
                inputs: node
                    .inputs
                    .iter()
                    .map(|input| {
                        input
                            .connections
                            .iter()
                            .map(|&cable_id| {
                                let cable = patch
                                    .cable(cable_id)
                                    .expect("node connected to nonexistent cable");
                                (
                                    *node_id_to_vec_index_map.force_get(cable.input_node),
                                    cable.output_multiplier_fac,
                                )
                            })
                            .collect()
                    })
                    .collect(),
                node_id,
                output_buffer: Buffer::new(options.buffer_size),
            })
        }

        debug_assert!(node_vec.len() <= u32::MAX as usize);

        Self {
            nodes: node_vec,
            id_to_index: node_id_to_vec_index_map,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeGraphEntry {
    pub inner: DynNode,
    pub inputs: Vec<Vec<(u32, f32)>>,
    pub node_id: Id<NodeData>,
    pub output_buffer: Buffer,
}
