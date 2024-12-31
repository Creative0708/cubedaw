use std::num::NonZeroU32;

use crate::registry::NodeRegistry;

mod ui;
use ahash::HashMapExt;
pub use ui::{NodeInputUiOptions, NodeUiContext};

mod impls;
pub mod registry;

pub fn register_builtin_nodes(registry: &mut NodeRegistry) {
    registry.register_node_no_inner(
        resourcekey::literal!("builtin:track_input"),
        "Track Input",
        Box::new(impls::builtin::TrackInputNodeThingy),
    );
    registry.register_node_no_inner(
        resourcekey::literal!("builtin:track_output"),
        "Track Output",
        Box::new(impls::builtin::TrackOutputNodeThingy),
    );
    registry.register_node_no_inner(
        resourcekey::literal!("builtin:note_output"),
        "Note Output",
        Box::new(impls::builtin::NoteInputNodeThingy),
    );
}

pub fn register_cubedaw_nodes(registry: &mut NodeRegistry) {
    registry.register_node(
        resourcekey::literal!("cubedaw:math"),
        "Math",
        impls::MathNode,
    );
    registry.register_node(
        resourcekey::literal!("cubedaw:oscillator"),
        "Math",
        impls::OscillatorNode,
    );
    let inner = std::sync::Arc::get_mut(&mut registry.inner)
        .expect("shared registry passed to register_cubedaw_nodes");

    inner.register_plugin(
        cubedaw_plugin::Plugin::new(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../plugin/target/wasm32-unknown-unknown/release/cubedaw_default_nodes.wasm"
        )))
        .expect("default cubedaw plugins aren't a valid plugin :("),
        &mut registry.dyn_node_factories,
    );
}

// #[derive(Clone, Copy)]
// pub enum DataSource<'a> {
//     Const(BufferType),
//     Buffer(&'a [BufferType]),
// }
// impl std::ops::Index<u32> for DataSource<'_> {
//     type Output = BufferType;
//     fn index(&self, index: u32) -> &Self::Output {
//         match self {
//             Self::Const(val) => val,
//             Self::Buffer(buf) => &buf[index as usize],
//         }
//     }
// }

// pub enum DataDrain<'a> {
//     Disconnected,
//     NodeInput(&'a [std::cell::Cell<BufferType>]),
// }
// impl DataDrain<'_> {
//     pub fn set(&self, i: u32, val: BufferType) {
//         match self {
//             Self::Disconnected => (),
//             Self::NodeInput(buf) => {
//                 buf[i as usize].set(val);
//             }
//         }
//     }
// }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NoteProperty(pub NonZeroU32);
impl NoteProperty {
    pub const fn new(val: u32) -> Option<Self> {
        match NonZeroU32::new(val) {
            None => None,
            Some(val) => Some(Self(val)),
        }
    }
    /// # Safety
    /// `val != 0` otherwise it's UB. You know the drill.
    pub const unsafe fn new_unchecked(val: u32) -> Self {
        Self(unsafe { NonZeroU32::new_unchecked(val) })
    }

    const fn new_or_panic(val: u32) -> Self {
        match Self::new(val) {
            Some(p) => p,
            None => panic!("0 passed to NoteProperty::new_or_panic"),
        }
    }
    pub const PITCH: Self = Self::new_or_panic(1);
    pub const TIME_SINCE_START: Self = Self::new_or_panic(2);
    pub const BEATS_SINCE_START: Self = Self::new_or_panic(3);
}

#[derive(Default)]
pub struct NodeCreationContext<'a> {
    pub alias: Option<std::borrow::Cow<'a, str>>,
}
