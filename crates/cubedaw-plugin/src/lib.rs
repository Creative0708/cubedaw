pub use cubedaw_wasm as wasm;

mod misc;
pub mod prepare;
mod stitch;
pub use stitch::{FunctionStitch, ModuleStitch, ModuleStitchInfo, ShimContext, ShimInfo};
use wasm::WasmFeatures;
mod util;

use ahash::{HashMap, HashMapExt};
use anyhow::{Context, Result};
use prepare::{PrepareContext, PreparedFunction};
use resourcekey::{Namespace, ResourceKey};

// TODO: currently we use wasm_encoder types, is it worth revealing implementation details
// about the crate?
pub use wasm_encoder::Instruction;

pub struct Plugin {
    hash: u64,

    plugin_version: semver::Version,
    plugin_manifest: PluginManifest,
    tys: Box<[wasm_encoder::FuncType]>,

    import_indices: [Option<u32>; CubedawPluginImport::SIZE],
    exported_nodes: ahash::HashMap<ResourceKey, u32>,
    funcs: Box<[PreparedFunction]>,
    /// The index that the module's functions start at.
    /// This is equal to the number of `Some(_)`s in `cubedaw_imports`.
    func_offset: u32,
    tables: Box<[misc::Table]>,
    memory: wasm_encoder::MemoryType,
    globals: Box<[misc::Global]>,
    elems: Box<[misc::ElementSegment]>,
    datas: Box<[misc::DataSegment]>,

    start_function: Option<PreparedFunction>,
}

pub fn plugin_wasm_features() -> WasmFeatures {
    WasmFeatures::BULK_MEMORY
        | WasmFeatures::MULTI_VALUE
        | WasmFeatures::RELAXED_SIMD
        | WasmFeatures::REFERENCE_TYPES // idk why this is necessary but cubedaw-default-plugins requires it
        | WasmFeatures::SIMD
        | WasmFeatures::TAIL_CALL
}
pub fn executing_wasm_features() -> WasmFeatures {
    plugin_wasm_features() | WasmFeatures::MULTI_MEMORY
}

const MAX_SUPPORTED_VERSION: semver::Version = semver::Version {
    major: 0,
    minor: 1,
    patch: 0,
    pre: semver::Prerelease::EMPTY,
    build: semver::BuildMetadata::EMPTY,
};

