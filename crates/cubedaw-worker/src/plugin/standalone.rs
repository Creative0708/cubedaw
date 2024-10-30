use ahash::HashMap;
use anyhow::{Context as _, Result};
use cubedaw_plugin::ValType;
use cubedaw_wasm::{Memory, Value};
use resourcekey::ResourceKey;
use unwrap_todo::UnwrapTodo;

use crate::WorkerOptions;

use super::PLUGIN_ALIGN;

#[derive(Debug)]
// dear god the oop is leaking
pub struct StandalonePluginFactory {
    module: cubedaw_wasm::Module,
    memory_location: cubedaw_wasm::ExportLocation,
    exported_nodes: HashMap<ResourceKey, cubedaw_wasm::ExportLocation>,
}

impl StandalonePluginFactory {
    pub fn new(plugin: &cubedaw_plugin::Plugin, options: &WorkerOptions) -> Self {
        let WorkerOptions { sample_rate, .. } = *options;
        let module_stitch =
            cubedaw_plugin::ModuleStitch::new(cubedaw_plugin::ShimInfo::new(move |ctx| {
                use cubedaw_plugin::{CubedawPluginImport as I, Instruction};
                match ctx.import() {
                    I::SampleRate => {
                        let prev_instruction = ctx.prev_instruction.clone();
                        ctx.replace([prev_instruction, Instruction::I32Const(sample_rate as i32)]);
                    }
                    _ => todo!(),
                }
            }));
        for node in plugin.exported_nodes() {
            let func_stitch = cubedaw_plugin::FunctionStitch::new([ValType::I32, ValType::I32], []);
            // TODO
            // plugin.stitch_node(&mut func_stitch, &mut module_stitch);
        }
        let wasm_module = module_stitch.finish();
        let module = cubedaw_wasm::Module::new(options.registry.engine(), &wasm_module).todo();

        // let (mut store, instance) = Self::instantiate(&module, options)?;
        Self {
            memory_location: module
                .get_export("memory")
                .expect("plugin has no exported memory???"),
            exported_nodes: plugin
                .exported_nodes()
                .map(|key| {
                    (
                        key.clone(),
                        module
                            .get_export(key.item_str())
                            .expect("exported node isn't exported???"),
                    )
                })
                .collect(),
            // exported_nodes: plugin
            //     .exported_nodes()
            //     .map(|key| {
            //         Ok((
            //             key.clone(),
            //             instance
            //                 .get_func(
            //                     &mut store,
            //                     &module.get_export(key.item_str()).with_context(|| {
            //                         format!("function {key} doesn't exist in plugin")
            //                     })?,
            //                 )
            //                 .expect("unreachable"),
            //         ))
            //     })
            //     .collect::<Result<_>>()?,
            // store,
            // instance,
            module,
        }
    }

    pub fn create(&self, options: &WorkerOptions) -> StandalonePlugin {
        let mut store =
            StandalonePluginStore::new(options.registry.engine(), StandalonePluginParameters {});
        let mut instance = options
            .registry
            .standalone_linker()
            .instantiate(&mut store, &self.module)
            .expect("failed to instantiate module");
        let mut memory = instance
            .get_memory(&mut store, &self.memory_location)
            .expect("no memory in module or not exported");

        StandalonePlugin {
            byte_start: memory.size(&store),
            exported_nodes: self
                .exported_nodes
                .iter()
                .map(|(key, export_location)| {
                    (
                        key.clone(),
                        instance
                            .get_func(&mut store, export_location)
                            .expect("????????"),
                    )
                })
                .collect(),

            store,
            instance,
            memory,
        }
    }
}

#[derive(Debug)]
pub struct StandalonePlugin {
    store: StandalonePluginStore,
    instance: cubedaw_wasm::Instance,
    memory: cubedaw_wasm::Memory,
    exported_nodes: HashMap<ResourceKey, cubedaw_wasm::Func>,

    /// Start of overwritable memory. Aligned to `PLUGIN_ALIGN`
    byte_start: u32,
}

const fn ceil_to_multiple_of_power_of_2(n: u32, logm: u32) -> u32 {
    let mask = (1 << logm) - 1;
    (n + mask) & !mask
}

impl StandalonePlugin {
    fn instantiate(
        module: &cubedaw_wasm::Module,
        options: &WorkerOptions,
    ) -> Result<(StandalonePluginStore, cubedaw_wasm::Instance)> {
        let mut store = StandalonePluginStore::new(options.registry.engine(), Default::default());
        let instance = options
            .registry
            .standalone_linker()
            .instantiate(&mut store, module)?;
        Ok((store, instance))
    }

    pub fn run(&mut self, key: &ResourceKey, args: &[u8], state: &mut [u8]) -> anyhow::Result<()> {
        assert!(
            args.len() <= u32::MAX as usize,
            "args won't fit in 32-bit assembly"
        );
        assert!(
            state.len() <= u32::MAX as usize,
            "state won't fit in 32-bit assembly"
        );
        assert!(
            (args.len() as u64).saturating_add(state.len() as u64) <= u32::MAX as u64,
            "args & state won't fit in 32-bit assembly"
        );
        let Some(func) = self.exported_nodes.get(key) else {
            panic!("key {key:?} doesn't exist in plugin {self:?}");
        };

        let args_size = ceil_to_multiple_of_power_of_2(args.len() as u32, PLUGIN_ALIGN.ilog2());
        let args_start = self.byte_start;
        let state_size = ceil_to_multiple_of_power_of_2(state.len() as u32, PLUGIN_ALIGN.ilog2());
        let state_start = self.byte_start + args_size;

        let page_size = self.memory.page_size_log2(&self.store);
        let required_size = self.byte_start + args_size + state_size;
        let current_size = self.memory.size(&self.store);
        if current_size < required_size {
            self.memory
                .grow(
                    &mut self.store,
                    ceil_to_multiple_of_power_of_2(required_size - current_size, page_size)
                        >> page_size,
                )
                .map_err(|_| anyhow::anyhow!("plugin memory limit exceeded"))?;
        }
        // write() only returns Err when the indices are out of range so these expect()s are unreachable
        self.memory
            .write(&mut self.store, args_start, args)
            .expect("unreachable");
        self.memory
            .write(&mut self.store, state_start, state)
            .expect("unreachable");

        func.call(
            &mut self.store,
            &[
                Value::I32(args_start as i32),
                Value::I32(state_start as i32),
            ],
            &mut [],
        );

        Ok(())
    }

    // fn clone(&self, options: &WorkerOptions) -> Result<Self> {
    //     // this doesn't create an _exact_ exact copy of this object
    //     // but since plugins aren't supposed to use global state anyways
    //     // this is probably fine
    //     let (mut store, instance) = Self::instantiate(&self.module, options)?;
    //     Ok(Self {
    //         memory: instance.get_memory(&mut store, &self.memory_location),

    //         store,
    //         module: self.module.clone(),
    //         instance,
    //         exported_function: self.exported_function,
    //         byte_start: self.byte_start,
    //     })
    // }
}

/// Typed store for cubedaw plugins.
pub type StandalonePluginStore = cubedaw_wasm::Store<StandalonePluginParameters>;

/// Parameters for `StandalonePluginStore`.
#[derive(Debug, Default)]
pub struct StandalonePluginParameters {
    // TODO
}
