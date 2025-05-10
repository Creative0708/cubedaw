use std::{mem, ptr::NonNull};

use ahash::HashMap;
use anyhow::Result;
use cubedaw_lib::InternalBufferType;
use cubedaw_plugin::{CubedawPluginImport, Instruction};
use cubedaw_wasm::{ValType, Value};
use resourcekey::ResourceKey;

use crate::{WorkerOptions, plugin::Attribute};

use super::{AttributeMap, PLUGIN_ALIGN};

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
                    CubedawPluginImport::Attribute => {
                        ctx.replace_only_current([]);
                        ctx.add_instruction_raw(Instruction::Call(2));
                    }
                }
            }),
            [
                ("input", CubedawPluginImport::Input.ty()),
                ("output", CubedawPluginImport::Output.ty()),
                ("attribute", CubedawPluginImport::Attribute.ty()),
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

        let module =
            cubedaw_wasm::Module::new(options.registry.engine(), &wasm_module).expect("todo!()");

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
    /// Runs the plugin. Make sure the input data is set before running this!
    pub fn run(
        &mut self,
        key: &ResourceKey,
        args: &[u8],
        state: &mut [u8],
        attribute_map: &mut dyn AttributeMap,
    ) -> anyhow::Result<()> {
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

        let params = self.store.data_mut();
        // SAFETY: we're just extending the lifetime here
        params.attribute_map = unsafe {
            mem::transmute::<NonNull<_>, NonNull<_>>(NonNull::from_mut(&mut *attribute_map))
        };

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
#[derive(Debug)]
pub struct StandalonePluginParameters {
    pub inputs: Vec<InternalBufferType>,
    pub outputs: Vec<InternalBufferType>,

    attribute_map: NonNull<dyn AttributeMap>,
}
impl Default for StandalonePluginParameters {
    fn default() -> Self {
        Self {
            inputs: Default::default(),
            outputs: Default::default(),

            attribute_map: NonNull::<super::NoopAttributeMap>::dangling(),
        }
    }
}
impl StandalonePluginParameters {
    /// # Safety
    /// The caller must be accessing this from a linker function call, as the attribute map is only valid during the call. Also, like, the returned `AttributeMap` isn't actually `AttributeMap + 'static`; don't share the reference anywhere.
    unsafe fn attribute_map(&mut self) -> &mut dyn AttributeMap {
        unsafe { self.attribute_map.as_mut() }
    }
}

pub(crate) fn make_linker(
    engine: &cubedaw_wasm::Engine,
) -> cubedaw_wasm::Linker<StandalonePluginParameters> {
    use cubedaw_wasm::{V128, wasmtime::V128 as OtherV128};

    let mut linker = cubedaw_wasm::Linker::new(engine);
    linker
        .func_wrap(
            "host",
            "input",
            |caller: cubedaw_wasm::wasmtime::Caller<'_, StandalonePluginParameters>,
             input_idx: u32|
             -> (OtherV128, OtherV128, OtherV128, OtherV128) {
                let data = caller.data();
                let [a, b, c, d]: [V128; 4] = bytemuck::must_cast(
                    data.inputs
                        .get(input_idx as usize)
                        .copied()
                        .unwrap_or_else(|| {
                            tracing::warn!(
                                "plugin tried to fetch out of range input index {input_idx}"
                            );
                            bytemuck::zeroed()
                        }),
                );
                (a.into(), b.into(), c.into(), d.into())
            },
        )
        .expect("failed to link");
    linker
        .func_wrap(
            "host",
            "output",
            |mut caller: cubedaw_wasm::wasmtime::Caller<'_, StandalonePluginParameters>,
             a: OtherV128,
             b: OtherV128,
             c: OtherV128,
             d: OtherV128,
             output_idx: u32| {
                let data = caller.data_mut();
                let Some(output) = data.outputs.get_mut(output_idx as usize) else {
                    return;
                };
                let arr: [V128; 4] = [a.into(), b.into(), c.into(), d.into()];

                *output = bytemuck::must_cast(arr);
            },
        )
        .expect("failed to link");
    linker
        .func_wrap(
            "host",
            "attribute",
            |mut caller: cubedaw_wasm::wasmtime::Caller<'_, StandalonePluginParameters>,
             attribute_int: u32|
             -> (OtherV128, OtherV128, OtherV128, OtherV128) {
                let data = caller.data_mut();

                // SAFETY: we are inside a linker-wrapped function
                let attribute_map = unsafe { data.attribute_map() };
                let val = match Attribute::from_int(attribute_int) {
                    Some(attribute) => attribute_map.attribute(attribute),
                    None => {
                        tracing::warn!("plugin tried to fetch unknown attribute {attribute_int}");
                        bytemuck::zeroed()
                    }
                };

                let [a, b, c, d]: [V128; 4] = bytemuck::must_cast(val);
                (a.into(), b.into(), c.into(), d.into())
            },
        )
        .expect("failed to link");
    linker
}