impl Plugin {
    pub fn new(buf: &[u8]) -> anyhow::Result<Self> {
        let plugin_version = Self::module_version_from(buf)?;

        if plugin_version
            .cmp_precedence(&MAX_SUPPORTED_VERSION)
            .is_gt()
        {
            anyhow::bail!(
                "unsupported plugin version: {}. max supported version is {}",
                plugin_version,
                MAX_SUPPORTED_VERSION
            );
        }

        if buf.len() > u32::MAX as usize {
            anyhow::bail!("WASM module too big: {} (max {})", buf.len(), u32::MAX);
        }

        let mut validator = wasmparser::Validator::new_with_features(plugin_wasm_features().into());

        // good enough for now. shove this into the parser loop if performance is an issue
        let _types = validator.validate_all(buf)?;

        use wasm_encoder::reencode::Reencode;

        let mut parser = wasmparser::Parser::new(0);
        parser.set_features(plugin_wasm_features().into());

        let mut plugin_manifest: Option<PluginManifest> = None;

        // prepare_ctx only contains data from the type and import sections.
        // since those two sections come first in the module (see https://webassembly.github.io/spec/core/binary/modules.html#binary-module)
        // this will be Some(_) for everything afterwards (tables, memories, globals, etc.)
        let mut prepare_ctx: Option<PrepareContext> = None;

        let mut num_imports: usize = 0;

        let mut tys: Vec<wasm_encoder::FuncType> = Vec::new();
        let mut cubedaw_imports = [None; CubedawPluginImport::SIZE];
        let mut funcs: Vec<PreparedFunction> = Vec::new();
        let mut tables: Box<[misc::Table]> = Box::new([]);
        let mut memory: Option<wasm_encoder::MemoryType> = None;
        let mut globals: Box<[misc::Global]> = Box::new([]);
        let mut elems: Box<[misc::ElementSegment]> = Box::new([]);
        let mut datas: Box<[misc::DataSegment]> = Box::new([]);

        let mut func_exports: HashMap<&str, u32> = HashMap::new();
        let mut function_names_of_exported_modules: HashMap<ResourceKey, &str> = HashMap::new();

        let mut start_function: Option<u32> = None;

        let mut func_tys: Box<[u32]> = Box::new([]);

        let mut current_function = 0;

        let mut reencoder = wasm_encoder::reencode::RoundtripReencoder;

        let mut module = wasm_encoder::Module::new();

        for payload in parser.parse_all(buf) {
            match payload? {
                wasmparser::Payload::Version {
                    num: _,
                    encoding,
                    range: _,
                } => {
                    if encoding != wasmparser::Encoding::Module {
                        anyhow::bail!("webassembly module is a component")
                    }
                }
                wasmparser::Payload::TypeSection(r) => {
                    let mut type_section = wasm_encoder::TypeSection::new();
                    reencoder.parse_type_section(&mut type_section, r.clone())?;
                    module.section(&type_section);

                    for rec_group in r {
                        for ty in rec_group?.into_types() {
                            let wasmparser::CompositeInnerType::Func(func) =
                                ty.composite_type.inner
                            else {
                                panic!(
                                    "array/struct type in webassembly module; validator didn't validate"
                                );
                            };
                            tys.push(reencoder.func_type(func)?);
                        }
                    }
                }
                wasmparser::Payload::ImportSection(r) => {
                    for (func_idx, import) in r.into_iter().enumerate() {
                        let import = import?;

                        let plugin_import = CubedawPluginImport::new(import.name)
                            .ok_or_else(|| anyhow::anyhow!("unknown import {:?}", import.name))?;

                        let wasmparser::TypeRef::Func(type_idx) = import.ty else {
                            anyhow::bail!(
                                "import {:?} has invalid type: {:?}, expected {:?}",
                                import.name,
                                import.ty,
                                plugin_import.ty()
                            );
                        };
                        let ty = tys
                            .get(type_idx as usize)
                            .expect("validation didn't validate??");

                        if &plugin_import.ty() != ty
                            || cubedaw_imports[plugin_import as usize].is_some()
                        {
                            anyhow::bail!(
                                "import {:?} has invalid type: {:?}, expected {:?}",
                                import.name,
                                ty,
                                plugin_import.ty()
                            );
                        }

                        cubedaw_imports[plugin_import as usize] = Some(func_idx as u32);
                        num_imports += 1;
                    }

                    prepare_ctx = Some(PrepareContext {
                        import_function_indices: cubedaw_imports,
                    });
                }
                wasmparser::Payload::FunctionSection(r) => {
                    assert!(
                        func_tys.is_empty(),
                        "module has more than one function section"
                    );

                    func_tys = r
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?
                        .into_boxed_slice();
                }
                wasmparser::Payload::TableSection(r) => {
                    assert!(tables.is_empty(), "module has more than one table section");

                    tables = r
                        .into_iter()
                        .map(|table| {
                            Ok(misc::Table::new(
                                prepare_ctx
                                    .as_ref()
                                    .expect("unreachable; see declaration of prepare_ctx"),
                                table?,
                            )?)
                        })
                        .collect::<anyhow::Result<_>>()?;
                }
                wasmparser::Payload::MemorySection(r) => {
                    if r.count() != 1 {
                        anyhow::bail!(
                            "invalid number of memories: expected 1, found {}",
                            r.count()
                        );
                    }
                    if memory.is_some() {
                        // TODO is this reachable?
                        panic!("multiple memory sections in plugin");
                    }
                    let mem = r.into_iter().next().expect("unreachable")?;
                    // these should all have been covered by the validator at the start
                    assert!(!mem.memory64, "validator didn't validate");
                    assert!(!mem.shared, "validator didn't validate");

                    memory = Some(reencoder.memory_type(mem));
                }
                wasmparser::Payload::TagSection(_) => {
                    todo!("sorry i don't know what a tag section is")
                }
                wasmparser::Payload::GlobalSection(r) => {
                    assert!(
                        globals.is_empty(),
                        "module has more than one global section"
                    );

                    globals = r
                        .into_iter()
                        .map(|global| {
                            Ok(misc::Global::new(
                                prepare_ctx
                                    .as_ref()
                                    .expect("unreachable; see declaration of prepare_ctx"),
                                global?,
                            )?)
                        })
                        .collect::<anyhow::Result<_>>()?;
                }
                wasmparser::Payload::ExportSection(r) => {
                    for export in r {
                        let export = export?;
                        if export.kind == wasmparser::ExternalKind::Func {
                            func_exports.insert(export.name, export.index);
                        }
                    }
                }
                wasmparser::Payload::StartSection { func, range: _ } => {
                    start_function = Some(func);
                }
                wasmparser::Payload::ElementSection(r) => {
                    assert!(elems.is_empty(), "module has more than one element section");

                    elems = r
                        .into_iter()
                        .map(|elem| {
                            Ok(misc::ElementSegment::new(
                                prepare_ctx
                                    .as_ref()
                                    .expect("unreachable; see declaration of prepare_ctx"),
                                elem?,
                            )?)
                        })
                        .collect::<anyhow::Result<_>>()?;
                }
                wasmparser::Payload::DataCountSection { .. } => (),
                wasmparser::Payload::DataSection(r) => {
                    assert!(datas.is_empty(), "module has more than one data section");

                    datas = r
                        .into_iter()
                        .map(|data| {
                            Ok(misc::DataSegment::new(
                                prepare_ctx
                                    .as_ref()
                                    .expect("unreachable; see declaration of prepare_ctx"),
                                data?,
                            )?)
                        })
                        .collect::<anyhow::Result<_>>()?;
                }
                wasmparser::Payload::CodeSectionStart {
                    count,
                    range: _,
                    size: _,
                } => {
                    assert_eq!(count as usize, func_tys.len());
                }
                wasmparser::Payload::CodeSectionEntry(r) => {
                    let ty_index = func_tys[current_function];
                    funcs.insert(
                        current_function,
                        PreparedFunction::new(
                            prepare_ctx
                                .as_ref()
                                .expect("unreachable; see declaration of prepare_ctx"),
                            ty_index,
                            tys[ty_index as usize].clone(),
                            r,
                            &mut reencoder,
                        )?,
                    );
                    current_function += 1;
                }

                wasmparser::Payload::ModuleSection { .. }
                | wasmparser::Payload::InstanceSection(_)
                | wasmparser::Payload::CoreTypeSection(_)
                | wasmparser::Payload::ComponentSection { .. }
                | wasmparser::Payload::ComponentInstanceSection(_)
                | wasmparser::Payload::ComponentAliasSection(_)
                | wasmparser::Payload::ComponentTypeSection(_)
                | wasmparser::Payload::ComponentCanonicalSection(_)
                | wasmparser::Payload::ComponentStartSection { .. }
                | wasmparser::Payload::ComponentImportSection(_)
                | wasmparser::Payload::ComponentExportSection(_) => {
                    unreachable!("component payload recieved on a non-component wasm module");
                }

                wasmparser::Payload::CustomSection(section) => {
                    fn postcard_deserialize<'de, T: serde::Deserialize<'de>>(
                        bytes: &mut &'de [u8],
                    ) -> anyhow::Result<T> {
                        let val: T;
                        (val, *bytes) = postcard::take_from_bytes(bytes)?;
                        Ok(val)
                    }
                    match CubedawSectionType::from_name(section.name()) {
                        Some(CubedawSectionType::NodeList) => {
                            let mut bytes = section.data();
                            while !bytes.is_empty() {
                                let (key, export_name): (ResourceKey, &str) =
                                    postcard_deserialize(&mut bytes).with_context(|| {
                                        format!(
                                            "can't deserialize plugin entry from {:#?}",
                                            util::ByteString(bytes)
                                        )
                                    })?;
                                function_names_of_exported_modules.insert(key, export_name);
                            }
                        }
                        Some(CubedawSectionType::PluginMeta) => {
                            let mut bytes = section.data();
                            let mut id: Option<Namespace> = None;
                            let mut name: Option<String> = None;
                            let mut description: Option<String> = None;
                            while !bytes.is_empty() {
                                match postcard_deserialize::<&str>(&mut bytes)? {
                                    "id" => id = Some(postcard_deserialize(&mut bytes)?),
                                    "name" => name = Some(postcard_deserialize(&mut bytes)?),
                                    "description" => {
                                        description = Some(postcard_deserialize(&mut bytes)?)
                                    }
                                    other => anyhow::bail!("invalid key {other} in PluginManifest"),
                                }
                            }
                            plugin_manifest = Some(PluginManifest {
                                id: id.context("id doesn't exist in plugin manifest")?,
                                name: name.context("name doesn't exist in plugin manifest")?,
                                description,
                            });
                        }
                        _ => (),
                    }
                }
                wasmparser::Payload::UnknownSection {
                    id,
                    contents: _contents,
                    range: _range,
                } => {
                    tracing::warn!("unknown section with id {id}");
                }
                wasmparser::Payload::End(_end_length) => {}
            }
        }

        let mut exported_nodes = HashMap::new();
        let expected_func_type = wasm_encoder::FuncType::new(
            [wasm_encoder::ValType::I32, wasm_encoder::ValType::I32],
            [],
        );
        for (key, func_name) in function_names_of_exported_modules {
            let func_idx = *func_exports.get(func_name).with_context(|| {
                format!(
                    "module references nonexistent function {func_name}; available functions: {:?}",
                    func_exports.keys()
                )
            })?;

            // do some other checks on the function
            let ty = &tys[func_tys[func_idx as usize - num_imports] as usize];
            if ty != &expected_func_type {
                anyhow::bail!(
                    "invalid signature for node {key:?}, expected {:?}, got {:?}",
                    expected_func_type,
                    ty
                );
            }

            if exported_nodes.insert(key.clone(), func_idx).is_some() {
                anyhow::bail!("plugin has multiple exports for {key:?}");
            }
        }

        let start_function = start_function.map(|function| funcs[function as usize].clone());

        Ok(Self {
            hash: ahash::random_state::RandomState::new().hash_one(buf),

            plugin_version,
            plugin_manifest: plugin_manifest.context("plugin manifest doesn't exist")?,
            tys: tys.into_boxed_slice(),

            import_indices: cubedaw_imports,
            exported_nodes,
            funcs: funcs.into_boxed_slice(),
            func_offset: cubedaw_imports.iter().filter(|i| i.is_some()).count() as u32,
            tables,
            memory: memory.ok_or_else(|| anyhow::anyhow!("plugin has no memory section"))?,
            globals,
            elems,
            datas,

            start_function,
        })
    }

