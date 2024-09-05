#[cfg(not(target_family = "wasm"))]
mod no {
    compile_error!("Non-WebAssembly targets are not supported");
}

/*
TODO:

- find a way for plugins to render ui (with egui or other)
- use a custom section (#[wasm_custom_section = "..."]) for plugin info (name, license, author, etc.)

*/

#[repr(u32)]
pub enum Attribute {
    Pitch = 1,
}

extern "C" {
    // can't use a static global here. https://github.com/rust-lang/rust/issues/65987#issuecomment-566271861
    pub fn sample_rate() -> u32;
    pub fn input(index: u32) -> f32x16;
    pub fn output(index: u32, val: f32x16);
    pub fn attribute(attr: Attribute, sample: u8) -> f32x16;
}

// TODO possibly turn into proc_macro?
macro_rules! export_plugin {
    ($plugin_name:ident, ) => {};
}
