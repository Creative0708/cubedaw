use anyhow::Context;

use crate::plugin::misc;

use super::{function::PreparedFunction, stitch};

pub struct PreparedModule {
    hash: u64,

    plugin_version: semver::Version,
    tys: Box<[wasm_encoder::FuncType]>,

    plugin_imports: [Option<u32>; CubedawPluginImport::SIZE],
    plugin_exports: ahash::HashMap<resourc>,
    funcs: Box<[PreparedFunction]>,
    tables: Box<[misc::Table]>,
    memory: wasm_encoder::MemoryType,
    globals: Box<[misc::Global]>,
    elems: Box<[misc::ElementSegment]>,
    datas: Box<[misc::DataSegment]>,

    start_function: Option<u32>,
}

fn plugin_wasm_features() -> wasmparser::WasmFeatures {
    use wasmparser::WasmFeatures;
    WasmFeatures::BULK_MEMORY
        | WasmFeatures::FLOATS
        | WasmFeatures::MULTI_VALUE
        | WasmFeatures::MUTABLE_GLOBAL
        | WasmFeatures::RELAXED_SIMD
        | WasmFeatures::SATURATING_FLOAT_TO_INT
        | WasmFeatures::SIGN_EXTENSION
        | WasmFeatures::SIMD
        | WasmFeatures::TAIL_CALL
}
fn executing_wasm_features() -> wasmparser::WasmFeatures {
    use wasmparser::WasmFeatures;
    plugin_wasm_features() | WasmFeatures::MULTI_MEMORY
}

const MAX_SUPPORTED_VERSION: semver::Version = semver::Version {
    major: 0,
    minor: 1,
    patch: 0,
    pre: semver::Prerelease::EMPTY,
    build: semver::BuildMetadata::EMPTY,
};