    pub fn version(&self) -> &semver::Version {
        &self.plugin_version
    }
    pub fn manifest(&self) -> &PluginManifest {
        &self.plugin_manifest
    }

    pub fn exported_nodes(&self) -> impl Iterator<Item = &ResourceKey> {
        self.exported_nodes.keys()
    }

    pub fn memory(&self) -> wasm_encoder::MemoryType {
        self.memory
    }

    pub fn stitch_node(
        &self,
        node: &ResourceKey,
        func: &mut stitch::FunctionStitch,
        module: &mut stitch::ModuleStitch,
    ) -> Result<()> {
        let info = self.get_stitch_info_or_insert(module);

        let func_idx = *self
            .exported_nodes
            .get(node)
            .with_context(|| format!("node {node} doesn't exist in plugin"))?;

        self.funcs[func_idx as usize - self.func_offset as usize].stitch(func, &info);

        Ok(())
    }

    /// Gets the `ModuleStitchInfo` for this module, stitching if not present.
    fn get_stitch_info_or_insert(
        &self,
        module: &mut stitch::ModuleStitch,
    ) -> stitch::ModuleStitchInfo {
        if let Some(offsets) = module.offset_map.get(&self.hash) {
            return offsets.clone();
        }

        let info = module.get_current_offsets_for(self);
        module.offset_map.insert(self.hash, info.clone());

        for ty in &self.tys {
            module.tys.func_type(ty);
        }
        for func in &self.funcs {
            module.add_function(func.encode_standalone(&info));
        }
        for table in &self.tables {
            table.stitch(module, &info);
        }
        module.memories.memory(self.memory);
        for global in &self.globals {
            module
                .globals
                .global(global.ty.encode(), &global.init.encode(&info));
        }
        for elem in &self.elems {
            match elem.kind {
                misc::ElementKind::Active {
                    table_index,
                    ref offset,
                } => module.elems.active(
                    Some(table_index),
                    &offset.encode(&info),
                    wasm_encoder::Elements::Functions(&elem.items),
                ),
                misc::ElementKind::Passive => module
                    .elems
                    .passive(wasm_encoder::Elements::Functions(&elem.items)),
            };
        }
        for data in &self.datas {
            match data.mode {
                misc::DataSegmentKind::Passive => {
                    module.datas.passive(data.data.iter().cloned());
                }
                misc::DataSegmentKind::Active {
                    memory_index,
                    ref offset,
                } => {
                    module.datas.active(
                        memory_index,
                        &offset.encode(&info),
                        data.data.iter().cloned(),
                    );
                }
            }
        }

        if let Some(ref start_function) = self.start_function {
            start_function.stitch(&mut module.start_function, &info);
        }

        info
    }

