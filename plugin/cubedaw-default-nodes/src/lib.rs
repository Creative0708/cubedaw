#![no_std]
#![feature(portable_simd)]
#![feature(const_fn_floating_point_arithmetic)]

use core::arch::wasm32 as wasm;

// mod math;
mod nodes;
mod util;

#[cfg(not(test))]
#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
    wasm::unreachable()
}

cubedaw_pluginlib::declare_plugin!(
    id: "cubedaw",
    name: "Cubedaw Default Plugins",
    description: "Default plugins for Cubedaw",
);
