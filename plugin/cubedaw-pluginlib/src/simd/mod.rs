use super::wasm::{self, v128};
use core::{fmt, ops};

mod math;

macro_rules! every {
    (wasm::$func:tt, $($arg:ident),+) => {
        Self([
            wasm::$func($($arg.0[0]),+),
            wasm::$func($($arg.0[1]),+),
            wasm::$func($($arg.0[2]),+),
            wasm::$func($($arg.0[3]),+),
        ])
    };
    ($func:tt, $($arg:ident),+) => {
        Self([
            $func($($arg.0[0]),+),
            $func($($arg.0[1]),+),
            $func($($arg.0[2]),+),
            $func($($arg.0[3]),+),
        ])
    };
}
pub(crate) use every;

/// FFI-safe clone of f32x16. Also, uses WebAssembly relaxed SIMD and has approximations for common operations.
///
/// ...Okay, _technically_ `v128` isn't currently FFI-safe but in the latest Rust compiler it compiles to a WebAssembly `v128` primitive
/// and I don't see a reason for it to change from that.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct f32x16(pub [v128; 4]);

impl f32x16 {
    pub fn splat(val: f32) -> Self {
        let inner = wasm::f32x4_splat(val);
        Self([inner; 4])
    }

    /// Extracts a lane from 0 to 15. If the index is out of range it is wrapped.
    pub fn extract(self, mut index: u8) -> f32 {
        index &= 0xF;

        let f32x4 = self.0[index as usize >> 2];
        match index & 0x3 {
            0 => wasm::f32x4_extract_lane::<0>(f32x4),
            1 => wasm::f32x4_extract_lane::<1>(f32x4),
            2 => wasm::f32x4_extract_lane::<2>(f32x4),
            3 => wasm::f32x4_extract_lane::<3>(f32x4),
            _ => unreachable!(),
        }
    }
    /// Replaces a lane from 0 to 15. If the index is out of range it is wrapped.
    pub fn replace(&mut self, mut index: u8, val: f32) {
        index &= 0xF;

        let f32x4 = &mut self.0[index as usize >> 2];
        match index & 0x3 {
            0 => *f32x4 = wasm::f32x4_replace_lane::<0>(*f32x4, val),
            1 => *f32x4 = wasm::f32x4_replace_lane::<1>(*f32x4, val),
            2 => *f32x4 = wasm::f32x4_replace_lane::<2>(*f32x4, val),
            3 => *f32x4 = wasm::f32x4_replace_lane::<3>(*f32x4, val),
            _ => unreachable!(),
        }
    }

    pub fn to_array(self) -> [f32; 16] {
        // SAFETY: f32 is valid for all bit patterns. Probably.
        unsafe { core::mem::transmute::<[v128; 4], [f32; 16]>(self.0) }
    }
    pub fn from_array(val: [f32; 16]) -> Self {
        // SAFETY: v128 is valid for all bit patterns.
        Self(unsafe { core::mem::transmute::<[f32; 16], [v128; 4]>(val) })
    }

    #[cfg(feature = "portable_simd")]
    pub fn to_array_f32x4(self) -> [core::simd::f32x4; 4] {
        // SAFETY: [f32; 16] and [[f32; 4]; 4] have the same layout
        let arrays = unsafe { core::mem::transmute::<[f32; 16], [[f32; 4]; 4]>(self.to_array()) };

        [
            core::simd::f32x4::from_array(arrays[0]),
            core::simd::f32x4::from_array(arrays[1]),
            core::simd::f32x4::from_array(arrays[2]),
            core::simd::f32x4::from_array(arrays[3]),
        ]
    }

    // Wrapper functions

    pub fn madd(self, mul: Self, add: Self) -> Self {
        every!(wasm::f32x4_relaxed_madd, self, mul, add)
    }
    pub fn nmadd(self, mul: Self, add: Self) -> Self {
        every!(wasm::f32x4_relaxed_nmadd, self, mul, add)
    }
    pub fn min(self, rhs: Self) -> Self {
        every!(wasm::f32x4_relaxed_min, self, rhs)
    }
    pub fn max(self, rhs: Self) -> Self {
        every!(wasm::f32x4_relaxed_max, self, rhs)
    }
    pub fn trunc(self) -> Self {
        every!(wasm::f32x4_trunc, self)
    }
    pub fn fract(self) -> Self {
        self - self.trunc()
    }
    pub fn abs(self) -> Self {
        every!(wasm::f32x4_abs, self)
    }
    pub fn floor(self) -> Self {
        every!(wasm::f32x4_floor, self)
    }
    pub fn ceil(self) -> Self {
        every!(wasm::f32x4_ceil, self)
    }

