#![deny(unsafe_op_in_unsafe_fn)]

// TODO

pub mod ctx;

mod sealed {
    pub trait Sealed {}
}
pub(crate) use sealed::Sealed;
