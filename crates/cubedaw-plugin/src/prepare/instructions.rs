use wasm_encoder::{reencode, Encode};

use crate::{
    stitch::{FunctionStitch, ModuleStitchInfo},
    CubedawPluginImport,
};

use super::PrepareContext;

// Used in functions and ConstExprs.
#[derive(Clone, Debug)]
pub struct PreparedInstructionList {
    import_function_indices: [Option<u32>; CubedawPluginImport::SIZE],
    instructions: Box<[wasm_encoder::Instruction<'static>]>,
    special_instructions: Box<[(u32, CubedawPluginImport)]>,
}
impl PreparedInstructionList {
    pub fn new<'a, E>(
        ctx: &PrepareContext,
        operator: impl IntoIterator<Item = Result<wasmparser::Operator<'a>, E>>,
        reencoder: &mut reencode::RoundtripReencoder,
    ) -> Result<Self, E> {
        let mut instructions = operator
            .into_iter()
            .map(|result| {
                result.map(|op| {
                    reencode_instruction_to_static(reencoder, op)
                        .expect("reencoder failed to encode instruction")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        // the End instruction should be left out of ConstExprs but wasmparser includes it for some reason
        if matches!(instructions.last(), Some(wasm_encoder::Instruction::End)) {
            instructions.pop();
        }
        let import_function_indices = ctx.import_function_indices;

        let mut special_instructions = Vec::new();
        for (instruction_idx, instruction) in instructions.iter().enumerate() {
            match instruction {
                wasm_encoder::Instruction::Call(func_idx) => {
                    if let Some(import) = import_function_indices
                        .iter()
                        .zip(CubedawPluginImport::ALL)
                        .find_map(|(idx, import)| (Some(*func_idx) == *idx).then_some(import))
                    {
                        special_instructions.push((instruction_idx as u32, import));
                    }
                }
                _ => (),
            }
        }
        Ok(Self {
            import_function_indices,
            instructions: instructions.into_boxed_slice(),
            special_instructions: special_instructions.into_boxed_slice(),
        })
    }
    /// Convenience function.
    pub fn from_constexpr(
        ctx: &PrepareContext,
        expr: &wasmparser::ConstExpr,
    ) -> Result<Self, wasmparser::BinaryReaderError> {
        Self::new(
            ctx,
            expr.get_operators_reader().into_iter(),
            &mut wasm_encoder::reencode::RoundtripReencoder,
        )
    }

    pub fn stitch(&self, func: &mut FunctionStitch, info: &ModuleStitchInfo) {
        let mut special_iter = self.special_instructions.iter().peekable();
        let mut instructions_iter = self.instructions.iter().enumerate().peekable();
        while let Some((instruction_idx, inst)) = instructions_iter.peek().copied() {
            'add_instructions: {
                if let Some((special_idx, plugin_import)) = special_iter.peek().copied().copied() {
                    debug_assert!(instruction_idx <= special_idx as usize, "next special_idx is before the instruction index, meaning it'll never get picked. did you forget to call special_iter.next()?");

                    if instruction_idx + 1 == special_idx as usize {
                        let (_, prev_instruction) = instructions_iter
                            .next()
                            .expect("peek() returned Some(_) so i'm not sure why next() didn't");
                        let (_, current_instruction) = instructions_iter
                            .next()
                            .expect("instruction at special_idx doesn't exist :/");
                        info.shim_info.shim(crate::ShimContext::new(
                            plugin_import,
                            prev_instruction.clone(),
                            current_instruction.clone(),
                            func,
                            info,
                        ));

                        special_iter.next();
                        break 'add_instructions;
                    }
                }

                func.add_instruction(&inst, info);
                instructions_iter.next();
            }
        }
    }
    pub fn encode(&self, info: &ModuleStitchInfo) -> wasm_encoder::ConstExpr {
        let mut func = FunctionStitch::empty();
        self.stitch(&mut func, info);
        wasm_encoder::ConstExpr::raw(func.code)
    }
}

// clone of wasm_encoder::reencode::Reencode::instruction but reencoding to Instruction<'static> instead
fn reencode_instruction_to_static<T: ?Sized + reencode::Reencode>(
    reencoder: &mut T,
    arg: wasmparser::Operator,
) -> Result<wasm_encoder::Instruction<'static>, reencode::Error<T::Error>> {
    use wasm_encoder::Instruction;

    macro_rules! translate {
                ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident)*) => {
                    Ok(match arg {
                        $(
                            wasmparser::Operator::$op $({ $($arg),* })? => {
                                $(
                                    $(let $arg = translate!(map $arg $arg);)*
                                )?
                                translate!(build $op $($($arg)*)?)
                            }
                        )*
                    })
                };

                // This case is used to map, based on the name of the field, from the
                // wasmparser payload type to the wasm-encoder payload type through
                // `Translator` as applicable.
                (map $arg:ident tag_index) => (reencoder.tag_index($arg));
                (map $arg:ident function_index) => (reencoder.function_index($arg));
                (map $arg:ident table) => (reencoder.table_index($arg));
                (map $arg:ident table_index) => (reencoder.table_index($arg));
                (map $arg:ident dst_table) => (reencoder.table_index($arg));
                (map $arg:ident src_table) => (reencoder.table_index($arg));
                (map $arg:ident type_index) => (reencoder.type_index($arg));
                (map $arg:ident array_type_index) => (reencoder.type_index($arg));
                (map $arg:ident array_type_index_dst) => (reencoder.type_index($arg));
                (map $arg:ident array_type_index_src) => (reencoder.type_index($arg));
                (map $arg:ident struct_type_index) => (reencoder.type_index($arg));
                (map $arg:ident global_index) => (reencoder.global_index($arg));
                (map $arg:ident mem) => (reencoder.memory_index($arg));
                (map $arg:ident src_mem) => (reencoder.memory_index($arg));
                (map $arg:ident dst_mem) => (reencoder.memory_index($arg));
                (map $arg:ident data_index) => (reencoder.data_index($arg));
                (map $arg:ident elem_index) => (reencoder.element_index($arg));
                (map $arg:ident array_data_index) => (reencoder.data_index($arg));
                (map $arg:ident array_elem_index) => (reencoder.element_index($arg));
                (map $arg:ident blockty) => (reencoder.block_type($arg)?);
                (map $arg:ident relative_depth) => ($arg);
                (map $arg:ident targets) => ((
                    $arg
                        .targets()
                        .collect::<Result<Vec<_>, wasmparser::BinaryReaderError>>()?
                        .into(),
                    $arg.default(),
                ));
                (map $arg:ident ty) => (reencoder.val_type($arg)?);
                (map $arg:ident hty) => (reencoder.heap_type($arg)?);
                (map $arg:ident from_ref_type) => (reencoder.ref_type($arg)?);
                (map $arg:ident to_ref_type) => (reencoder.ref_type($arg)?);
                (map $arg:ident memarg) => (reencoder.mem_arg($arg));
                (map $arg:ident ordering) => (reencoder.ordering($arg));
                (map $arg:ident local_index) => ($arg);
                (map $arg:ident value) => ($arg);
                (map $arg:ident lane) => ($arg);
                (map $arg:ident lanes) => ($arg);
                (map $arg:ident array_size) => ($arg);
                (map $arg:ident field_index) => ($arg);
                (map $arg:ident try_table) => ($arg);

                // This case takes the arguments of a wasmparser instruction and creates
                // a wasm-encoder instruction. There are a few special cases for where
                // the structure of a wasmparser instruction differs from that of
                // wasm-encoder.
                (build $op:ident) => (Instruction::$op);
                (build BrTable $arg:ident) => (Instruction::BrTable($arg.0, $arg.1));
                (build I32Const $arg:ident) => (Instruction::I32Const($arg));
                (build I64Const $arg:ident) => (Instruction::I64Const($arg));
                (build F32Const $arg:ident) => (Instruction::F32Const(f32::from_bits($arg.bits())));
                (build F64Const $arg:ident) => (Instruction::F64Const(f64::from_bits($arg.bits())));
                (build V128Const $arg:ident) => (Instruction::V128Const($arg.i128()));
                (build TryTable $table:ident) => (Instruction::TryTable(reencoder.block_type($table.ty)?, {
                    $table.catches.into_iter().map(|c| reencoder.catch(c)).collect::<Vec<_>>().into()
                }));
                (build $op:ident $arg:ident) => (Instruction::$op($arg));
                (build $op:ident $($arg:ident)*) => (Instruction::$op { $($arg),* });
            }

    wasmparser::for_each_operator!(translate)
}
