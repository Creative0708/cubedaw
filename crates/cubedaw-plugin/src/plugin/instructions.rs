use wasm_encoder::{reencode, Encode};

use super::stitch::{FunctionStitch, ModuleOffsets};

// Used in functions and ConstExprs.
pub struct PreparedInstructionList {
    instructions: Box<[wasm_encoder::Instruction<'static>]>,
}
impl PreparedInstructionList {
    pub fn new<'a, E>(
        operator: impl IntoIterator<Item = Result<wasmparser::Operator<'a>, E>>,
        reencoder: &mut reencode::RoundtripReencoder,
    ) -> Result<Self, E> {
        Ok(Self {
            instructions: operator
                .into_iter()
                .map(|result| {
                    result.map(|op| {
                        reencode_instruction_to_static(reencoder, op)
                            .expect("reencoder failed to encode instruction")
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        })
    }

    pub fn stitch(&self, func: &mut FunctionStitch, offsets: &ModuleOffsets) {
        for inst in self.instructions.iter() {
            let modified = func.instruction(inst, offsets);
            modified.encode(&mut func.code);
        }
    }
    pub fn encode(&self, offsets: &ModuleOffsets) -> wasm_encoder::ConstExpr {
        let mut func = FunctionStitch::new();
        self.stitch(&mut func, offsets);
        wasm_encoder::ConstExpr::raw(func.code)
    }
}

impl<'a> TryFrom<wasmparser::ConstExpr<'a>> for PreparedInstructionList {
    type Error = wasmparser::BinaryReaderError;
    fn try_from(expr: wasmparser::ConstExpr) -> Result<Self, Self::Error> {
        Self::new(
            expr.get_operators_reader().into_iter(),
            &mut wasm_encoder::reencode::RoundtripReencoder,
        )
    }
}

// clone of wasm_encoder::reencode::Reencode::instruction but reencoding to static instead
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
