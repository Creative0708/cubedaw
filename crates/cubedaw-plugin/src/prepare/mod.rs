mod function;
mod instructions;

pub use function::PreparedFunction;
pub use instructions::PreparedInstructionList;

use crate::CubedawPluginImport;

pub struct PrepareContext {
    pub import_function_indices: [Option<u32>; CubedawPluginImport::SIZE],
}
