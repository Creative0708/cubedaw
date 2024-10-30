#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
use native as variable;

#[cfg(target_arch = "wasm32")]
pub mod web;
#[cfg(target_arch = "wasm32")]
use web as variable;

#[cfg(feature = "fmt")]
mod fmt;

pub use variable::{
    Engine, ExportLocation, Func, Instance, Linker, Memory, MemoryAccessError, Module, Store,
};

#[derive(Clone)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128([u8; 16]),
}
