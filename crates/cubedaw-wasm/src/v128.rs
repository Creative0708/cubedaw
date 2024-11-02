/// WebAssembly `v128` type.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct V128(pub [u64; 2]);

// SAFETY (safeties?): V128 is a repr(transparent) [u64; 2] and thus is Zeroable and Pod.
unsafe impl bytemuck::Zeroable for V128 {}
unsafe impl bytemuck::Pod for V128 {}

macro_rules! impl_arrays {
    (
        $([$ty:ident; $num:literal],)+
    ) => {
        $crate::__paste::paste! {
            impl V128 {
                $(
                    pub const fn [< $ty x $num _splat >](val: $ty) -> Self {
                        Self::from_array([val; $num])
                    }
                    pub const fn [< from_ $ty x $num >](arr: [$ty; $num]) -> Self {
                        Self::from_array(arr)
                    }
                    pub const fn [< into_ $ty x $num >](self) -> [$ty; $num] {
                        self.into_array()
                    }
                    pub const fn [< as_ $ty x $num >](&self) -> &[$ty; $num] {
                        self.as_array()
                    }
                )+
            }
        }
    };
}

impl_arrays!(
    [u8; 16], [i8; 16], [u16; 8], [i16; 8], [u32; 4], [i32; 4], [u64; 2], [i64; 2], [f32; 4],
    [f64; 2],
);

impl V128 {
    pub const ZERO: Self = bytemuck::zeroed();

    pub const fn from_array<T: bytemuck::Pod, const N: usize>(arr: [T; N]) -> Self {
        bytemuck::must_cast(arr)
    }
    pub const fn as_array<T: bytemuck::Pod, const N: usize>(&self) -> &[T; N] {
        bytemuck::must_cast_ref(self)
    }
    pub const fn into_array<T: bytemuck::Pod, const N: usize>(self) -> [T; N] {
        bytemuck::must_cast(self)
    }
}
