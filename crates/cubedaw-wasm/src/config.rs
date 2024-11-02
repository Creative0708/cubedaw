bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct WasmFeatures: u32 {
        const TAIL_CALL = 1 << 0;
        const CUSTOM_PAGE_SIZES = 1 << 1;
        const THREADS = 1 << 2;
        const REFERENCE_TYPES = 1 << 3;
        const FUNCTION_REFERENCES = 1 << 4;
        const GC = 1 << 5;
        const SIMD = 1 << 6;
        const RELAXED_SIMD = 1 << 7;
        const BULK_MEMORY = 1 << 8;
        const MULTI_VALUE = 1 << 9;
        const MULTI_MEMORY = 1 << 10;
        const MEMORY64 = 1 << 11;
        const EXTENDED_CONST = 1 << 12;
    }
}

#[derive(Default, Clone)]
pub struct WasmConfig {
    pub(crate) features: WasmFeatures,
}
impl WasmConfig {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn features(&self) -> WasmFeatures {
        self.features
    }
    pub fn set_features(mut self, features: WasmFeatures) -> Self {
        self.features = features;
        self
    }
}
