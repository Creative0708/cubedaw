use cubedaw_lib::NodeRegistry;

#[derive(Debug, Default)]
pub struct NodeSearch {
    pub inner: String,
}

impl NodeSearch {
    pub fn search(&mut self, registry: &NodeRegistry) {}
}
