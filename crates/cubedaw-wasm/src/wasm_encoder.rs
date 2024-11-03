use crate::{FuncType, ValType};

impl From<wasm_encoder::ValType> for ValType {
    fn from(value: wasm_encoder::ValType) -> Self {
        use wasm_encoder::ValType as V;
        match value {
            V::I32 => Self::I32,
            V::I64 => Self::I64,
            V::F32 => Self::F32,
            V::F64 => Self::F64,
            V::V128 => Self::V128,
            other => panic!("wasm_encoder type {other:?} not supported"),
        }
    }
}
impl From<ValType> for wasm_encoder::ValType {
    fn from(value: ValType) -> Self {
        use ValType as V;
        match value {
            V::I32 => Self::I32,
            V::I64 => Self::I64,
            V::F32 => Self::F32,
            V::F64 => Self::F64,
            V::V128 => Self::V128,
        }
    }
}

impl From<wasm_encoder::FuncType> for FuncType {
    fn from(value: wasm_encoder::FuncType) -> Self {
        Self::new(
            value.params().iter().cloned().map(Into::into),
            value.results().iter().cloned().map(Into::into),
        )
    }
}
impl From<FuncType> for wasm_encoder::FuncType {
    fn from(value: FuncType) -> Self {
        Self::new(
            value.params().iter().cloned().map(Into::into),
            value.results().iter().cloned().map(Into::into),
        )
    }
}