    // Utility functions

    pub fn prefix_sum_with(self, start: f32) -> Self {
        let mut new = self;
        let mut tot = start;
        // go go gadget loop unroller
        for i in 0..16 {
            tot += new.extract(i);
            new.replace(i, tot);
        }
        new
    }
    pub fn copysign(self, other: Self) -> Self {
        fn copysign_inner(val: v128, other: v128) -> v128 {
            wasm::v128_bitselect(val, other, wasm::u32x4_splat(0x7fffffff))
        }
        every!(copysign_inner, self, other)
    }
}

impl ops::Add<Self> for f32x16 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        every!(wasm::f32x4_add, self, rhs)
    }
}
impl ops::Sub<Self> for f32x16 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        every!(wasm::f32x4_sub, self, rhs)
    }
}
impl ops::Mul<Self> for f32x16 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        every!(wasm::f32x4_mul, self, rhs)
    }
}
impl ops::Div<Self> for f32x16 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        every!(wasm::f32x4_div, self, rhs)
    }
}
impl ops::AddAssign<Self> for f32x16 {
    fn add_assign(&mut self, rhs: Self) {
        *self = every!(wasm::f32x4_add, self, rhs)
    }
}
impl ops::SubAssign<Self> for f32x16 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = every!(wasm::f32x4_sub, self, rhs)
    }
}
impl ops::MulAssign<Self> for f32x16 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = every!(wasm::f32x4_mul, self, rhs)
    }
}
impl ops::DivAssign<Self> for f32x16 {
    fn div_assign(&mut self, rhs: Self) {
        *self = every!(wasm::f32x4_div, self, rhs);
    }
}
impl fmt::Debug for f32x16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("f32x16").field(&self.to_array()).finish()
    }
}

impl PartialEq for f32x16 {
    fn eq(&self, other: &Self) -> bool {
        wasm::v128_any_true(wasm::f32x4_eq(self.0[0], other.0[0]))
            || wasm::v128_any_true(wasm::f32x4_eq(self.0[1], other.0[1]))
            || wasm::v128_any_true(wasm::f32x4_eq(self.0[2], other.0[2]))
            || wasm::v128_any_true(wasm::f32x4_eq(self.0[3], other.0[3]))
    }
}

#[cfg(feature = "portable_simd")]
mod portable_simd_impls {
    use super::{
        f32x16,
        wasm::{self, v128},
    };
    use core::simd;

    impl From<f32x16> for simd::f32x16 {
        fn from(value: f32x16) -> Self {
            fn unf32x4(val: v128) -> [f32; 4] {
                [
                    wasm::f32x4_extract_lane::<0>(val),
                    wasm::f32x4_extract_lane::<1>(val),
                    wasm::f32x4_extract_lane::<2>(val),
                    wasm::f32x4_extract_lane::<3>(val),
                ]
            }
            // SAFETY: [[f32; 4]; 4] and [f32; 16] have the same layout
            Self::from_array(unsafe {
                core::mem::transmute::<[[f32; 4]; 4], [f32; 16]>([
                    unf32x4(value.0[0]),
                    unf32x4(value.0[1]),
                    unf32x4(value.0[2]),
                    unf32x4(value.0[3]),
                ])
            })
        }
    }
    impl From<simd::f32x16> for f32x16 {
        fn from(value: simd::f32x16) -> Self {
            fn f32x4(val: [f32; 4]) -> v128 {
                wasm::f32x4(val[0], val[1], val[2], val[3])
            }
            // SAFETY: [f32; 16] and [[f32; 4]; 4] have the same layout
            let arr = unsafe { core::mem::transmute::<[f32; 16], [[f32; 4]; 4]>(value.to_array()) };
            Self([f32x4(arr[0]), f32x4(arr[1]), f32x4(arr[2]), f32x4(arr[3])])
        }
    }
}
