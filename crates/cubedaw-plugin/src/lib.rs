// TODO: add ability to have multiple nodes in a module?
pub fn prepare_module(buf: &[u8]) -> Result<PreparedModuleData<'_>, PrepareModuleError> {
    let parser = wasmparser::Parser::new(0);

    let mut tys = Vec::new();
    let mut plugin_imports = [None; CubedawPluginImport::SIZE];

    let mut functions: Option<Box<[PreparedFunction]>> = None;
    let mut current_func = 0;

    for payload in parser.parse_all(buf) {
        match payload? {
            wasmparser::Payload::Version {
                num: _,
                encoding,
                range: _,
            } => {
                if encoding != wasmparser::Encoding::Module {
                    return Err(PrepareModuleError::WasmIsComponent);
                }
            }
            wasmparser::Payload::TypeSection(r) => {
                // TODO figure out what the hell a recgroup is and if this type ordering is correct
                // https://webassembly.github.io/gc/core/syntax/types.html#recursive-types
                for rec_group in r {
                    for ty in rec_group?.into_types() {
                        // am i doing this right?
                        tys.push(ty.composite_type.inner);
                    }
                }
            }
            wasmparser::Payload::ImportSection(r) => {
                // TODO is func_idx just the order the imports are in?
                for (func_idx, import) in r.into_iter().enumerate() {
                    let import = import?;

                    let plugin_import = CubedawPluginImport::new(import.name).ok_or_else(|| {
                        PrepareModuleError::InvalidImport {
                            name: import.name.to_owned(),
                            ty: import.ty,
                        }
                    })?;

                    let wasmparser::TypeRef::Func(type_idx) = import.ty else {
                        return Err(PrepareModuleError::InvalidImport {
                            name: import.name.to_owned(),
                            ty: import.ty,
                        });
                    };
                    let ty = tys
                        .get(type_idx as usize)
                        .ok_or_else(|| PrepareModuleError::MalformedWasm)?;
                    let wasmparser::CompositeInnerType::Func(func_type) = ty else {
                        return Err(PrepareModuleError::MalformedWasm);
                    };

                    if plugin_import.ty() != *func_type
                        || plugin_imports[plugin_import as usize].is_some()
                    {
                        return Err(PrepareModuleError::InvalidImport {
                            name: import.name.to_owned(),
                            ty: import.ty,
                        });
                    }

                    plugin_imports[plugin_import as usize] = Some(func_idx as u32);
                }
            }
            wasmparser::Payload::FunctionSection(r) => {
                if functions.is_some() {
                    return Err(PrepareModuleError::MalformedWasm);
                }

                let mut funcs = vec![
                    PreparedFunction {
                        ty: 0,
                        locals: Box::new([]),
                        code: Box::new([])
                    };
                    r.count() as usize
                ]
                .into_boxed_slice();

                for (func_idx, func) in r.into_iter().enumerate() {
                    funcs[func_idx].ty = func?;
                }

                functions = Some(funcs);
            }
            wasmparser::Payload::TableSection(r) => {
                for table in r {
                    dbg!(table?);
                }
            }
            wasmparser::Payload::MemorySection(r) => {
                for memory in r {
                    dbg!(memory?);
                }
            }
            wasmparser::Payload::TagSection(_) => todo!(),
            wasmparser::Payload::GlobalSection(r) => {
                for global in r {
                    dbg!(global?);
                }
            }
            wasmparser::Payload::ExportSection(r) => {
                for export in r {
                    dbg!(export?);
                }
            }
            wasmparser::Payload::StartSection { func, range } => todo!(),
            wasmparser::Payload::ElementSection(_) => todo!(),
            wasmparser::Payload::DataCountSection { count, range } => todo!(),
            wasmparser::Payload::DataSection(_) => todo!(),
            wasmparser::Payload::CodeSectionStart {
                count,
                range: _,
                size: _,
            } => {
                dbg!((count, functions.as_ref().unwrap().len()));
            }
            wasmparser::Payload::CodeSectionEntry(r) => {
                let Some(ref mut functions) = functions else {
                    return Err(PrepareModuleError::MalformedWasm);
                };
                let function = &mut functions[current_func];
                let locals = r.get_locals_reader()?;
                function.locals = vec![(0, wasmparser::ValType::I32); locals.get_count() as usize]
                    .into_boxed_slice();
                for (local_idx, local) in locals.into_iter().enumerate() {
                    *function
                        .locals
                        .get_mut(local_idx)
                        .ok_or_else(|| PrepareModuleError::MalformedWasm)? = local?;
                }
                let instructions = r.get_operators_reader()?;
                let mut code = Vec::new();
                for instruction in instructions {
                    code.push(instruction?.try_into()?);
                }
                function.code = code.into_boxed_slice();

                current_func += 1;
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
            wasmparser::Payload::End(_) => todo!(),
        }
    }

    Ok(todo!())
}

#[derive(Debug)]
pub enum PrepareModuleError {
    WasmIsComponent,
    MalformedWasm,
    InvalidImport {
        name: String,
        ty: wasmparser::TypeRef,
    },
    WasmError(wasmparser::BinaryReaderError),
    ReencodeError(wasm_encoder::reencode::Error),
}
impl From<wasmparser::BinaryReaderError> for PrepareModuleError {
    fn from(value: wasmparser::BinaryReaderError) -> Self {
        Self::WasmError(value)
    }
}
impl From<wasm_encoder::reencode::Error> for PrepareModuleError {
    fn from(value: wasm_encoder::reencode::Error) -> Self {
        Self::ReencodeError(value)
    }
}

#[derive(Default)]
pub struct PreparedModuleData<'a> {
    tys: Vec<wasmparser::CompositeInnerType>,
    plugin_imports: [Option<u32>; CubedawPluginImport::SIZE],
    functions: Box<[PreparedFunction<'a>]>,
}

// TODO possibly store a semi-complete byte representation for function instructions
// for optimization purposes?
#[derive(Clone)]
pub struct PreparedFunction<'a> {
    ty: u32,
    locals: Box<[(u32, wasmparser::ValType)]>,
    code: Box<[wasm_encoder::Instruction<'a>]>,
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

    pub fn ty(self) -> wasmparser::FuncType {
        match self {
            Self::SampleRate => wasmparser::FuncType::new([], [wasmparser::ValType::I32]),
            Self::Input => {
                wasmparser::FuncType::new([wasmparser::ValType::I32], [wasmparser::ValType::F32])
            }
            Self::Output => {
                wasmparser::FuncType::new([wasmparser::ValType::I32, wasmparser::ValType::F32], [])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        // TODO implement
        // super::prepare_module(&std::fs::read("/tmp/a.wasm").unwrap()).unwrap();
    }
}
