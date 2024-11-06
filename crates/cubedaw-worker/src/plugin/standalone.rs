use ahash::HashMap;
use anyhow::Result;
use cubedaw_lib::InternalBufferType;
use cubedaw_plugin::{CubedawPluginImport, Instruction};
use cubedaw_wasm::{ValType, Value};
use resourcekey::ResourceKey;
use unwrap_todo::UnwrapTodo;

use crate::WorkerOptions;

use super::PLUGIN_ALIGN;

#[derive(Debug)]
pub struct StandalonePluginFactory {
    module: cubedaw_wasm::Module,
    memory_location: cubedaw_wasm::ExportLocation,
    exported_nodes: HashMap<ResourceKey, cubedaw_wasm::ExportLocation>,
}

impl StandalonePluginFactory {
    pub fn new(plugin: &cubedaw_plugin::Plugin, options: &WorkerOptions) -> Result<Self> {
        let WorkerOptions { sample_rate, .. } = *options;
        let mut module = cubedaw_plugin::ModuleStitch::with_imports(
            cubedaw_plugin::ShimInfo::new(move |mut ctx| {
                use cubedaw_plugin::CubedawPluginImport;
                use cubedaw_plugin::Instruction;
                match ctx.import() {
                    CubedawPluginImport::SampleRate => {
                        ctx.replace_only_current([Instruction::I32Const(sample_rate as i32)]);
                    }
                    CubedawPluginImport::Input => {
                        ctx.replace_only_current([]);
                        ctx.add_instruction_raw(Instruction::Call(0));
                    }
                    CubedawPluginImport::Output => {
                        ctx.replace_only_current([]);
                        ctx.add_instruction_raw(Instruction::Call(1));
                    }
                }
            }),
            [
                ("input", CubedawPluginImport::Input.ty()),
                ("output", CubedawPluginImport::Output.ty()),
            ],
        );
        for node in plugin.exported_nodes() {
            let mut func = cubedaw_plugin::FunctionStitch::new(cubedaw_wasm::FuncType::new(
                [ValType::I32, ValType::I32],
                [],
            ));
            func.add_instruction_raw(&Instruction::LocalGet(0));
            func.add_instruction_raw(&Instruction::LocalGet(1));
            // TODO
            plugin.stitch_node(node, &mut func, &mut module)?;

            let func_idx = module.add_function(func);
            module.export_function(node.as_str(), func_idx);
        }

        module.export_memory("mem", 0);
        let wasm_module = module.finish();

        std::fs::write("/tmp/a.wasm", &wasm_module).unwrap();
        let module = cubedaw_wasm::Module::new(options.registry.engine(), &wasm_module).todo();

        Ok(Self {
            memory_location: module
                .get_export("mem")
                .expect("plugin has no exported memory despite us exporting it like 10 lines ago"),
            exported_nodes: plugin
                .exported_nodes()
                .map(|key| {
                    (
                        key.clone(),
                        module
                            .get_export(key.as_str())
                            .expect("exported node isn't exported???"),
                    )
                })
                .collect(),
            module,
        })
    }

    pub fn create(&self, options: &WorkerOptions) -> StandalonePlugin {
        let mut store = StandalonePluginStore::new(
            options.registry.engine(),
            StandalonePluginParameters::default(),
        );
        let instance = options
            .registry
            .standalone_linker()
            .instantiate(&mut store, &self.module)
            .expect("failed to instantiate module");
        let memory = instance
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
            _instance: instance,
            memory,
        }
    }
}

#[derive(Debug)]
pub struct StandalonePlugin {
    store: StandalonePluginStore,
    _instance: cubedaw_wasm::Instance,
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
        if !args.is_empty() {
            self.memory
                .write(&mut self.store, args_start, args)
                .expect("unreachable");
        }
        if !state.is_empty() {
            self.memory
                .write(&mut self.store, state_start, state)
                .expect("unreachable");
        }

        func.call(
            &mut self.store,
            &[
                Value::I32(args_start as i32),
                Value::I32(state_start as i32),
            ],
            &mut [],
        )?;

        if !state.is_empty() {
            self.memory
                .read(&self.store, state_start, state)
                .expect("unreachable");
        }

        Ok(())
    }

    pub fn store(&self) -> &StandalonePluginStore {
        &self.store
    }
    pub fn store_mut(&mut self) -> &mut StandalonePluginStore {
        &mut self.store
    }
}

/// Typed store for cubedaw plugins.
pub type StandalonePluginStore = cubedaw_wasm::Store<StandalonePluginParameters>;

/// Parameters for `StandalonePluginStore`.
#[derive(Debug, Default)]
pub struct StandalonePluginParameters {
    pub inputs: Vec<InternalBufferType>,
    pub outputs: Vec<InternalBufferType>,
}