    pub fn module_version_from(buf: &[u8]) -> anyhow::Result<semver::Version> {
        let parser = wasmparser::Parser::new(0);

        for payload in parser.parse_all(buf) {
            if let wasmparser::Payload::CustomSection(section) = payload? {
                if matches!(
                    CubedawSectionType::from_name(section.name()),
                    Some(CubedawSectionType::PluginVersion)
                ) {
                    return Ok(semver::Version::parse(
                        std::str::from_utf8(section.data())
                            .context("plugin version isn't valid utf-8")?,
                    )?);
                }
            }
        }

        anyhow::bail!(
            "module doesn't have a version (a custom section called \"cubedaw::plugin_version\"). is this actually a cubedaw plugin?"
        );
    }
}

impl PartialEq for Plugin {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
impl Eq for Plugin {}
impl std::hash::Hash for Plugin {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash)
    }
}

impl std::fmt::Debug for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plugin")
            .field("id", &self.plugin_manifest.id)
            .finish_non_exhaustive()
    }
}

/// Valid imports for a cubedaw plugin.
/// These are optional in case a plugin decides to not take any inputs or something.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubedawPluginImport {
    SampleRate = 0,
    Input = 1,
    Output = 2,
    Attribute = 3,
}

impl CubedawPluginImport {
    pub const SIZE: usize = 4;
    pub const ALL: [Self; Self::SIZE] =
        [Self::SampleRate, Self::Input, Self::Output, Self::Attribute];

