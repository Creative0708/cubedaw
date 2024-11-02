cfg_if::cfg_if! {
    if #[cfg(feature = "runtime")] {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                pub mod web;
                use web as variable;
            } else {
                pub mod native;
                use native as variable;

                // TODO not do this
                pub use wasmtime;
            }
        }
        pub use variable::{
            Engine, ExportLocation, Func, Instance, Linker, Memory, MemoryAccessError, Module, Store,
        };
    }
}
cfg_if::cfg_if! {
    if #[cfg(feature = "v128")] {
        mod v128;
        pub use v128::V128;
    }
}

#[doc(hidden)]
pub use paste as __paste;

mod config;
pub use config::{WasmConfig, WasmFeatures};

#[cfg(feature = "fmt")]
mod fmt;
#[cfg(feature = "wasmparser")]
mod wasmparser;

#[derive(Clone, Debug)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128([u8; 16]),
}

#[derive(Clone, Debug)]
pub struct FuncType {}
