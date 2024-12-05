use std::{fmt, ops};

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
pub type BufferType = f32;

#[repr(align(16))]
#[derive(Clone, Copy, Debug, Default)]
/// A fixed grouping of `BufferType`s with stricter alignment. For optimization purposes. What do you mean "prematurely optimizing"?
///
/// This is also used in `cubedaw-plugin` as plugins operate in chunks of 16 `f32`s (4 `f32x4` `v128`s).
pub struct InternalBufferType(pub [BufferType; 16]);
unsafe impl bytemuck::Zeroable for InternalBufferType {}
unsafe impl bytemuck::Pod for InternalBufferType {}

impl InternalBufferType {
    /// The number of `BufferType`s that fit in this object. This is always a power of 2.
    pub const N: usize = core::mem::size_of::<Self>() / core::mem::size_of::<BufferType>();

    pub fn splat(val: BufferType) -> Self {
        Self([val; Self::N])
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
    use super::boxed_slice;
    use std::hint::black_box;

    #[test]
    #[allow(clippy::unit_arg)]
    fn test_boxed_slice() {
        let mut i32s: Box<[i32]> = boxed_slice(10);
        i32s.fill(42);
        i32s[0] = 43;
        assert!(i32s[9] == 42);
        assert!(i32s[0] == 43);
        assert!(i32s.len() == 10);
        black_box(i32s);

        let nothing: Box<[u64]> = boxed_slice(0);
        assert!(nothing.len() == 0);
        black_box(nothing);

        let mut zsts: Box<[()]> = boxed_slice(usize::MAX);
        assert!(zsts.len() == usize::MAX);
        black_box(zsts[1000]);
        black_box(zsts[1000000]);
        zsts[1] = ();
        black_box(zsts);
    }
}
