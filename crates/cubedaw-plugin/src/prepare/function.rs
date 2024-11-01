use wasm_encoder::reencode::Reencode;

use crate::{
    prepare::PreparedInstructionList,
    stitch::{FunctionStitch, ModuleStitchInfo},
};

use super::PrepareContext;

// TODO possibly store a semi-complete byte representation for function instructions
// for optimization purposes?
/// A function, ready to be stitched into a `FunctionStitch`.
#[derive(Clone)]
pub struct PreparedFunction {
    ty_index: u32,
    ty: wasm_encoder::FuncType,
    locals: Box<[wasm_encoder::ValType]>,
    instructions: PreparedInstructionList,
}

impl PreparedFunction {
    pub fn new(
        ctx: &PrepareContext,
        ty_index: u32,
        ty: wasm_encoder::FuncType,
        reader: wasmparser::FunctionBody,
        reencoder: &mut wasm_encoder::reencode::RoundtripReencoder,
    ) -> Result<Self, wasmparser::BinaryReaderError> {
        let locals_reader = reader.get_locals_reader()?;
        let mut locals = Vec::new();
        for local in locals_reader.into_iter() {
            let (num, ty) = local?;
            for _ in 0..num {
                locals.push(
                    reencoder
                        .val_type(ty)
                        .expect("non-ref types can't panic when reencoding"),
                );
            }
        }

        Ok(Self {
            ty_index,
            ty,
            locals: locals.into_boxed_slice(),
            instructions: PreparedInstructionList::new(
                ctx,
                reader.get_operators_reader()?,
                reencoder,
            )?,
        })
    }

    pub fn ty(&self) -> u32 {
        self.ty_index
    }

    /// Adds the locals and code of this function into `instructions_sink`.
    /// This assumes that the arguments are already on the stack, and will put the results on the stack.
    pub fn stitch(&self, func: &mut FunctionStitch, offsets: &ModuleStitchInfo) {
        let locals_offset = func.locals_offset();
        // put stuff from the stack into locals
        for local_index in locals_offset..locals_offset + self.ty.params().len() as u32 {
            func.add_instruction_raw(&wasm_encoder::Instruction::LocalSet(local_index));
        }
        self.instructions.stitch(func, offsets);

        // allocate locals for the parameters
        func.extend_locals(self.ty.params().iter().cloned());
        func.extend_locals(self.locals.iter().cloned());
    }

    pub fn encode_standalone(&self, offsets: &ModuleStitchInfo) -> FunctionStitch {
        let mut func = FunctionStitch::standalone(self.ty.clone());
        // no need to do anything fancy here, just encode the locals and instructions without needing extra locals
        self.instructions.stitch(&mut func, offsets);
        func.extend_locals(self.locals.iter().cloned());
        func
    }
}
