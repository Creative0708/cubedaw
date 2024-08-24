// wow, so much code here

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
    pub static SAMPLE_RATE: u32;
    pub fn input(index: u32) -> f32;
    pub fn output(index: u32, sample: f32);
    pub fn attribute(attr: Attribute) -> f32;
}

// TODO possibly turn into proc_macro?
macro_rules! export_plugin {
    ($plugin_name:ident, ) => {};
}
