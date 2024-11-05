#![no_std]

use core::arch::wasm32 as wasm;

use cubedaw_pluginlib::{f32x16, input, output, sample_rate};

#[cfg(not(test))]
#[panic_handler]
fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
    wasm::unreachable()
}

cubedaw_pluginlib::declare_plugin!(
    id: "test",
    name: "Cubedaw Test Plugin",
    description: "testing testing 123",
);

#[repr(C)]
pub enum TestPluginArgs {
    Add = 0,
    Multiply = 1,
    SampleRate = 2,
}
#[repr(C)]
pub struct TestPluginState {
    val: f32x16,
}

#[no_mangle]
pub extern "C" fn test_plugin(args: &TestPluginArgs, state: &mut TestPluginState) {
    let i0 = input::<0>();
    let i1 = input::<1>();
    match args {
        TestPluginArgs::Add => state.val = i0 + i1,
        TestPluginArgs::Multiply => state.val = i0 * i1,
        TestPluginArgs::SampleRate => state.val = f32x16::splat(sample_rate() as f32),
    }
    output::<0>(state.val);
}

cubedaw_pluginlib::export_node!("test:test", test_plugin);

#[no_mangle]
pub extern "C" fn add(_: *const (), _: *mut ()) {
    let i0 = input::<0>();
    let i1 = input::<1>();
    output::<0>(i0 + i1);
}
#[no_mangle]
pub extern "C" fn mul(_: *const (), _: *mut ()) {
    let i0 = input::<0>();
    let i1 = input::<1>();
    output::<0>(i0 * i1);
}
cubedaw_pluginlib::export_node!("test:add", add);
cubedaw_pluginlib::export_node!("test:mul", mul);
