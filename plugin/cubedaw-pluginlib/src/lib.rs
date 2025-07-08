#![no_std]
#![cfg_attr(feature = "portable_simd", feature(portable_simd))]
#![feature(adt_const_params)]
#![feature(stdarch_wasm_relaxed_simd)]
#![feature(const_fn_floating_point_arithmetic)]
// v128 (and thus f32x16) are not technically ffi-safe but i don't see any situation where they would not be
#![allow(improper_ctypes_definitions, improper_ctypes)]
// for adt_const_params; no reason other than "it works right now" :P
#![allow(incomplete_features)]

use core::arch::wasm32 as wasm;
use core::marker::ConstParamTy;

mod simd;
pub use simd::f32x16;
mod macros;

#[doc(hidden)]
pub use macros::{__paste, __postcard_stringify};

/*
TODO:

- find a way for plugins to render ui (with egui or other)

*/

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub enum Attribute {
    Pitch = 1,
}

#[cfg(not(test))]
#[allow(clippy::items_after_test_module)]
mod ffi {
    use crate::f32x16;

    extern "C" {
        pub fn sample_rate() -> u32;
        pub fn input(index: u32) -> f32x16;
        pub fn output(val: f32x16, index: u32);
        pub fn attribute(attr: super::Attribute) -> f32x16;
        pub fn kick();
    }
}
#[cfg(test)]
// Use a fake FFI to not get import errors when wasmtiming the tests
#[allow(clippy::items_after_test_module)]
mod ffi {
    use crate::f32x16;

    pub unsafe extern "C" fn sample_rate() -> u32 {
        44100
    }
    pub unsafe extern "C" fn input(_index: u32) -> f32x16 {
        f32x16::splat(42.0)
    }
    pub unsafe extern "C" fn output(_val: f32x16, _index: u32) {}
    pub unsafe extern "C" fn attribute(_attr: super::Attribute) -> f32x16 {
        f32x16::splat(42.0)
    }
    pub unsafe extern "C" fn kick() {}
}

#[inline(always)]
pub fn sample_rate() -> u32 {
    unsafe { ffi::sample_rate() }
}

macro_rules! conditional_inline {
    ($($args:tt)*) => {
        #[cfg_attr(not(feature = "inline"), inline(never))]
        #[cfg_attr(feature = "inline", inline(always))]
        $($args)*
    }
}

// #[inline(never)] is required here because each `call` instruction must be
// preceded by a `i32.const`. Passing an i32 as a parameter could lead to
// it being modifiable at runtime which would be terrible to figure out in cubedaw-plugin.
//
// ...but also sometimes we get lucky and rust doesn't generate variable-parameter function calls.
// so as a compromise we can use a crate feature to disable noinline.
conditional_inline!(
    pub extern "C" fn input<const INDEX: u32>() -> f32x16 {
        unsafe { ffi::input(INDEX) }
    }
);
conditional_inline!(
    pub extern "C" fn output<const INDEX: u32>(val: f32x16) {
        unsafe { ffi::output(val, INDEX) }
    }
);
conditional_inline!(
    pub extern "C" fn attribute<const ATTR: Attribute>() -> f32x16 {
        unsafe { ffi::attribute(ATTR) }
    }
);
