#![no_std]
#![feature(portable_simd)]
#![feature(const_fn_floating_point_arithmetic)]

use core::arch::wasm32 as wasm;

mod math;
mod nodes;
mod util;

#[cfg(not(test))]
#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
    wasm::unreachable()
}

#[link_section = "cubedaw:plugin_version"]
static _VERSION: [u8; 5] = *b"0.1.0";
