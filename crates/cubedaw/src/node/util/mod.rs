use std::ptr::NonNull;

use bytemuck::Zeroable;

fn boxed_slice<T: Zeroable>(length: usize) -> Box<[T]> {
    use core::{mem, slice};
    use std::alloc::Layout;

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
            let layout = Layout::from_size_align_unchecked(size * length, mem::align_of::<T>());
            let ptr = std::alloc::alloc_zeroed(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }

            Box::from_raw(slice::from_raw_parts_mut(ptr as *mut _, length))
        }
    }
}

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
        let mut x: Box<[u32]> = boxed_slice(1_000_000_000_000_000_000);
        x[0] = 1;
        core::hint::black_box(x);
    }
}

pub struct Buffer(Box<[f32]>);
impl Buffer {
    pub fn new() -> Self {
        Self(boxed_slice(0))
    }
    pub fn resize_and_get_mut(&mut self, size: u32) -> &mut [f32] {
        if self.0.len() != size as usize {
            self.0 = boxed_slice(size as usize);
        }
        &mut self.0
    }
}