    pub fn new(name: &str) -> Option<Self> {
        Some(match name {
            "sample_rate" => Self::SampleRate,
            "input" => Self::Input,
            "output" => Self::Output,
            "attribute" => Self::Attribute,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::SampleRate => "sample_rate",
            Self::Input => "input",
            Self::Output => "output",
            Self::Attribute => "attribute",
        }
    }

    pub fn ty(self) -> wasm_encoder::FuncType {
        match self {
            Self::SampleRate => wasm_encoder::FuncType::new([], [wasm_encoder::ValType::I32]),
            Self::Input => wasm_encoder::FuncType::new([wasm_encoder::ValType::I32], [
                wasm_encoder::ValType::V128,
                wasm_encoder::ValType::V128,
                wasm_encoder::ValType::V128,
                wasm_encoder::ValType::V128,
            ]),
            Self::Output => wasm_encoder::FuncType::new(
                [
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::I32,
                ],
                [],
            ),
            Self::Attribute => wasm_encoder::FuncType::new([wasm_encoder::ValType::I32], [
                wasm_encoder::ValType::V128,
                wasm_encoder::ValType::V128,
                wasm_encoder::ValType::V128,
                wasm_encoder::ValType::V128,
            ]),
        }
    }
}

enum CubedawSectionType {
    PluginVersion,
    PluginMeta,
    NodeList,
}

impl CubedawSectionType {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "cubedaw:plugin_version" => Self::PluginVersion,
            "cubedaw:plugin_meta" => Self::PluginMeta,
            "cubedaw:node_list" => Self::NodeList,
            _ => return None,
        })
    }
}

pub struct PluginManifest {
    pub id: Namespace,
    pub name: String,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests;
