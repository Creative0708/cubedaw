use std::{any::TypeId, ops};

use ahash::{HashMap, HashMapExt};
use cubedaw_lib::{Id, IdMap, NodeData, ResourceKey};

pub struct DynNodeFactory(pub Box<dyn Send + Sync + Fn() -> Box<u8>>);
impl ops::Deref for DynNodeFactory {
    type Target = dyn Send + Sync + Fn() -> Box<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for DynNodeFactory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl std::fmt::Debug for DynNodeFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynNodeFactory {{ <{:?}> }}", self as *const _)
    }
}

// pub struct NodeStateFactory(pub Box<dyn Send + Sync + Fn(NodeCreationContext) -> Box<u8>>);
// impl ops::Deref for NodeStateFactory {
//     type Target = dyn Send + Sync + Fn(NodeCreationContext) -> DynNodeState;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
// impl ops::DerefMut for NodeStateFactory {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }
// impl std::fmt::Debug for NodeStateFactory {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "NodeStateFactory {{ <{:?}> }}", self as *const _)
//     }
// }

#[derive(Debug)]
pub struct NodeRegistry {
    type_id_to_resource_key: HashMap<TypeId, ResourceKey>,
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
        this.register_node::<nodes::TrackInputNode>(
            ResourceKey::new("builtin:track_input"),
            "Track Input".into(),
        );
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

    pub fn register_node(&mut self, key: ResourceKey) {
        let key_id = key.id();
        self.type_id_to_resource_key
            .insert(TypeId::of::<N::State>(), key_id);
        self.entries.insert(
            key_id,
            NodeRegistryEntry {
                key,
                node_factory: DynNodeFactory(Box::new(|| Box::new(N::new()))),
            },
        );
        self.name_entries.push(NameEntry {
            name,
            node_key: key_id,
            entry_type: NameEntryType::Name,
        });
    }
    pub fn register_alias(&mut self, node_key: ResourceKey, alias: Box<str>) {
        self.name_entries.push(NameEntry {
            name: alias,
            node_key,
            entry_type: NameEntryType::Alias,
        });
    }
    pub fn get_resource_key_of(&self, node: &dyn crate::NodeStateWrapper) -> ResourceKey {
        *self
            .type_id_to_resource_key
            .get(&node.type_id())
            .expect("node of unregistered type passed to get_resource_key_of")
    }

    pub fn create_node(&self, key_id: ResourceKey) -> DynNode {
        let Some(entry) = self.entries.get(key_id) else {
            panic!("invalid key id passed to create_node");
        };
        (entry.node_factory)()
    }
    pub fn create_state(
        &self,
        key_id: ResourceKey,
        creation_context: NodeCreationContext<'_>,
    ) -> DynNodeState {
        let Some(entry) = self.entries.get(key_id) else {
            panic!("invalid key id passed to create_state: {key_id:?}");
        };
        (entry.node_state_factory)(creation_context)
    }
    pub fn create_data(
        &self,
        key_id: ResourceKey,
        creation_context: NodeCreationContext<'_>,
    ) -> NodeData {
        NodeData {
            key_id,
            inner: self.create_state(key_id, creation_context),
        }
    }

    pub fn create_node_and_state(
        &self,
        key_id: ResourceKey,
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

    pub fn get(&self, key_id: ResourceKey) -> Option<&NodeRegistryEntry> {
        self.entries.get(key_id)
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct NodeRegistryEntry {
    pub key: ResourceKey,
    pub node_factory: DynNodeFactory,
}

#[derive(Debug)]
pub struct NameEntry {
    pub name: Box<str>,
    pub node_key: ResourceKey,
    pub entry_type: NameEntryType,
}

#[derive(Debug)]
pub enum NameEntryType {
    Name,
    Alias,
}
