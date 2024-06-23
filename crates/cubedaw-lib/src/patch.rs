use crate::{Id, IdMap, NodeStateWrapper, ResourceKey};

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

    pub fn insert_cable(&mut self, cable_id: Id<Cable>, cable: Cable) {
        self.cables.insert(cable_id, cable);
    }
    pub fn take_cable(&mut self, cable_id: Id<Cable>) -> Cable {
        let cable = self.cables.take(cable_id);

        self.nodes
            .get_mut(cable.input_node)
            .expect("cable connected to nonexistent node?!?")
            .outputs
            .remove(cable.input_output_index);
        self.nodes
            .get_mut(cable.output_node)
            .expect("cable connected to nonexistent node?!?")
            .inputs
            .remove(cable.input_output_index);

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

#[derive(Debug, Clone)]
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
    pub input_output_index: usize,

    pub output_node: Id<NodeData>,
    pub output_input_index: usize,

    pub invalid: bool,
    pub output_multiplier_fac: f32,
}
impl Cable {
    pub fn assert_valid(&self, patch: &Patch) {
        let (input_node, output_node) = (
            patch.node(self.input_node).expect("nonexistent input node"),
            patch
                .node(self.output_node)
                .expect("nonexistent output node"),
        );

        assert_eq!(
            input_node.tag, output_node.tag,
            "node tags connected to the same cable should be equal"
        );
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTag {
    Disconnected,
    Note,
    Track,
}
