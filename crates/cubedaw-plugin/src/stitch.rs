use std::{any::Any, borrow::Cow, mem, rc::Rc};

use wasm_encoder::{Encode, Instruction, ValType};
use wasmparser::FunctionSectionReader;

use crate::CubedawPluginImport;

#[derive(Debug)]
pub struct ModuleStitch {
    pub offset_map: ahash::HashMap<u64, ModuleStitchInfo>,
    pub shim_info: ShimInfo,

    // https://webassembly.github.io/spec/core/binary/modules.html#indices
    pub tys: wasm_encoder::TypeSection,
    pub funcs: wasm_encoder::FunctionSection,
    pub code: wasm_encoder::CodeSection,
    pub tables: wasm_encoder::TableSection,
    pub memories: wasm_encoder::MemorySection,
    pub globals: wasm_encoder::GlobalSection,
    pub elems: wasm_encoder::ElementSection,
    pub datas: wasm_encoder::DataSection,
    pub exports: wasm_encoder::ExportSection,

    pub start_function: FunctionStitch,

    _private: private::Private,
}

mod private {
    #[derive(Debug, Clone, Copy)]
    pub struct Private;
}

impl ModuleStitch {
    pub fn new(shim_info: ShimInfo) -> Self {
        let mut this = Self {
            offset_map: Default::default(),
            shim_info,

            tys: Default::default(),
            funcs: Default::default(),
            code: Default::default(),
            tables: Default::default(),
            memories: Default::default(),
            globals: Default::default(),
            elems: Default::default(),
            datas: Default::default(),
            exports: Default::default(),

            start_function: FunctionStitch::empty(),

            _private: private::Private,
        };
        for import in CubedawPluginImport::ALL {
            this.tys.func_type(&import.ty());
        }
        this
    }
    pub fn get_current_offsets_for(&self, module: &crate::Plugin) -> ModuleStitchInfo {
        ModuleStitchInfo {
            shim_info: self.shim_info.clone(),

            ty_offset: self.tys.len(),
            func_offset: self.funcs.len(),
            table_offset: self.tables.len(),
            memory_offset: self.memories.len(),
            global_offset: self.globals.len(),
            elem_offset: self.elems.len(),
            data_index: self.datas.len() as u32,

            cubedaw_imports: ImportStitchInfo {
                mappings: module.cubedaw_imports,
            },
        }
    }
    pub fn add_function(&mut self, func: FunctionStitch) -> u32 {
        let type_idx = self.tys.len();
        self.tys.function(func.params, func.results);
        let func_idx = self.funcs.len();
        self.funcs.function(type_idx);
        func_idx
    }
    pub fn finish(mut self) -> Vec<u8> {
        let mut encoder = wasm_encoder::Module::new();

        let start_function = mem::replace(&mut self.start_function, FunctionStitch::empty());
        let start_func_idx = self.add_function(start_function);

        // https://webassembly.github.io/spec/core/binary/modules.html#binary-module
        encoder.section(&self.tys);
        encoder.section(&{
            let mut cubedaw_imports = wasm_encoder::ImportSection::new();
            for import in CubedawPluginImport::ALL {
                cubedaw_imports.import(
                    "env",
                    import.name(),
                    wasm_encoder::EntityType::Function(import as u32),
                );
            }

            cubedaw_imports
        });
        encoder.section(&self.funcs);
        encoder.section(&self.tables);
        encoder.section(&self.memories);
        encoder.section(&self.globals);
        encoder.section(&self.exports);
        encoder.section(&wasm_encoder::StartSection {
            function_index: start_func_idx,
        });
        encoder.section(&self.elems);
        encoder.section(&wasm_encoder::DataCountSection {
            count: self
                .datas
                .len()
                .try_into()
                .expect("how the heck do you have 4 billion data sections"),
        });

        encoder.finish()
    }
}

#[derive(Clone, Debug)]
pub struct ModuleStitchInfo {
    pub shim_info: ShimInfo,

    pub ty_offset: u32,
    pub func_offset: u32,
    pub table_offset: u32,
    pub memory_offset: u32,
    pub global_offset: u32,
    pub elem_offset: u32,
    pub data_index: u32,

    pub cubedaw_imports: ImportStitchInfo,
}

#[derive(Debug)]
pub struct FunctionStitch {
    params: Box<[ValType]>,
    results: Box<[ValType]>,
    pub locals: Vec<ValType>,
    pub code: Vec<u8>,
}
impl FunctionStitch {
    pub fn new(
        params: impl IntoIterator<Item = ValType>,
        results: impl IntoIterator<Item = ValType>,
    ) -> Self {
        Self {
            params: params.into_iter().collect(),
            results: results.into_iter().collect(),

            locals: Default::default(),
            code: Default::default(),
        }
    }
    pub fn empty() -> Self {
        Self::new([], [])
    }

