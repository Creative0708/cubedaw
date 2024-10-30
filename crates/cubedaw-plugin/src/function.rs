use wasm_encoder::reencode::Reencode;

use super::{
    instructions::PreparedInstructionList,
    stitch::{FunctionStitch, ModuleStitchInfo},
};

// TODO possibly store a semi-complete byte representation for function instructions
// for optimization purposes?
#[derive(Clone)]
pub struct PreparedFunction {
    ty: u32,
    locals: Box<[wasm_encoder::ValType]>,
    instructions: PreparedInstructionList,
}

impl PreparedFunction {
    pub fn new(
        ty: u32,
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
            ty,
            locals: locals.into_boxed_slice(),
            instructions: PreparedInstructionList::new(reader.get_operators_reader()?, reencoder)?,
        })
    }

    pub fn ty(&self) -> u32 {
        self.ty
    }

    /// Adds the locals and code of this function into `instructions_sink`.
    /// This assumes that the arguments are already on the stack, and will put the results on the stack.
    pub fn stitch(&self, func: &mut FunctionStitch, offsets: &ModuleStitchInfo) {
        self.instructions.stitch(func, offsets);
        func.locals.extend(self.locals.iter().cloned());
    }

    pub fn encode_empty(&self, offsets: &ModuleStitchInfo) -> wasm_encoder::Function {
        let mut func = FunctionStitch::empty();
        self.stitch(&mut func, offsets);
        func.finalize()
    }
}
