use std::{borrow::Cow, mem, rc::Rc};

use wasm_encoder::{Encode, FuncType, Instruction, ValType};

use crate::CubedawPluginImport;

#[derive(Debug)]
pub struct ModuleStitch {
    pub offset_map: ahash::HashMap<u64, ModuleStitchInfo>,
    pub shim_info: ShimInfo,

    // https://webassembly.github.io/spec/core/binary/modules.html#indices
    pub tys: wasm_encoder::TypeSection,
    pub imports: wasm_encoder::ImportSection,
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
        Self::with_imports(shim_info, [])
    }
    /// Like `Self::new`, but with imports. Imports are limited to functions for now, sorry!
    pub fn with_imports<'a>(
        shim_info: ShimInfo,
        imports: impl IntoIterator<Item = (&'a str, wasm_encoder::FuncType)>,
    ) -> Self {
        let mut this = Self {
            offset_map: Default::default(),
            shim_info,

            tys: Default::default(),
            imports: Default::default(),
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
        for (name, ty) in imports {
            let func_idx = this.tys.len();
            this.tys.func_type(&ty);
            this.imports
                .import("host", name, wasm_encoder::EntityType::Function(func_idx));
        }
        this
    }
    pub fn get_current_offsets_for(&self, plugin: &crate::Plugin) -> ModuleStitchInfo {
        ModuleStitchInfo {
            shim_info: self.shim_info.clone(),

            ty_offset: self.tys.len(),
            func_offset: (self.imports.len() + self.funcs.len()) as i32
                - plugin.import_indices.iter().filter(|x| x.is_some()).count() as i32,
            table_offset: self.tables.len(),
            memory_offset: self.memories.len(),
            global_offset: self.globals.len(),
            elem_offset: self.elems.len(),
            data_index: self.datas.len() as u32,

            import_indices: plugin.import_indices,
        }
    }
    /// Stitches a function into the module and returns its index.
    pub fn add_function(&mut self, func: FunctionStitch) -> u32 {
        let (func_type, code) = func.finish();
        let type_idx = self.tys.len();
        self.tys.func_type(&func_type);
        let func_idx = self.funcs.len();
        self.funcs.function(type_idx);
        self.code.function(&code);

        func_idx + self.imports.len()
    }
    /// Utility function. Equivalent to `self.exports.export(name, wasm_encoder::ExportKind::Func, idx)`
    pub fn export_function(&mut self, name: &str, func_idx: u32) {
        self.exports
            .export(name, wasm_encoder::ExportKind::Func, func_idx);
    }
    /// Utility function. Equivalent to `self.exports.export(name, wasm_encoder::ExportKind::Memory, idx)`
    pub fn export_memory(&mut self, name: &str, memory_idx: u32) {
        self.exports
            .export(name, wasm_encoder::ExportKind::Memory, memory_idx);
    }
    pub fn finish(mut self) -> Vec<u8> {
        let mut encoder = wasm_encoder::Module::new();

        let start_function = mem::replace(&mut self.start_function, FunctionStitch::empty());
        let start_func_idx = if start_function.code.is_empty() {
            None
        } else {
            Some(self.add_function(start_function))
        };

        // WASM sections have to be in a specific order
        // https://webassembly.github.io/spec/core/binary/modules.html#binary-module
        encoder.section(&self.tys);
        encoder.section(&self.imports);
        if self.funcs.len() > 0 {
            encoder.section(&self.funcs);
        }
        if self.tables.len() > 0 {
            encoder.section(&self.tables);
        }
        if self.memories.len() > 0 {
            encoder.section(&self.memories);
        }
        if self.globals.len() > 0 {
            encoder.section(&self.globals);
        }
        if self.exports.len() > 0 {
            encoder.section(&self.exports);
        }
        if let Some(start_func_idx) = start_func_idx {
            encoder.section(&wasm_encoder::StartSection {
                function_index: start_func_idx,
            });
        }
        if self.elems.len() > 0 {
            encoder.section(&self.elems);
        }
        encoder.section(&wasm_encoder::DataCountSection {
            count: self
                .datas
                .len()
                .try_into()
                .expect("how the heck do you have 4 billion data sections"),
        });
        eprintln!("code offset: {}", encoder.as_slice().len());
        encoder.section(&self.code);
        if self.datas.len() > 0 {
            encoder.section(&self.datas);
        }

        encoder.finish()
    }
}

