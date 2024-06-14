use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

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

#[cfg(test)]
mod tests {
    use super::boxed_slice;

    #[test]
    fn test_boxed_slice() {
        let mut i32s: Box<[i32]> = boxed_slice(10);
        i32s.fill(42);
        i32s[0] = 43;
        assert!(i32s[9] == 42);
        assert!(i32s[0] == 43);
        assert!(i32s.len() == 10);
        core::hint::black_box(i32s);

        let nothing: Box<[u64]> = boxed_slice(0);
        assert!(nothing.len() == 0);
        core::hint::black_box(nothing);

        let mut zsts: Box<[()]> = boxed_slice(usize::MAX);
        assert!(zsts.len() == usize::MAX);
        zsts[1000];
        zsts[1000000];
        zsts[1] = ();
        core::hint::black_box(zsts);
    }

    #[test]
    #[should_panic]
    fn test_boxed_slice_memory_limit() {
        let mut x: Box<[u32]> = boxed_slice(1_000_000_000_000_000);
        x[0] = 1;
        core::hint::black_box(x);
    }
}

// the internal buffer representation. possibly subject to change in the future
// (i.e. i find out that f32 is too imprecise and change it to an f64)
pub type BufferType = f32;

#[derive(Clone)]
pub struct Buffer(Box<[f32]>);
impl Buffer {
    pub fn new(length: usize) -> Self {
        Self(boxed_slice(length))
    }
    pub fn resize(&mut self, length: u32) {
        if self.0.len() != length as usize {
            self.0 = boxed_slice(length as usize);
        }
    }
}
impl Default for Buffer {
    fn default() -> Self {
        Self::new(0)
    }
}
impl Deref for Buffer {
    type Target = [BufferType];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
