use std::{borrow, fmt, ops, ptr::NonNull};

use bytemuck::Zeroable;

fn boxed_slice<T: Zeroable>(length: usize) -> Box<[T]> {
    use core::{mem, slice};
    use std::alloc;

    let size = mem::size_of::<T>();
    if size == 0 || length == 0 {
        unsafe {
            Box::from_raw(slice::from_raw_parts_mut(
                NonNull::dangling().as_ptr(),
                length,
            ))
        }
    } else {
        unsafe {
            let layout = alloc::Layout::array::<T>(length).expect("total size exceeds isize::MAX");
            let ptr = alloc::alloc_zeroed(layout);
            if ptr.is_null() {
                alloc::handle_alloc_error(layout);
            }

            Box::from_raw(slice::from_raw_parts_mut(ptr as *mut _, length))
        }
    }
}

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

pub struct Buffer<'a>(&'a mut [BufferType]);
impl<'a> Buffer<'a> {
    pub fn new(inner: &'a mut [BufferType]) -> Self {
        assert!(
            inner.len() <= u32::MAX as usize,
            "buffer length must fit in a u32"
        );
        Self(inner)
    }
    pub fn len(&self) -> u32 {
        self.0.len() as u32
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn as_slice(&self) -> &[BufferType] {
        self.0
    }
    pub fn as_slice_mut(&mut self) -> &mut [BufferType] {
        self.0
    }

    pub fn reborrow(&mut self) -> Buffer<'_> {
        Buffer(self.0)
    }

    pub fn accumulate(&mut self, that: &Buffer) {
        // TODO accelerate with like simd or something
        debug_assert!(self.len() == that.len(), "buffer length mismatch");

        for (this, that) in self.0.iter_mut().zip(that.0.iter()) {
            *this += that;
        }
    }
}
impl<'a> Default for Buffer<'a> {
    fn default() -> Self {
        Self::new(&mut [])
    }
}
impl<'a> ops::Index<u32> for Buffer<'a> {
    type Output = BufferType;
    fn index(&self, index: u32) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl<'a> ops::IndexMut<u32> for Buffer<'a> {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}
impl<'a> fmt::Debug for Buffer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.len() <= 4 {
            write!(f, "Buffer {{ {:?} }}", &self.0)
        } else {
            write!(
                f,
                "Buffer {{ [{:?}, {:?}, {:?}, ... length {}] }}",
                self.0[0],
                self.0[1],
                self.0[2],
                self.0.len()
            )
        }
    }
}

#[derive(Clone)]
pub struct BufferOwned(Box<[BufferType]>);
impl BufferOwned {
    pub fn new(inner: Box<[BufferType]>) -> Self {
        assert!(
            inner.len() <= u32::MAX as usize,
            "buffer length must fit in a u32"
        );
        Self(inner)
    }
    pub fn zeroed(len: u32) -> Self {
        Self(boxed_slice(len as usize))
    }

    pub fn len(&self) -> u32 {
        self.0.len() as u32
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn as_slice(&self) -> &[BufferType] {
        &self.0
    }
    pub fn as_slice_mut(&mut self) -> &mut [BufferType] {
        &mut self.0
    }

    pub fn borrow_mut(&mut self) -> Buffer {
        Buffer::new(&mut self.0)
    }

    pub fn accumulate(&mut self, that: &Self) {
        // TODO accelerate with like simd or something
        debug_assert!(self.len() == that.len(), "buffer length mismatch")
    }
}
impl<'a> Default for BufferOwned {
    fn default() -> Self {
        Self::new(Box::new([]))
    }
}
impl ops::Index<u32> for BufferOwned {
    type Output = BufferType;
    fn index(&self, index: u32) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl ops::IndexMut<u32> for BufferOwned {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}
impl fmt::Debug for BufferOwned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.len() <= 4 {
            write!(f, "BufferOwned {{ {:?} }}", &self.0)
        } else {
            write!(
                f,
                "BufferOwned {{ [{:?}, {:?}, {:?}, ... length {}] }}",
                self.0[0],
                self.0[1],
                self.0[2],
                self.0.len()
            )
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

    #[test]
    #[should_panic]
    fn test_boxed_slice_memory_limit() {
        let mut x: Box<[u32]> = boxed_slice(1_000_000_000_000_000);
        x[0] = 1;
        black_box(x);
    }
}
