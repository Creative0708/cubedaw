#![no_std]
#![cfg_attr(feature = "portable_simd", feature(portable_simd))]
#![feature(stdarch_wasm_relaxed_simd)]

use core::arch::wasm32 as wasm;

mod simd;
pub use simd::f32x16;

/*
TODO:

- find a way for plugins to render ui (with egui or other)
- use a custom section (#[wasm_custom_section = "..."]) for plugin info (name, license, author, etc.)

*/

#[repr(u32)]
pub enum Attribute {
    Pitch = 1,
}

#[cfg(not(test))]
#[allow(clippy::items_after_test_module)]
mod ffi {
    use crate::f32x16;

    #[allow(improper_ctypes)]
    extern "C" {
        pub fn sample_rate() -> u32;
        pub fn input(index: u32) -> f32x16;
        pub fn output(index: u32, val: f32x16);
        pub fn attribute(attr: super::Attribute) -> f32x16;
    }
}
#[cfg(test)]
// Use a fake FFI to not get import errors when wasmtiming the tests.
#[allow(improper_ctypes_definitions)]
#[allow(clippy::items_after_test_module)]
mod ffi {
    use crate::f32x16;

    pub unsafe extern "C" fn sample_rate() -> u32 {
        44100
    }
    pub unsafe extern "C" fn input(_index: u32) -> f32x16 {
        f32x16::splat(42.0)
    }
    pub unsafe extern "C" fn output(_index: u32, _val: f32x16) {}
    pub unsafe extern "C" fn attribute(_attr: super::Attribute) -> f32x16 {
        f32x16::splat(42.0)
    }
}

#[inline(always)]
pub fn sample_rate() -> u32 {
    unsafe { ffi::sample_rate() }
}
#[inline(always)]
pub fn input(index: u32) -> f32x16 {
    unsafe { ffi::input(index) }
}
#[inline(always)]
pub fn output(index: u32, val: f32x16) {
    unsafe { ffi::output(index, val) }
}
#[inline(always)]
pub fn attribute(attr: Attribute) -> f32x16 {
    unsafe { ffi::attribute(attr) }
}