#[derive(Clone, Debug)]
pub struct ModuleStitchInfo {
    pub shim_info: ShimInfo,

    pub ty_offset: u32,
    /// Import-aware function offset for this plugin.
    ///
    /// Basically, when creating a `ModuleStitchInfo`, this is the number of functions currently in the `ModuleStitchInfo` minus the number of imports a plugin has.
    ///
    /// For example, if a plugin has imports `sample_rate` and `input` and is the first plugin stitched into this `ModuleStitchInfo`, `func_offset` would be `-2`. This makes it so that the first "real" function has index `0`. Convenient!
    pub func_offset: i32,
    pub table_offset: u32,
    pub memory_offset: u32,
    pub global_offset: u32,
    pub elem_offset: u32,
    pub data_index: u32,

    import_indices: [Option<u32>; CubedawPluginImport::SIZE],
}
impl ModuleStitchInfo {
    pub fn get_import_of_func_idx(&self, func_idx: u32) -> Option<CubedawPluginImport> {
        self.import_indices
            .into_iter()
            .zip(CubedawPluginImport::ALL)
            .find_map(|(import_idx, import)| (import_idx == Some(func_idx)).then_some(import))
    }
}

#[derive(Debug)]
pub struct FunctionStitch {
    ty: FuncType,
    /// Locals. Does not include parameters
    locals: Vec<ValType>,
    // this shouldn't be pub but also `wasm_encoder::Instruction::encode` requires a `&mut Vec<u8>`. :shrug:
    pub code: Vec<u8>,

    // required for "standalone" functions which are stitched once then finished
    include_params_in_locals_offset: bool,
}
impl FunctionStitch {
    pub fn new(ty: FuncType) -> Self {
        Self {
            ty,
            locals: Default::default(),
            code: Default::default(),
            include_params_in_locals_offset: true,
        }
    }
    pub fn standalone(ty: FuncType) -> Self {
        Self {
            ty,
            locals: Default::default(),
            code: Default::default(),
            include_params_in_locals_offset: false,
        }
    }
    pub fn empty() -> Self {
        Self::new(FuncType::new([], []))
    }

    pub fn params(&self) -> &[ValType] {
        self.ty.params()
    }
    pub fn results(&self) -> &[ValType] {
        self.ty.results()
    }

    /// "If I were to add another local right now, what index would it be?"
    ///
    /// This function answers that. `self.params().len() + self.locals.len()`\*
    ///
    /// _\*except when constructed with `FunctionStitch::standalone`, in which case it's just `self.locals.len()`_
    pub fn locals_offset(&self) -> u32 {
        (if self.include_params_in_locals_offset {
            self.params().len() + self.locals.len()
        } else {
            self.locals.len()
        }) as u32
    }

    /// Adds locals to this function. Be warned that `encode_instruction` takes the current locals length into account; This means you have to add locals to a function _after_ you add the instructions!
    pub fn extend_locals(&mut self, locals: impl IntoIterator<Item = ValType>) {
        self.locals.extend(locals);
    }

