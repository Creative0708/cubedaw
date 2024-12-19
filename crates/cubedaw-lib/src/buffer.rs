use std::{fmt, mem, ops};

// TODO figure this out
// fn resize_boxed_slice<T: Zeroable>(t: &mut Box<[T]>, length: usize) {
//     use core::{mem, slice};
//     use std::alloc;

//     let size = mem::size_of::<T>();
//     if size == 0 || length == 0 || t.len() == 0 {
//         // pointer either was already dangling or will be dangling (or both), an allocation or free has to be performed
//         *t = boxed_slice(length);
//     } else {
//         unsafe {
//             let old_ptr = Box::into_raw(mem::replace(t, boxed_slice(0)));

//             let old_layout = alloc::Layout::array::<T>(t.len()).expect("total size exceeds isize::MAX");
//             let new_layout = alloc::Layout::array::<T>(length).expect("total size exceeds isize::MAX");
//             let new_ptr = alloc::realloc(old_ptr, old_layout, new_layout.size())
//         }
//     }
// }

// the internal buffer representation. possibly subject to change in the future
// (i.e. f32 is too imprecise and is changed to an f64)
//
// ...probably not. well, just to be safe i guess
pub type BufferType = f32;

// the _actual_ internal buffer representation
pub type BufferTypeInt = u32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C, align(16))]
/// A fixed grouping of `BufferType`s with stricter alignment. For optimization purposes. What do you mean "prematurely optimizing"?
///
/// This is also used in `cubedaw-plugin` as plugins operate in chunks of 16 `f32`s (4 `f32x4` `v128`s).
///
/// This is _also_ also used as args and state for plugins.
pub struct InternalBufferType(pub [BufferTypeInt; 16]);

const _: () = {
    const fn assert_zeroable_pod<T: bytemuck::Zeroable + bytemuck::Pod>() {}
    assert_zeroable_pod::<BufferTypeInt>();
};
// SAFETY: BufferTypeInt is Zeroable and Pod so [BufferTypeInt; 16] is Zeroable and Pod so InternalBufferType which is repr(C) is Zeroable and Pod.
unsafe impl bytemuck::Zeroable for InternalBufferType {}
unsafe impl bytemuck::Pod for InternalBufferType {}

impl InternalBufferType {
    pub const BYTES: usize = mem::size_of::<Self>();
    /// The number of `BufferType`s that fit in this object. This is always a power of 2.
    pub const N: usize = Self::BYTES / mem::size_of::<BufferType>();

    pub const ZERO: Self = Self([0; Self::N]);

    pub fn splat(val: BufferType) -> Self {
        Self([val.to_bits(); Self::N])
    }
    pub fn as_array(&self) -> &[BufferType; Self::N] {
        bytemuck::must_cast_ref(self)
    }
}
const _: () = assert!(
    InternalBufferType::N.is_power_of_two(),
    "InternalBufferType::N must be a power of 2"
);

#[repr(transparent)]
#[derive(PartialEq, Eq)]
pub struct Buffer([InternalBufferType]);
impl Buffer {
    pub fn new(inner: &[InternalBufferType]) -> &Self {
        assert!(
            inner.len() <= u32::MAX as usize / InternalBufferType::N,
            "buffer length must fit in a u32"
        );
        // SAFETY: Buffer is repr(transparent) and thus has the same layout as [InternalBufferType]
        unsafe { &*(inner as *const [InternalBufferType] as *const Buffer) }
    }
    pub fn new_mut(inner: &mut [InternalBufferType]) -> &mut Self {
        assert!(
            inner.len() <= u32::MAX as usize / InternalBufferType::N,
            "buffer length must fit in a u32"
        );
        // SAFETY: Buffer is repr(transparent) and thus has the same layout as [InternalBufferType]
        unsafe { &mut *(inner as *mut [InternalBufferType] as *mut Buffer) }
    }
    pub fn new_box(inner: Box<[InternalBufferType]>) -> Box<Self> {
        assert!(
            inner.len() <= u32::MAX as usize / InternalBufferType::N,
            "buffer length must fit in a u32"
        );
        // SAFETY: Buffer is repr(transparent) and thus has the same layout as [InternalBufferType]
        unsafe { Box::from_raw(Box::into_raw(inner) as *mut Buffer) }
    }
    /// Creates a `Box<Self>` with `length` elements.
    pub fn new_box_zeroed(length: u32) -> Box<Self> {
        assert!(
            length % InternalBufferType::N as u32 == 0,
            "buffer length {length} must be a multiple of InternalBufferType::N ({})",
            InternalBufferType::N
        );
        Self::new_box(bytemuck::zeroed_slice_box(
            (length / InternalBufferType::N as u32) as usize,
        ))
    }

