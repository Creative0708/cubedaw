use cubedaw_lib::InternalBufferType;

pub mod standalone;

// TODO JIT-compile plugins together
// pub mod stitched;

/// Alignment for pointers passed to plugins, in bytes.
const PLUGIN_ALIGN: u32 = 64;

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Attribute {
    Pitch = 1,
}
impl Attribute {
    pub fn from_int(int: u32) -> Option<Self> {
        Some(match int {
            1 => Self::Pitch,
            _ => return None,
        })
    }
}

pub trait AttributeMap {
    fn attribute(&self, attr: Attribute) -> InternalBufferType;
    fn tick(&mut self) {}
}
impl std::fmt::Debug for dyn AttributeMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("dyn AttributeMap { .. }")
    }
}

pub struct NoopAttributeMap;

impl AttributeMap for NoopAttributeMap {
    fn attribute(&self, _attr: Attribute) -> InternalBufferType {
        InternalBufferType::ZERO
    }
}