    pub fn finish(self) -> (wasm_encoder::FuncType, wasm_encoder::Function) {
        let mut locals_vec: Vec<(u32, ValType)> = Vec::new();
        for local_ty in self.locals {
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
        func.instruction(&Instruction::End);
        (self.ty, func)
    }

    /// Helper function. Equivalent to `self.encode_instruction(inst, info).encode(&mut self.code)`.
    pub fn add_instruction(&mut self, inst: &Instruction, info: &ModuleStitchInfo) {
        self.encode_instruction(inst, info).encode(&mut self.code);
    }
    /// Helper function. Equivalent to `inst.encode(&mut self.code)`.
    pub fn add_instruction_raw(&mut self, inst: &Instruction) {
        inst.encode(&mut self.code);
    }

    /// Encodes an instruction. This does not take shims into account; use with caution!
    pub fn encode_instruction<'i, 'a>(
        &self,
        inst: &'i Instruction<'a>,
        info: &ModuleStitchInfo,
    ) -> Cow<'i, Instruction<'a>> {
        use wasm_encoder::Instruction as I;

        let l = |local: u32| local + self.locals_offset();

        let m = |memarg: wasm_encoder::MemArg| wasm_encoder::MemArg {
            memory_index: info.memory_offset,
            ..memarg
        };

        let ty = |ty: u32| ty + info.ty_offset;
        let t = |table: u32| table + info.table_offset;
        let e = |elem: u32| elem + info.elem_offset;
        let f = |func_idx: u32| {
            if let Some(import) = info.get_import_of_func_idx(func_idx) {
                panic!("calling import {import:?} was passed to FunctionStitch::instruction");
            }
            func_idx
                .checked_add_signed(info.func_offset)
                .expect("func offset overflowed. if you don't have 4 billion functions then this means `func_offset < 0 && func_idx < -func_offset`")
        };
        let me = |mem: u32| {
            assert_eq!(mem, 0, "webassembly module uses multiple memories");
            info.memory_offset
        };
        let d = |data: u32| data + info.data_index;

        let modified_inst = match *inst {
            // Call instructions.
            I::Call(func_idx) => I::Call(f(func_idx)),
            I::CallIndirect {
                type_index,
                table_index,
            } => I::CallIndirect {
                type_index: ty(type_index),
                table_index: t(table_index),
            },
            I::ReturnCallRef(func_idx) => I::ReturnCallRef(f(func_idx)),
            I::ReturnCallIndirect {
                type_index,
                table_index,
            } => I::ReturnCallIndirect {
                type_index: ty(type_index),
                table_index: t(table_index),
            },

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
}

#[derive(Clone)]
pub struct ShimInfo {
    pub shim: Rc<dyn Fn(ShimContext)>,
}
impl ShimInfo {
    pub fn new(f: impl Fn(ShimContext) + 'static) -> Self {
        Self { shim: Rc::new(f) }
    }
    /// Makes a `ShimInfo` that does nothing.
    pub fn identity() -> Self {
        Self::new(|ctx| {
            ctx.insert_original_instructions();
        })
    }
    pub fn shim(&self, context: ShimContext) {
        (self.shim)(context)
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

    func: &'a mut FunctionStitch,
    info: &'a ModuleStitchInfo,
}
impl<'a> ShimContext<'a> {
    pub fn new(
        import: CubedawPluginImport,
        prev_instruction: Instruction<'static>,
        current_instruction: Instruction<'static>,
        func: &'a mut FunctionStitch,
        info: &'a ModuleStitchInfo,
    ) -> Self {
        Self {
            import,
            prev_instruction,
            current_instruction,
            func,
            info,
        }
    }
    pub fn import(&self) -> CubedawPluginImport {
        self.import
    }
    /// Replaces the original instructions with those provided.
    pub fn replace(&mut self, iter: impl IntoIterator<Item = Instruction<'static>>) {
        for inst in iter {
            self.func.add_instruction(&inst, self.info);
        }
    }
    /// Replaces the original current instructions with those provided. Leaves the previous instruction unaffected.
    pub fn replace_only_current(&mut self, iter: impl IntoIterator<Item = Instruction<'static>>) {
        for inst in
            std::iter::once(mem::replace(&mut self.prev_instruction, Instruction::End)).chain(iter)
        {
            self.func.add_instruction(&inst, self.info);
        }
    }
    // TODO docs for these

    pub fn insert_original_instructions(mut self) {
        let insts = [
            mem::replace(&mut self.prev_instruction, Instruction::End),
            mem::replace(&mut self.current_instruction, Instruction::End),
        ];
        self.replace(insts);
    }
    pub fn add_instruction(&mut self, inst: Instruction<'static>) {
        self.func.add_instruction(&inst, self.info);
    }
    pub fn add_instruction_raw(&mut self, inst: Instruction<'static>) {
        inst.encode(&mut self.func.code);
    }
}
