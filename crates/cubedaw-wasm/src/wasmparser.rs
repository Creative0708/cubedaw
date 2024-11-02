use crate::WasmFeatures;

impl From<WasmFeatures> for wasmparser::WasmFeatures {
    fn from(value: WasmFeatures) -> Self {
        // these features are always enabled on any semi-modern WASM engine
        let mut features = Self::empty()
            | Self::SATURATING_FLOAT_TO_INT
            | Self::SIGN_EXTENSION
            | Self::MUTABLE_GLOBAL
            | Self::FLOATS;
        if value.contains(WasmFeatures::TAIL_CALL) {
            features.insert(Self::TAIL_CALL);
        }
        if value.contains(WasmFeatures::CUSTOM_PAGE_SIZES) {
            features.insert(Self::CUSTOM_PAGE_SIZES);
        }
        if value.contains(WasmFeatures::THREADS) {
            features.insert(Self::THREADS);
        }
        if value.contains(WasmFeatures::REFERENCE_TYPES) {
            features.insert(Self::REFERENCE_TYPES);
        }
        if value.contains(WasmFeatures::FUNCTION_REFERENCES) {
            features.insert(Self::FUNCTION_REFERENCES);
        }
        if value.contains(WasmFeatures::GC) {
            features.insert(Self::GC);
        }
        if value.contains(WasmFeatures::SIMD) {
            features.insert(Self::SIMD);
        }
        if value.contains(WasmFeatures::RELAXED_SIMD) {
            features.insert(Self::RELAXED_SIMD);
        }
        if value.contains(WasmFeatures::BULK_MEMORY) {
            features.insert(Self::BULK_MEMORY);
        }
        if value.contains(WasmFeatures::MULTI_VALUE) {
            features.insert(Self::MULTI_VALUE);
        }
        if value.contains(WasmFeatures::MULTI_MEMORY) {
            features.insert(Self::MULTI_MEMORY);
        }
        if value.contains(WasmFeatures::MEMORY64) {
            features.insert(Self::MEMORY64);
        }
        if value.contains(WasmFeatures::EXTENDED_CONST) {
            features.insert(Self::EXTENDED_CONST);
        }
        features
    }
}
