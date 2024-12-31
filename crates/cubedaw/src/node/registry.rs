use std::sync::Arc;

use ahash::{HashMap, HashMapExt};
use anyhow::Result;
use cubedaw_lib::{Buffer, ResourceKey};
use cubedaw_worker::DynNodeFactory;

use crate::node::{NodeCreationContext, NodeUiContext};

// TODO get a better name
pub trait NodeThingy: 'static + Send + Sync {
    fn create(&self, ctx: &NodeCreationContext) -> Box<Buffer>;
    fn title(&self, state: &Buffer) -> Result<std::borrow::Cow<'_, str>>;
    fn ui(&self, state: &mut Buffer, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext) -> Result<()>;

    fn make_nodefactory(&self) -> DynNodeFactory {
        DynNodeFactory(Box::new(|_| Box::new([])))
    }
}
impl std::fmt::Debug for dyn NodeThingy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("dyn NodeThingy").finish_non_exhaustive()
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

    pub fn register_node(&mut self, key: ResourceKey, name: &str, node_thingy: impl NodeThingy) {
        self.register_node_dyn(key, name.into(), Box::new(node_thingy));
    }
    pub fn register_node_dyn(
        &mut self,
        key: ResourceKey,
        name: &str,
        node_thingy: Box<dyn NodeThingy>,
    ) {
        self.dyn_node_factories
            .insert(key.clone(), node_thingy.make_nodefactory());
        self.register_node_no_inner(key, name, node_thingy);
    }
    pub(super) fn register_node_no_inner(
        &mut self,
        key: ResourceKey,
        name: &str,
        node_thingy: Box<dyn NodeThingy>,
    ) {
        self.entries.insert(key.clone(), NodeRegistryEntry {
            key: key.clone(),
            node_thingy,
        });
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