    pub fn as_internal(&self) -> &[InternalBufferType] {
        &self.0
    }
    pub fn as_internal_mut(&mut self) -> &mut [InternalBufferType] {
        &mut self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::must_cast_slice(&self.0)
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        bytemuck::must_cast_slice_mut(&mut self.0)
    }

    pub fn copy_from(&mut self, that: &Buffer) {
        self.copy_from_slice(that);
    }
    pub fn accumulate(&mut self, that: &Buffer) {
        // TODO accelerate with like simd or something
        debug_assert!(self.len() == that.len(), "buffer length mismatch");

        for (this, that) in self.iter_mut().zip(that.iter()) {
            *this += that;
        }
    }
}
impl Default for &mut Buffer {
    fn default() -> Self {
        Buffer::new_mut(&mut [])
    }
}
impl Default for Box<Buffer> {
    fn default() -> Self {
        Buffer::new_box(Box::new([]))
    }
}
impl ops::Deref for Buffer {
    type Target = [BufferType];
    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice(self.as_internal())
    }
}
impl ops::DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        bytemuck::cast_slice_mut(self.as_internal_mut())
    }
}
impl ops::Index<u32> for Buffer {
    type Output = BufferType;
    fn index(&self, index: u32) -> &Self::Output {
        &(**self)[index as usize]
    }
}
impl ops::IndexMut<u32> for Buffer {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut (**self)[index as usize]
    }
}

impl Clone for Box<Buffer> {
    fn clone(&self) -> Self {
        let b: Box<[InternalBufferType]> = self.0.into();
        Buffer::new_box(b)
    }
}

impl From<&'_ Buffer> for Box<Buffer> {
    fn from(value: &Buffer) -> Self {
        Buffer::new_box(Box::<[InternalBufferType]>::from(value.as_internal()))
    }
}
impl From<&'_ [u8]> for Box<Buffer> {
    fn from(value: &'_ [u8]) -> Self {
        let rounded_up_size =
            value.len().div_ceil(InternalBufferType::BYTES) * InternalBufferType::BYTES;
        let mut this = Buffer::new_box_zeroed(
            rounded_up_size
                .try_into()
                .expect("buffer length does not fit in u32"),
        );
        this.as_bytes_mut()[..value.len()].copy_from_slice(value);
        this
    }
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.len() <= 4 {
            f.debug_tuple("Buffer").field(&&**self).finish()
        } else {
            // replace with `field_with` when it's stabilized
            // https://github.com/rust-lang/rust/issues/117729
            struct DebugListHelper<'a>(&'a Buffer);
            impl<'a> std::fmt::Debug for DebugListHelper<'a> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    let list: &[f32] = self.0;
                    if list.len() <= 16 {
                        list.fmt(f)
                    } else {
                        let (min, max, sum) = list
                            .iter()
                            .copied()
                            .fold((0.0f32, 0.0f32, 0.0f32), |(min, max, sum), val| {
                                (min.min(val), max.max(val), sum + val)
                            });
                        // uiua-like list formatting
                        // https://www.uiua.org/pad?src=0_14_0-dev_6__4oehMTAwMDAwCg==
                        write!(
                            f,
                            "[{}: {:.3}-{:.3} ~{:.3}]",
                            list.len(),
                            min,
                            max,
                            sum / list.len() as f32
                        )
                    }
                }
            }
            f.debug_tuple("Buffer")
                .field(&DebugListHelper(self))
                .finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Buffer, InternalBufferType};

    #[test]
    fn test_buffer() {
        let mut buf = Buffer::new_box_zeroed(64);
        assert_eq!(buf.len(), 64);
        (**buf)[0..3].copy_from_slice(&[1.0, 2.0, 42.42]);

        // this probably doesn't work on big endian but whatever
        assert_eq!(
            buf.as_bytes()[0..12],
            [0, 0, 128, 63, 0, 0, 0, 64, 20, 174, 41, 66],
            "if you're on big-endian and this fails go scream at one of the maintainers (jk)"
        );

        buf.as_internal_mut()[1] = InternalBufferType::splat(4200.1234);
        assert_eq!((**buf)[16..32], [4200.1234f32; InternalBufferType::N]);
    }
}
