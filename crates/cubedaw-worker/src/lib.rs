#![feature(coroutines)]
#![feature(gen_blocks)]
#![feature(non_null_from_ref)]

mod common;
pub mod host;
mod plugin;
mod registry;
pub use registry::{DynNodeFactory, NodeRegistry, NodeRegistryEntry, PluginData};
mod worker;
pub use host::WorkerHost;
pub use worker::WorkerOptions;
mod state;
pub(crate) use state::WorkerState;

pub mod sync;

mod util;

mod job;
pub(crate) use job::{NoteDescriptor, WorkerJob};

mod node_graph;
