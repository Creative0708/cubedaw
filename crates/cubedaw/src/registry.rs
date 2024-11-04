use std::{any::TypeId, ops, sync::Arc};

use ahash::{HashMap, HashMapExt};
use anyhow::Result;
use cubedaw_lib::{Id, IdMap, NodeData, ResourceKey};

use crate::node::{NodeCreationContext, NodeInputUiOptions, NodeUiContext};

// pub struct DynNodeThingy(pub Box<dyn Send + Sync + Fn(&NodeCreationContext) -> Box<[u8]>>);
// impl ops::Deref for DynNodeThingy {
//     type Target = dyn Send + Sync + Fn(&NodeCreationContext) -> Box<[u8]>;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
// impl ops::DerefMut for DynNodeThingy {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }
// impl std::fmt::Debug for DynNodeThingy {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "DynNodeFactory {{ <{:?}> }}", self as *const _)
//     }
// }

// TODO get a better name
pub trait NodeThingy: 'static + Send + Sync {
    fn create(&self, ctx: &NodeCreationContext) -> Box<[u8]>;
    fn title(&self, state: &[u8]) -> Result<std::borrow::Cow<'_, str>>;
    fn ui(&self, state: &mut [u8], ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext) -> Result<()>;
}
impl std::fmt::Debug for dyn NodeThingy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("dyn NodeThingy").finish_non_exhaustive()
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
    inner: Arc<cubedaw_worker::NodeRegistry>,

    entries: HashMap<ResourceKey, NodeRegistryEntry>,
    name_entries: Vec<NameEntry>,
}

const fn _assert_send_sync<T: Send + Sync>() {}
const _: () = _assert_send_sync::<NodeRegistry>();

impl NodeRegistry {
    pub fn new() -> Self {
        let mut this = Self {
            inner: Default::default(),
            entries: HashMap::new(),
            name_entries: Vec::new(),
        };

        struct TrackInputNodeThingy;
        impl NodeThingy for TrackInputNodeThingy {
            fn create(&self, _creation_context: &NodeCreationContext) -> Box<[u8]> {
                Box::new([])
            }
            fn title(&self, _: &[u8]) -> Result<std::borrow::Cow<'_, str>> {
                Ok("Track Input".into())
            }
            fn ui(
                &self,
                _: &mut [u8],
                ui: &mut egui::Ui,
                node_ui: &mut dyn NodeUiContext,
            ) -> Result<()> {
                node_ui.output_ui(ui, "Track Input");
                Ok(())
            }
        }
        this.register_node_dyn(
            resourcekey::literal!("builtin:track_input"),
            "Track Input".into(),
            Box::new(TrackInputNodeThingy),
        );
        struct TrackOutputNodeThingy;
        impl NodeThingy for TrackOutputNodeThingy {
            fn create(&self, _creation_context: &NodeCreationContext) -> Box<[u8]> {
                Box::new([])
            }
            fn title(&self, _: &[u8]) -> Result<std::borrow::Cow<'_, str>> {
                Ok("Track Output".into())
            }
            fn ui(
                &self,
                _: &mut [u8],
                ui: &mut egui::Ui,
                node_ui: &mut dyn NodeUiContext,
            ) -> Result<()> {
                node_ui.input_ui(ui, "Track Output", NodeInputUiOptions::uninteractable());
                Ok(())
            }
        }
        this.register_node_dyn(
            resourcekey::literal!("builtin:track_output"),
            "Track Output".into(),
            Box::new(TrackOutputNodeThingy),
        );
        struct NoteInputNodeThingy;
        impl NodeThingy for NoteInputNodeThingy {
            fn create(&self, _creation_context: &NodeCreationContext) -> Box<[u8]> {
                Box::new([])
            }
            fn title(&self, _: &[u8]) -> Result<std::borrow::Cow<'_, str>> {
                Ok("Note Input".into())
            }
            fn ui(
                &self,
                _: &mut [u8],
                ui: &mut egui::Ui,
                node_ui: &mut dyn NodeUiContext,
            ) -> Result<()> {
                node_ui.input_ui(ui, "Note Output", NodeInputUiOptions::uninteractable());
                node_ui.output_ui(ui, "Track Input");
                Ok(())
            }
        }
        this.register_node_dyn(
            resourcekey::literal!("builtin:note_output"),
            "Note Output".into(),
            Box::new(NoteInputNodeThingy),
        );
        this
    }

    pub fn register_node(&mut self, key: ResourceKey, name: &str, node_thingy: impl NodeThingy) {
        self.register_node_dyn(key, name.into(), Box::new(node_thingy));
    }
    pub fn register_node_dyn(
        &mut self,
        key: ResourceKey,
        name: Box<str>,
        node_thingy: Box<dyn NodeThingy>,
    ) {
        self.entries.insert(
            key.clone(),
            NodeRegistryEntry {
                key: key.clone(),
                node_thingy,
            },
        );
        self.name_entries.push(NameEntry {
            name,
            node_key: key,
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

    pub fn name_entries(&self) -> &[NameEntry] {
        &self.name_entries
    }

    pub fn get(&self, key: &ResourceKey) -> Option<&NodeRegistryEntry> {
        self.entries.get(key)
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
    pub node_thingy: Box<dyn NodeThingy>,
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