    /// Encodes an instruction. This does not take shims into account; use with caution!
    pub fn instruction<'i, 'a>(
        &self,
        inst: &'i Instruction<'a>,
        offsets: &ModuleStitchInfo,
    ) -> Cow<'i, Instruction<'a>> {
        use wasm_encoder::Instruction as I;

        let m = |memarg: wasm_encoder::MemArg| wasm_encoder::MemArg {
            memory_index: offsets.memory_offset,
            ..memarg
        };
        let l = |local: u32| local + self.locals.len() as u32;
        let t = |table: u32| table + offsets.table_offset;
        let e = |elem: u32| elem + offsets.elem_offset;

        let me = |mem: u32| {
            debug_assert_eq!(mem, 0, "webassembly module uses multiple memories");
            offsets.memory_offset
        };
        let d = |data: u32| data + offsets.data_index;

        let modified_inst = match *inst {
            // Memory instructions.
            I::I32Load(memarg) => I::I32Load(m(memarg)),
            I::I64Load(memarg) => I::I64Load(m(memarg)),
            I::F32Load(memarg) => I::F32Load(m(memarg)),
            I::F64Load(memarg) => I::F64Load(m(memarg)),
            I::I32Load8S(memarg) => I::I32Load8S(m(memarg)),
            I::I32Load8U(memarg) => I::I32Load8U(m(memarg)),
            I::I32Load16S(memarg) => I::I32Load16S(m(memarg)),
            I::I32Load16U(memarg) => I::I32Load16U(m(memarg)),
            I::I64Load8S(memarg) => I::I64Load8S(m(memarg)),
            I::I64Load8U(memarg) => I::I64Load8U(m(memarg)),
            I::I64Load16S(memarg) => I::I64Load16S(m(memarg)),
            I::I64Load16U(memarg) => I::I64Load16U(m(memarg)),
            I::I64Load32S(memarg) => I::I64Load32S(m(memarg)),
            I::I64Load32U(memarg) => I::I64Load32U(m(memarg)),
            I::I32Store(memarg) => I::I32Store(m(memarg)),
            I::I64Store(memarg) => I::I64Store(m(memarg)),
            I::F32Store(memarg) => I::F32Store(m(memarg)),
            I::F64Store(memarg) => I::F64Store(m(memarg)),
            I::I32Store8(memarg) => I::I32Store8(m(memarg)),
            I::I32Store16(memarg) => I::I32Store16(m(memarg)),
            I::I64Store8(memarg) => I::I64Store8(m(memarg)),
            I::I64Store16(memarg) => I::I64Store16(m(memarg)),
            I::I64Store32(memarg) => I::I64Store32(m(memarg)),
            I::MemorySize(mem) => I::MemorySize(me(mem)),
            I::MemoryGrow(mem) => I::MemoryGrow(me(mem)),
            I::MemoryInit { mem, data_index } => I::MemoryInit {
                mem: me(mem),
                data_index: d(data_index),
            },
            I::DataDrop(data) => I::DataDrop(d(data)),
            I::MemoryCopy { src_mem, dst_mem } => I::MemoryCopy {
                src_mem: me(src_mem),
                dst_mem: me(dst_mem),
            },
            I::MemoryFill(mem) => I::MemoryFill(me(mem)),
            I::MemoryDiscard(mem) => I::MemoryDiscard(me(mem)),

            // Bulk memory instructions.
            I::TableInit { elem_index, table } => I::TableInit {
                elem_index: e(elem_index),
                table: t(table),
            },
            I::ElemDrop(elem) => I::ElemDrop(e(elem)),
            I::TableFill(table) => I::TableFill(t(table)),
            I::TableSet(table) => I::TableSet(t(table)),
            I::TableGet(table) => I::TableGet(t(table)),
            I::TableGrow(table) => I::TableGrow(t(table)),
            I::TableSize(table) => I::TableSize(t(table)),
            I::TableCopy {
                src_table,
                dst_table,
            } => I::TableCopy {
                src_table: t(src_table),
                dst_table: t(dst_table),
            },

            // SIMD instructions.
            I::V128Load(memarg) => I::V128Load(m(memarg)),
            I::V128Load8x8S(memarg) => I::V128Load8x8S(m(memarg)),
            I::V128Load8x8U(memarg) => I::V128Load8x8U(m(memarg)),
            I::V128Load16x4S(memarg) => I::V128Load16x4S(m(memarg)),
            I::V128Load16x4U(memarg) => I::V128Load16x4U(m(memarg)),
            I::V128Load32x2S(memarg) => I::V128Load32x2S(m(memarg)),
            I::V128Load32x2U(memarg) => I::V128Load32x2U(m(memarg)),
            I::V128Load8Splat(memarg) => I::V128Load8Splat(m(memarg)),
            I::V128Load16Splat(memarg) => I::V128Load16Splat(m(memarg)),
            I::V128Load32Splat(memarg) => I::V128Load32Splat(m(memarg)),
            I::V128Load64Splat(memarg) => I::V128Load64Splat(m(memarg)),
            I::V128Load32Zero(memarg) => I::V128Load32Zero(m(memarg)),
            I::V128Load64Zero(memarg) => I::V128Load64Zero(m(memarg)),
            I::V128Store(memarg) => I::V128Store(m(memarg)),
            I::V128Load8Lane { memarg, lane } => I::V128Load8Lane {
                memarg: m(memarg),
                lane,
            },
            I::V128Load16Lane { memarg, lane } => I::V128Load16Lane {
                memarg: m(memarg),
                lane,
            },
            I::V128Load32Lane { memarg, lane } => I::V128Load32Lane {
                memarg: m(memarg),
                lane,
            },
            I::V128Load64Lane { memarg, lane } => I::V128Load64Lane {
                memarg: m(memarg),
                lane,
            },
            I::V128Store8Lane { memarg, lane } => I::V128Store8Lane {
                memarg: m(memarg),
                lane,
            },
            I::V128Store16Lane { memarg, lane } => I::V128Store16Lane {
                memarg: m(memarg),
                lane,
            },
            I::V128Store32Lane { memarg, lane } => I::V128Store32Lane {
                memarg: m(memarg),
                lane,
            },
            I::V128Store64Lane { memarg, lane } => I::V128Store64Lane {
                memarg: m(memarg),
                lane,
            },

            // Variable instructions.
            I::LocalGet(idx) => I::LocalGet(l(idx)),
            I::LocalSet(idx) => I::LocalSet(l(idx)),
            I::LocalTee(idx) => I::LocalTee(l(idx)),

            // there are a bunch of atomic/gc/whatever instructions but those aren't allowed in plugins
            // so we don't need to handle them

            // instruction doesn't need to be modified
            ref other => return Cow::Borrowed(other),
        };

        return Cow::Owned(modified_inst);
    }

