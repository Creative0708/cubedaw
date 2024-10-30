use anyhow::Result;

use crate::Value;

#[derive(Default)]
pub struct Engine(wasmtime::Engine);

#[derive(Clone)]
pub struct Module(wasmtime::Module);
impl Module {
    pub fn new(engine: &Engine, buf: &[u8]) -> Result<Self> {
        Ok(Self(wasmtime::Module::new(&engine.0, buf)?))
    }
    pub fn get_export(&self, name: &str) -> Option<ExportLocation> {
        Some(ExportLocation(self.0.get_export_index(name)?))
    }
}

pub struct Linker<T>(wasmtime::Linker<T>);

impl<T> Linker<T> {
    pub fn new(engine: &Engine) -> Self {
        Self(wasmtime::Linker::new(&engine.0))
    }
    pub fn instantiate(&self, store: &mut Store<T>, module: &Module) -> Result<Instance> {
        Ok(Instance(self.0.instantiate(&mut store.0, &module.0)?))
    }
}

#[derive(Clone)]
pub struct ExportLocation(wasmtime::ModuleExport);

pub struct Instance(wasmtime::Instance);
impl Instance {
    fn get_export<T>(
        &self,
        store: &mut Store<T>,
        export: &ExportLocation,
    ) -> Option<wasmtime::Extern> {
        self.0.get_module_export(&mut store.0, &export.0)
    }
    pub fn get_memory<T>(&self, store: &mut Store<T>, export: &ExportLocation) -> Option<Memory> {
        self.get_export(store, export)
            .and_then(|exp| exp.into_memory())
            .map(Memory)
    }
    pub fn get_func<T>(&self, store: &mut Store<T>, export: &ExportLocation) -> Option<Func> {
        self.get_export(store, export)
            .and_then(|exp| exp.into_func())
            .map(Func)
    }
}
impl std::fmt::Debug for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instance").finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct Memory(wasmtime::Memory);
impl Memory {
    pub fn read<T>(
        &self,
        store: &Store<T>,
        offset: u32,
        buffer: &mut [u8],
    ) -> Result<(), MemoryAccessError> {
        self.0
            .read(&store.0, offset as usize, buffer)
            .map_err(MemoryAccessError)
    }
    pub fn write<T>(
        &self,
        store: &mut Store<T>,
        offset: u32,
        buffer: &[u8],
    ) -> Result<(), MemoryAccessError> {
        self.0
            .write(&mut store.0, offset as usize, buffer)
            .map_err(MemoryAccessError)
    }
    /// Grows the memory by a given number of pages.
    pub fn grow<T>(&self, store: &mut Store<T>, pages: u32) -> Result<u64, ()> {
        self.0.grow(&mut store.0, pages as u64).map_err(|_| ())
    }
    pub fn size<T>(&self, store: &Store<T>) -> u32 {
        self.0.size(&store.0) as u32
    }
    pub fn page_size<T>(&self, store: &Store<T>) -> u32 {
        self.0.page_size(&store.0) as u32
    }
    pub fn page_size_log2<T>(&self, store: &Store<T>) -> u32 {
        self.0.page_size_log2(&store.0) as u32
    }
}

pub struct Func(wasmtime::Func);
impl Func {
    pub fn call<T>(
        &self,
        store: &mut Store<T>,
        params: &[Value],
        results: &mut [Value],
    ) -> anyhow::Result<()> {
        let mut vec: smallvec::SmallVec<[wasmtime::Val; 16]> = smallvec::SmallVec::new();
        for _ in 0..results.len() {
            vec.push(wasmtime::Val::null_any_ref());
        }
        for param in params {
            vec.push(param.clone().into());
        }
        let (wasmtime_results, wasmtime_params) = vec.split_at_mut(results.len());
        self.0
            .call(&mut store.0, wasmtime_params, wasmtime_results)?;
        for (wasmtime_result, result) in vec.into_iter().zip(results) {
            *result = wasmtime_result.try_into()?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct MemoryAccessError(wasmtime::MemoryAccessError);
impl std::fmt::Display for MemoryAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for MemoryAccessError {}

pub struct Store<T>(wasmtime::Store<T>);

impl<T> Store<T> {
    pub fn new(engine: &Engine, data: T) -> Self {
        Self(wasmtime::Store::new(&engine.0, data))
    }
    // fn get_memory(&self, &) -> ! {
    //     wasmtime::Instance::new(todo!(), todo!(), &[])
    //         .unwrap()
    //         .get_memory()
    // }
}

impl From<Value> for wasmtime::Val {
    fn from(value: Value) -> Self {
        match value {
            Value::I32(i32) => Self::I32(i32),
            Value::I64(i64) => Self::I64(i64),
            Value::F32(f32) => Self::F32(f32.to_bits()),
            Value::F64(f64) => Self::F64(f64.to_bits()),
            Value::V128(v128) => Self::V128(u128::from_ne_bytes(v128).into()),
        }
    }
}
impl TryFrom<wasmtime::Val> for Value {
    type Error = anyhow::Error;
    fn try_from(value: wasmtime::Val) -> Result<Self, Self::Error> {
        Ok(match value {
            wasmtime::Val::I32(i32) => Self::I32(i32),
            wasmtime::Val::I64(i64) => Self::I64(i64),
            wasmtime::Val::F32(f32) => Self::F32(f32::from_bits(f32)),
            wasmtime::Val::F64(f64) => Self::F64(f64::from_bits(f64)),
            wasmtime::Val::V128(v128) => Self::V128(u128::from(v128).to_ne_bytes()),
            other => anyhow::bail!("can't convert {other:?} to cubedaw_wasm::Value"),
        })
    }
}
