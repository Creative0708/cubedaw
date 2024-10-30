use std::{any::TypeId, ops, sync::Arc};

use ahash::{HashMap, HashMapExt};
use cubedaw_lib::{Id, IdMap, NodeData, ResourceKey};
use cubedaw_plugin::Plugin;
use resourcekey::Namespace;
use unwrap_todo::UnwrapTodo;

use crate::plugin::standalone::{StandalonePluginFactory, StandalonePluginParameters};

pub struct DynNodeFactory(pub Box<dyn Send + Sync + Fn(&[u8]) -> Box<[u8]>>);
impl ops::Deref for DynNodeFactory {
    type Target = dyn Send + Sync + Fn(&[u8]) -> Box<[u8]>;
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

/// Global registry for all of the nodes. All the instantiation of WebAssembly shenanigans occur with this.
///
/// This is shared across all workers and is read-only. This is changed whenever a plugin is loaded/unloaded.
pub struct NodeRegistry {
    engine: cubedaw_wasm::Engine,
    entries: HashMap<ResourceKey, NodeRegistryEntry>,
    all_plugin_data: Vec<Arc<PluginData>>,
    standalone_linker: Arc<cubedaw_wasm::Linker<StandalonePluginParameters>>,
}

impl NodeRegistry {
    pub fn new(engine: cubedaw_wasm::Engine) -> Self {
        let mut this = Self {
            entries: HashMap::new(),
            all_plugin_data: Vec::new(),
            standalone_linker: Arc::new(cubedaw_wasm::Linker::new(&engine)),

            engine,
        };
        this.register_dummy_node(ResourceKey::new("builtin:track_input").unwrap());
        this.register_dummy_node(ResourceKey::new("builtin:track_output").unwrap());
        this.register_dummy_node(ResourceKey::new("builtin:note_output").unwrap());
        this
    }

    fn register_dummy_node(&mut self, key: ResourceKey) {
        self.entries.insert(
            key.clone(),
            NodeRegistryEntry {
                key,
                node_factory: DynNodeFactory(Box::new(|_| Box::new([]))),
                plugin_data: None,
            },
        );
    }

    // TODO: passing in dyn_node_factories as a parameter is a terrible hack
    // but also i'm not implementing a whole egui translation layer for the MVP
    // sooooooooooooooooooooooooooooooooooo
    pub fn register_plugin(
        &mut self,
        plugin: Plugin,
        mut dyn_node_factories: HashMap<ResourceKey, DynNodeFactory>,
    ) {
        let plugin_data = Arc::new(PluginData { plugin });
        for key in plugin_data.plugin.exported_nodes() {
            self.entries
                .insert(
                    key.clone(),
                    NodeRegistryEntry {
                        key: key.clone(),
                        node_factory: dyn_node_factories.remove(key).todo(),
                        plugin_data: Some(plugin_data.clone()),
                    },
                )
                .inspect(|entry| {
                    panic!("plugin key collision for {}", entry.key);
                });
        }
        self.all_plugin_data.push(plugin_data);
    }

    // pub fn create_node(&self, key_id: ResourceKey) -> Box<[u8]> {
    //     let Some(entry) = self.entries.get(&key_id) else {
    //         panic!("invalid key id passed to create_node");
    //     };
    //     (entry.node_factory)()
    // }

    pub fn get(&self, key: &ResourceKey) -> Option<&NodeRegistryEntry> {
        self.entries.get(key)
    }
    pub fn entries(&self) -> impl Iterator<Item = (&ResourceKey, &NodeRegistryEntry)> {
        self.entries.iter()
    }

    pub fn engine(&self) -> &cubedaw_wasm::Engine {
        &self.engine
    }
    pub fn standalone_linker(&self) -> &cubedaw_wasm::Linker<StandalonePluginParameters> {
        &self.standalone_linker
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl std::fmt::Debug for NodeRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRegistry")
            .field("nodes", &self.entries.keys())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct NodeRegistryEntry {
    pub key: ResourceKey,
    pub node_factory: DynNodeFactory,
    pub plugin_data: Option<Arc<PluginData>>,
}

#[derive(Debug)]
pub struct PluginData {
    pub plugin: cubedaw_plugin::Plugin,
}