    pub fn finalize(self) -> wasm_encoder::Function {
        let mut locals_vec: Vec<(u32, ValType)> = Vec::new();
        for &local_ty in &self.locals {
            if let Some((num, ty)) = locals_vec.last_mut() {
                if *ty == local_ty {
                    *num += 1;
                    continue;
                }
            }
            locals_vec.push((1, local_ty));
        }
        let mut func = wasm_encoder::Function::new(locals_vec);
        func.raw(self.code.iter().cloned());
        func
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportStitchInfo {
    mappings: [Option<u32>; CubedawPluginImport::SIZE],
}
impl ImportStitchInfo {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn transform(&self, func_idx: u32) -> u32 {
        self.mappings
            .into_iter()
            .zip(CubedawPluginImport::ALL)
            .find_map(|(idx, ty)| (idx == Some(func_idx)).then_some(ty))
            .map_or(func_idx, |ty| ty as u32)
    }
}

#[derive(Clone)]
pub struct ShimInfo {
    shim: Rc<dyn Fn(ShimContext)>,
}
impl ShimInfo {
    pub fn new(f: impl Fn(ShimContext) + 'static) -> Self {
        Self { shim: Rc::new(f) }
    }
}

impl std::fmt::Debug for ShimInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShimInfo").finish_non_exhaustive()
    }
}

// TODO: currently this is hardcoded to provide the current instruction and the single instruction in front.
// in the future this should probably be configurable.
// also the fields probably shouldn't be pub
/// Docs TODO.
///
/// Call `ShimContext::replace` to replace the instructions and `ShimContext::insert_original` to
/// instert the original instructions.
pub struct ShimContext<'a> {
    import: CubedawPluginImport,

    pub prev_instruction: Instruction<'static>,
    pub current_instruction: Instruction<'static>,

    sink: &'a mut Vec<u8>,
}
impl ShimContext<'_> {
    pub fn import(&self) -> CubedawPluginImport {
        self.import
    }
    pub fn replace(mut self, iter: impl IntoIterator<Item = Instruction<'static>>) {
        for instruction in iter {
            instruction.encode(&mut self.sink);
        }
    }
    pub fn insert_original(mut self) {
        self.prev_instruction.encode(&mut self.sink);
        self.current_instruction.encode(&mut self.sink);
    }
}
