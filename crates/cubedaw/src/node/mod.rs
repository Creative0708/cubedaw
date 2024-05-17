use ahash::HashMap;
use cubedaw_lib::{Id, ResourceKey};
use cubedaw_worker::patch::DynNode;

pub mod math;
pub mod output;
mod util;

pub type DynNodeFactory = Box<dyn Fn(NodeCreationContext) -> DynNode>;

#[derive(Default)]
pub struct NodeRegistry {
    pub keys: HashMap<Id<ResourceKey>, ResourceKey>,
    pub names: HashMap<Id<ResourceKey>, String>,
    pub factories: HashMap<Id<ResourceKey>, DynNodeFactory>,
}

impl NodeRegistry {
    pub fn register_node(&mut self, key: ResourceKey, name: String, factory: DynNodeFactory) {
        let key_id = Id::new(&key);
        self.factories.insert(key_id, factory);
        self.names.insert(key_id, name);
        self.keys.insert(key_id, key);
    }
}

pub struct NodeCreationContext<'a> {
    pub alias: Option<&'a str>,
}
