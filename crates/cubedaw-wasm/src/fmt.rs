impl std::fmt::Debug for crate::Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for crate::Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memory").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for crate::Func {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Func").finish_non_exhaustive()
    }
}
impl<T> std::fmt::Debug for crate::Linker<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Linker").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for crate::Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module").finish_non_exhaustive()
    }
}
impl<T> std::fmt::Debug for crate::Store<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for crate::ExportLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportLocation").finish_non_exhaustive()
    }
}

#[cfg(feature = "v128")]
impl std::fmt::Debug for crate::V128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let u32s = self.as_u32x4();
        f.debug_tuple("V128")
            .field(&format_args!(
                "0x{:08x}_{:08x}_{:08x}_{:08x}",
                u32s[0], u32s[1], u32s[2], u32s[3]
            ))
            .finish()
    }
}
