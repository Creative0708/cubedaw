use std::any::TypeId;

use ahash::HashMap;
use cubedaw_lib::{Id, ResourceKey};
use cubedaw_node::{DynNode, Node};

pub mod math;
pub mod output;
mod util;

pub type DynNodeFactory = Box<dyn Fn(NodeCreationContext) -> DynNode>;

#[derive(Default)]
pub struct NodeRegistry {
    pub type_id_to_resource_key: HashMap<TypeId, Id<ResourceKey>>,
    pub keys: HashMap<Id<ResourceKey>, ResourceKey>,
    pub names: HashMap<Id<ResourceKey>, String>,
    pub factories: HashMap<Id<ResourceKey>, DynNodeFactory>,
}

impl NodeRegistry {
    pub fn register_node<N: Node>(
        &mut self,
        key: ResourceKey,
        name: String,
        factory: impl Fn(NodeCreationContext) -> N + 'static,
    ) {
        let key_id = key.id();
        self.type_id_to_resource_key
            .insert(TypeId::of::<N::State>(), key_id);
        self.names.insert(key_id, name);
        self.keys.insert(key_id, key);
        self.factories
            .insert(key_id, Box::new(move |ctx| Box::new(factory(ctx))));
    }
}

pub struct NodeCreationContext {
    pub alias: Option<Box<str>>,
}

pub fn register_cubedaw_nodes(registry: &mut NodeRegistry) {
    registry.register_node(
        ResourceKey::new("cubedaw:math".into()),
        "Math".into(),
        self::math::MathNode::create,
    );
}
