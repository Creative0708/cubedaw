use std::any::TypeId;

use ahash::{HashMap, HashMapExt};
use cubedaw_lib::{DynNodeState, Id, IdMap, ResourceKey};
use cubedaw_node::{DynNode, Node, NodeCreationContext};

use crate::nodes;

pub type DynNodeFactory = Box<dyn Send + Sync + Fn() -> DynNode>;

pub type NodeStateFactory = Box<dyn Send + Sync + Fn(NodeCreationContext) -> DynNodeState>;

pub struct NodeRegistry {
    type_id_to_resource_key: HashMap<TypeId, Id<ResourceKey>>,
    entries: IdMap<ResourceKey, NodeRegistryEntry>,
    name_entries: Vec<NameEntry>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        let mut this = Self {
            type_id_to_resource_key: HashMap::new(),
            entries: IdMap::new(),
            name_entries: Vec::new(),
        };
        this.register_node::<nodes::TrackOutputNode>(
            ResourceKey::new("builtin:track_output"),
            "Track Output".into(),
        );
        this.register_node::<nodes::NoteOutputNode>(
            ResourceKey::new("builtin:note_output"),
            "Note Output".into(),
        );
        this
    }

    pub fn register_node<N: Node>(&mut self, key: ResourceKey, name: Box<str>) {
        let key_id = key.id();
        self.type_id_to_resource_key
            .insert(TypeId::of::<N::State>(), key_id);
        self.entries.insert(
            key_id,
            NodeRegistryEntry {
                key,
                name: name.clone(),
                node_factory: Box::new(|| Box::new(N::new())),
                node_state_factory: Box::new(|ctx| Box::new(N::new_state(ctx))),
            },
        );
        self.name_entries.push(NameEntry {
            name,
            node_key: key_id,
            entry_type: NameEntryType::Name,
        });
    }
    pub fn register_alias(&mut self, node_key: Id<ResourceKey>, alias: Box<str>) {
        self.name_entries.push(NameEntry {
            name: alias,
            node_key,
            entry_type: NameEntryType::Alias,
        });
    }
    pub fn get_resource_key_of(&self, node: &dyn cubedaw_lib::NodeStateWrapper) -> Id<ResourceKey> {
        *self
            .type_id_to_resource_key
            .get(&node.type_id())
            .expect("node of unregistered type passed to get_resource_key_of")
    }

    pub fn create_node(&self, key_id: Id<ResourceKey>) -> DynNode {
        let Some(entry) = self.entries.get(key_id) else {
            panic!("invalid key id passed to create_node");
        };
        (entry.node_factory)()
    }
    pub fn create_state(
        &self,
        key_id: Id<ResourceKey>,
        creation_context: NodeCreationContext<'_>,
    ) -> DynNodeState {
        let Some(entry) = self.entries.get(key_id) else {
            panic!("invalid key id passed to create_state");
        };
        (entry.node_state_factory)(creation_context)
    }

    pub fn create_node_and_state(
        &self,
        key_id: Id<ResourceKey>,
        creation_context: NodeCreationContext<'_>,
    ) -> (DynNode, DynNodeState) {
        let Some(entry) = self.entries.get(key_id) else {
            panic!("invalid key id passed to create_state");
        };
        let state = (entry.node_state_factory)(creation_context);
        ((entry.node_factory)(), state)
    }

    pub fn name_entries(&self) -> &[NameEntry] {
        &self.name_entries
    }

    pub fn get(&self, key_id: Id<ResourceKey>) -> Option<&NodeRegistryEntry> {
        self.entries.get(key_id)
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NodeRegistryEntry {
    pub key: ResourceKey,
    pub name: Box<str>,
    pub node_factory: DynNodeFactory,
    pub node_state_factory: NodeStateFactory,
}

pub struct NameEntry {
    pub name: Box<str>,
    pub node_key: Id<ResourceKey>,
    pub entry_type: NameEntryType,
}

pub enum NameEntryType {
    Name,
    Alias,
}
