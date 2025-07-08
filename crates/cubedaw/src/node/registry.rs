use std::sync::Arc;

use ahash::{HashMap, HashMapExt};
use anyhow::Result;
use cubedaw_lib::{Buffer, ResourceKey};
use cubedaw_worker::DynNodeFactory;

use crate::{
    Context,
    node::{NodeCreationContext, NodeUiContext},
};

/// This trait represents an instance of a node, as known by the UI of the app.
/// It is responsible
pub trait NodeUi: 'static + Send + Sync {
    fn create(&self, ctx: &NodeCreationContext) -> Box<Buffer>;
    fn title(&self, state: &Buffer, ctx: &Context) -> Result<std::borrow::Cow<'_, str>>;
    fn ui(&self, state: &mut Buffer, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext) -> Result<()>;

    fn make_node_factory(&self) -> DynNodeFactory {
        DynNodeFactory(Box::new(|_| Box::new([])))
    }
}
impl std::fmt::Debug for dyn NodeUi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("dyn NodeUi").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct NodeRegistry {
    pub(super) inner: Arc<cubedaw_worker::NodeRegistry>,
    // used for cubedaw_worker::NodeRegistry
    pub(super) dyn_node_factories: HashMap<ResourceKey, cubedaw_worker::DynNodeFactory>,

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
            dyn_node_factories: HashMap::new(),
        };

        crate::node::register_builtin_nodes(&mut this);

        this
    }

    pub fn inner(&self) -> &Arc<cubedaw_worker::NodeRegistry> {
        &self.inner
    }

    pub fn register_node(&mut self, key: ResourceKey, name: &str, node_thingy: impl NodeUi) {
        self.register_node_dyn(key, name, Box::new(node_thingy));
    }
    pub fn register_node_dyn(
        &mut self,
        key: ResourceKey,
        name: &str,
        node_thingy: Box<dyn NodeUi>,
    ) {
        self.dyn_node_factories
            .insert(key.clone(), node_thingy.make_node_factory());
        self.register_node_without_factory(key, name, node_thingy);
    }
    pub(super) fn register_node_without_factory(
        &mut self,
        key: ResourceKey,
        name: &str,
        ui: Box<dyn NodeUi>,
    ) {
        self.entries.insert(
            key.clone(),
            NodeRegistryEntry {
                key: key.clone(),
                ui,
            },
        );
        self.name_entries.push(NameEntry {
            name: name.into(),
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
    pub ui: Box<dyn NodeUi>,
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
