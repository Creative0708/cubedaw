use std::{cell::RefCell, sync::Arc};

use ahash::{HashMap, HashMapExt};
use cubedaw_lib::{Buffer, State};
use cubedaw_wasm::Engine;
use resourcekey::ResourceKey;

use crate::{
    common::{HostToWorkerEvent, WorkerToHostEvent},
    plugin::standalone::{StandalonePlugin, StandalonePluginFactory, StandalonePluginParameters},
    registry::NodeRegistry,
    WorkerJob, WorkerState,
};

pub fn run_forever(
    tx: crossbeam_channel::Sender<WorkerToHostEvent>,
    rx: crossbeam_channel::Receiver<HostToWorkerEvent>,

    work_tx: crossbeam_channel::Sender<WorkerJob>,
    work_rx: crossbeam_channel::Receiver<WorkerJob>,

    options: &WorkerOptions,
) {
    let mut scratch = WorkerScratch::new(options);
    let mut worker_state = WorkerState::new(options);

    while let Ok(event) = rx.recv() {
        match event {
            HostToWorkerEvent::StartProcessing { state, start_pos } => {
                loop {
                    match work_rx.recv() {
                        Ok(WorkerJob::Finalize) => break,
                        Ok(job) => {
                            let result =
                                job.process(state, &options, &mut worker_state, &mut scratch);

                            if let Some(job_descriptor) = result.finished_job_descriptor {
                                tx.send(WorkerToHostEvent::FinishJob(job_descriptor))
                                    .expect("channel closed during processing");
                            }
                            if let Some(job_to_add) = result.job_to_add {
                                match job_to_add {
                                    WorkerJob::Finalize => {
                                        // repeat the finalization signal to all workers
                                        for _ in 0..options.num_workers {
                                            work_tx.send(WorkerJob::Finalize).unwrap();
                                        }
                                    }
                                    job_to_add => {
                                        work_tx.send(job_to_add).unwrap();
                                    }
                                }
                            }
                        }
                        Err(crossbeam_channel::RecvError) => {
                            panic!("channel closed during processing");
                        }
                    }
                }
                // Note: as per the documentation of `WorkerToHostEvent::Idle`, workers must send exactly 1 of this event
                // and must have dropped all references given from work_rx. Not doing this will cause UB.
                // Just, like, don't be stupid with how the worker is implemented.
                if tx.send(WorkerToHostEvent::Idle).is_err() {
                    break;
                }
            }
        }
    }
}

pub fn process_job(job: &WorkerJob, state: &State, options: &WorkerOptions) -> Box<[f32]> {
    todo!()
}

#[derive(Debug)]
/// Static worker options.
///
/// This is shared across all workers and is read-only. This is changed when the user changes the worker options (duh).
pub struct WorkerOptions {
    pub registry: NodeRegistry,
    pub standalone_plugin_factories: HashMap<ResourceKey, Arc<StandalonePluginFactory>>,

    pub num_workers: u32,

    pub sample_rate: u32,
    pub buffer_size: u32,
}

impl WorkerOptions {
    pub fn new(registry: NodeRegistry) -> Self {
        let mut this = Self {
            standalone_plugin_factories: Default::default(),

            // num_workers: std::thread::available_parallelism()
            //     .map(std::num::NonZero::get)
            //     .unwrap_or(1)
            //     .try_into()
            //     .unwrap_or(u32::MAX), // just to be safe
            num_workers: 1, // TODO remove

            sample_rate: 44100,
            buffer_size: 256,

            registry,
        };

        let mut standalone_plugin_factories = HashMap::new();
        let mut arc_ptr_to_standalone_plugin_factory = HashMap::new();
        for (key, entry) in this.registry.entries() {
            let Some(ref plugin_data) = entry.plugin_data else {
                continue;
            };
            let factory = arc_ptr_to_standalone_plugin_factory
                .entry(Arc::as_ptr(plugin_data))
                .or_insert_with(|| {
                    Arc::new(StandalonePluginFactory::new(&plugin_data.plugin, &this))
                });
            standalone_plugin_factories.insert(key.clone(), factory.clone());
        }

        this.standalone_plugin_factories = standalone_plugin_factories;

        // let mut instances = HashMap::new();
        // for (key, entry) in registry.entries() {
        //     let Some(ref plugin_data) = entry.plugin_data else {
        //         continue;
        //     };
        //     instances.insert(key.clone(), plugin_data.standalone_factory.create(&this));
        // }

        this
    }
}

// TODO remove
impl Default for WorkerOptions {
    fn default() -> Self {
        Self::new(NodeRegistry::new(Default::default()))
    }
}

// two oughta be enough for everyone
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct WorkerScratch(pub Box<Buffer>, pub Box<Buffer>);

impl WorkerScratch {
    pub fn new(options: &WorkerOptions) -> Self {
        Self(
            Buffer::new_box_zeroed(options.buffer_size),
            Buffer::new_box_zeroed(options.buffer_size),
        )
    }
}
impl Default for WorkerScratch {
    fn default() -> Self {
        Self(Buffer::new_box_zeroed(0), Buffer::new_box_zeroed(0))
    }
}