impl PreparedModule {
    pub fn new(buf: &[u8]) -> anyhow::Result<Self> {
        let plugin_version = Self::module_version_from(buf)?;

        if !plugin_version
            .cmp_precedence(&MAX_SUPPORTED_VERSION)
            .is_le()
        {
            anyhow::bail!(
                "unsupported plugin version: {}. max supported version is {}",
                plugin_version,
                MAX_SUPPORTED_VERSION
            );
        }

        let mut validator = wasmparser::Validator::new_with_features(plugin_wasm_features());
        if buf.len() > u32::MAX as usize {
            anyhow::bail!("WASM module too big: {} (max {})", buf.len(), u32::MAX);
        }

        // good enough for now. shove this into the parser loop if performance is an issue
        let _types = validator.validate_all(buf)?;

        use wasm_encoder::reencode::Reencode;

        let mut parser = wasmparser::Parser::new(0);
        parser.set_features(plugin_wasm_features());

        let mut tys: Vec<wasm_encoder::FuncType> = Vec::new();
        let mut plugin_imports = [None; CubedawPluginImport::SIZE];
        let mut funcs: Vec<PreparedFunction> = Vec::new();
        let mut tables: Box<[misc::Table]> = Box::new([]);
        let mut memory: Option<wasm_encoder::MemoryType> = None;
        let mut globals: Box<[misc::Global]> = Box::new([]);
        let mut elems: Box<[misc::ElementSegment]> = Box::new([]);
        let mut datas: Box<[misc::DataSegment]> = Box::new([]);

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
                                panic!("array/struct type in webassembly module; validator didn't validate");
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
                            || plugin_imports[plugin_import as usize].is_some()
                        {
                            anyhow::bail!(
                                "import {:?} has invalid type: {:?}, expected {:?}",
                                import.name,
                                import.ty,
                                plugin_import.ty()
                            );
                        }

                        plugin_imports[plugin_import as usize] = Some(func_idx as u32);
                    }
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
                        .map(|table| Ok(misc::Table::new(table?)?))
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
                        .map(|global| Ok(misc::Global::new(global?)?))
                        .collect::<anyhow::Result<_>>()?;
                }
                wasmparser::Payload::ExportSection(r) => {
                    for export in r {
                        dbg!(export?);
                    }
                }
                wasmparser::Payload::StartSection { func, range: _ } => {
                    start_function = Some(func);
                }
                wasmparser::Payload::ElementSection(r) => {
                    assert!(elems.is_empty(), "module has more than one element section");

                    elems = r
                        .into_iter()
                        .map(|elem| Ok(misc::ElementSegment::new(elem?)?))
                        .collect::<anyhow::Result<_>>()?;
                }
                wasmparser::Payload::DataCountSection { count, range } => todo!(),
                wasmparser::Payload::DataSection(r) => {
                    assert!(datas.is_empty(), "module has more than one data section");

                    for data in r {
                        dbg!(data?.kind);
                    }
                }
                wasmparser::Payload::CodeSectionStart {
                    count,
                    range: _,
                    size: _,
                } => {
                    assert_eq!(count as usize, func_tys.len());
                }
                wasmparser::Payload::CodeSectionEntry(r) => {
                    funcs.push(PreparedFunction::new(
                        func_tys[current_function],
                        r,
                        &mut reencoder,
                    )?);
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
                    log::warn!("unknown custom section {:?}", section.name());
                }
                wasmparser::Payload::UnknownSection {
                    id,
                    contents,
                    range,
                } => {
                    log::warn!("unknown section with id {id}");
                }
                wasmparser::Payload::End(_end_length) => {}
            }
        }

        Ok(Self {
            hash: ahash::random_state::RandomState::new().hash_one(buf),

            plugin_version,
            tys: tys.into_boxed_slice(),

            plugin_imports,
            funcs: funcs.into_boxed_slice(),
            tables,
            memory: memory.ok_or_else(|| anyhow::anyhow!("plugin has no memory section"))?,
            globals,
            elems,
            datas,

            start_function,
        })
    }

    pub fn stitch(&self, func: &mut stitch::FunctionStitch, module: &mut stitch::ModuleStitch) {
        let offsets = self.get_offsets_or_stitch(module);
        for f in &self.funcs {
            module.funcs.function(f.ty());
        }
        for table in &self.tables {
            table.stitch(module, &offsets);
        }
        module.memories.memory(self.memory);
        for global in &self.globals {
            module
                .globals
                .global(global.ty.encode(), &global.init.encode(&offsets));
        }
        for elem in &self.elems {
            match elem.kind {
                misc::ElementKind::Active {
                    table_index,
                    ref offset,
                } => module.elems.active(
                    Some(table_index),
                    &offset.encode(&offsets),
                    wasm_encoder::Elements::Functions(&elem.items),
                ),
                misc::ElementKind::Passive => module
                    .elems
                    .passive(wasm_encoder::Elements::Functions(&elem.items)),
            };
        }
    }

    /// Gets the offsets for this module, stitching if not present.
    fn get_offsets_or_stitch(&self, module: &mut stitch::ModuleStitch) -> stitch::ModuleOffsets {
        if let Some(offsets) = module.offset_map.get(&self.hash) {
            return offsets.clone();
        }

        let offsets = module.current_offsets();
        module.offset_map.insert(self.hash, offsets);

        for ty in &self.tys {
            module.tys.func_type(ty);
        }
        for func in &self.funcs {
            module.funcs.function(func.ty());
            module.code.function(&func.encode(&offsets));
        }

        offsets
    }

    pub fn module_version_from(buf: &[u8]) -> anyhow::Result<semver::Version> {
        let parser = wasmparser::Parser::new(0);

        for payload in parser.parse_all(buf) {
            match payload? {
                wasmparser::Payload::CustomSection(section) => {
                    if let Some(CubedawSectionType::PluginVersion) =
                        CubedawSectionType::from_name(section.name())
                    {
                        return Ok(semver::Version::parse(
                            std::str::from_utf8(section.data())
                                .context("plugin version isn't valid utf-8")?,
                        )?);
                    }
                }
                _ => (),
            }
        }

        anyhow::bail!("module doesn't have a version (a custom section called \"cubedaw::plugin_version\"). is this actually a cubedaw plugin?");
    }
}

impl PartialEq for PreparedModule {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
impl Eq for PreparedModule {}
impl std::hash::Hash for PreparedModule {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash)
    }
}

/// Valid imports for a cubedaw plugin.
/// These are optional in case a plugin decides to not take any inputs or something.
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum CubedawPluginImport {
    SampleRate = 0,
    Input = 1,
    Output = 2,
}

impl CubedawPluginImport {
    pub const SIZE: usize = 3;

    pub fn new(name: &str) -> Option<Self> {
        Some(match name {
            "sample_rate" => Self::SampleRate,
            "input" => Self::Input,
            "output" => Self::Output,
            _ => return None,
        })
    }

    pub fn ty(self) -> wasm_encoder::FuncType {
        match self {
            Self::SampleRate => wasm_encoder::FuncType::new([], [wasm_encoder::ValType::I32]),
            Self::Input => wasm_encoder::FuncType::new(
                [wasm_encoder::ValType::I32],
                [
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::V128,
                ],
            ),
            Self::Output => wasm_encoder::FuncType::new(
                [
                    wasm_encoder::ValType::I32,
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::V128,
                    wasm_encoder::ValType::V128,
                ],
                [],
            ),
        }
    }
}

enum CubedawSectionType {
    PluginVersion,
}

impl CubedawSectionType {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "cubedaw:plugin_version" => Self::PluginVersion,
            _ => return None,
        })
    }
}
