use std::borrow::Cow;

use wasm_encoder::Instruction;

#[derive(Default, Debug)]
pub struct ModuleStitch {
    pub offset_map: ahash::HashMap<u64, ModuleOffsets>,

    // https://webassembly.github.io/spec/core/binary/modules.html#indices
    pub tys: wasm_encoder::TypeSection,
    pub funcs: wasm_encoder::FunctionSection,
    pub code: wasm_encoder::CodeSection,
    pub tables: wasm_encoder::TableSection,
    pub memories: wasm_encoder::MemorySection,
    pub globals: wasm_encoder::GlobalSection,
    pub elems: wasm_encoder::ElementSection,
    pub datas: wasm_encoder::DataSection,
}

impl ModuleStitch {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn current_offsets(&self) -> ModuleOffsets {
        ModuleOffsets {
            ty_offset: self.tys.len(),
            func_offset: self.funcs.len(),
            table_offset: self.tables.len(),
            memory_offset: self.memories.len(),
            global_offset: self.globals.len(),
            elem_offset: self.elems.len(),
            data_offset: self.datas.len(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ModuleOffsets {
    pub ty_offset: u32,
    pub func_offset: u32,
    pub table_offset: u32,
    pub memory_offset: u32,
    pub global_offset: u32,
    pub elem_offset: u32,
    pub data_offset: u32,
}

#[derive(Default, Debug)]
pub struct FunctionStitch {
    pub locals: Vec<wasm_encoder::ValType>,
    pub code: Vec<u8>,
}

impl FunctionStitch {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn instruction<'i, 'a>(
        &self,
        inst: &'i Instruction<'a>,
        offsets: &ModuleOffsets,
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
        let d = |data: u32| data + offsets.data_offset;

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

            // there are a bunch of atomic instructions and stuff but those aren't allowed in plugins
            // so we don't need to handle them

            // instruction doesn't need to be modified
            ref other => return Cow::Borrowed(other),
        };

        return Cow::Owned(modified_inst);
    }

    pub fn finalize(self) -> wasm_encoder::Function {
        let mut locals_vec: Vec<(u32, wasm_encoder::ValType)> = Vec::new();
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
