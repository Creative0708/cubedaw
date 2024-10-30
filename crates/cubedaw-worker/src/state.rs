use std::{cell::RefCell, rc::Rc, sync::Arc};

use ahash::{HashMap, HashMapExt};
use resourcekey::ResourceKey;

use crate::{plugin::standalone::StandalonePlugin, WorkerOptions};

/// Per-worker state.
pub struct WorkerState {
    pub standalone_instances: HashMap<ResourceKey, Rc<RefCell<StandalonePlugin>>>,
}

impl WorkerState {
    pub fn new(options: &WorkerOptions) -> Self {
        let mut standalone_instances = HashMap::new();
        let mut arc_ptr_to_standalone_plugin_instance = HashMap::new();
        for (key, factory) in options.standalone_plugin_factories.iter() {
            let instance = arc_ptr_to_standalone_plugin_instance
                .entry(Arc::as_ptr(factory))
                .or_insert_with(|| Rc::new(RefCell::new(factory.create(options))));
            standalone_instances.insert(key.clone(), instance.clone());
        }

        Self {
            standalone_instances,
        }
    }
}
