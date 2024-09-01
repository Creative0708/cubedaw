use std::{
    cell::{Cell, UnsafeCell},
    mem::MaybeUninit,
    sync::{Condvar, Mutex},
    usize,
};

/// It's kinda hard to explain.
///
/// Basically, this synchronization helper thing allows for some number of readers and some number of writers.
/// Writers can use a `SyncAccessibleWriteHandle` to access and write to the inner `T`. This consumes the `SyncAccessibleWriteHandle`,
/// so the writer can't write more than once.
///
/// Readers can use a `SyncAccessibleReadHandle` to wait on the SyncBuffer. After all the writers have finished writing (the `SyncAccessibleWriteHandle`s are dropped)
/// the readers can read to the inner T and use it for whatever.
///
/// ```
/// let buf: SyncBuffer<u8, String> = SyncBuffer::new();
///
/// std::thread::scope(|env| {
///     let read = buf.get_read_handle();
///     let write1 = buf.get_write_handle();
///     let write2 = buf.get_write_handle();
///
///     buf.prime("🦀".into());
///
///     env.spawn(|| {
///         let waited: u8 = read.wait();
///         println!("reader got {waited}");
///     });
///
///     env.spawn(|| {
///         let res = write1.lock(|u8: &mut u8| {
///             println!("write1 writing, got {}", *u8);
///             u8 += 50;
///         });
///         println!("write1 done, got {res:?}");
///     });
///     env.spawn(|| {
///         std::thread::sleep(std::time::Duration::from_millis(100));
///
///         let res = write1.lock_with_return(|u8: &mut u8| {
///             println!("write2 writing, got {}", *u8);
///             u8 += 80;
///
///             u8 + 30
///         });
///         println!("write2 done, got {res:?}");
///     });
/// });
/// ```
pub struct SyncBuffer<T, E = ()> {
    num_writers: Cell<usize>,
    inner: SyncBufferInner<T, E>,
}

/// Special value denoting a `SyncBufferInner` that's not available for reading/writing yet.
const UNPRIMED: usize = usize::MAX;

impl<T, E> SyncBuffer<T, E> {
    pub fn new(t: T) -> Self {
        Self {
            num_writers: Cell::new(0),
            inner: SyncBufferInner {
                mutex: Mutex::new((UNPRIMED, UnsafeCell::new(t), MaybeUninit::uninit())),
                condvar: Condvar::new(),
            },
        }
    }
    pub fn get_read_handle(&self) -> SyncAccessibleReadHandle<T, E> {
        if self.num_writers.get() == UNPRIMED {
            panic!("SyncBuffer::get_read_handle called after SyncBuffer::prime");
        }

        SyncAccessibleReadHandle { inner: &self.inner }
    }
    pub fn get_write_handle(&self) -> SyncAccessibleWriteHandle<T, E> {
        if self.num_writers.get() == UNPRIMED {
            panic!("SyncBuffer::get_write_handle called after SyncBuffer::prime");
        }

        self.num_writers.set({
            let num_writers = self.num_writers.get() + 1;
            if num_writers == UNPRIMED {
                panic!("get_write_handle() called usize::MAX times. that's {} times. please call get_write_handle() less times", usize::MAX);
            }
            num_writers
        });
        SyncAccessibleWriteHandle { inner: &self.inner }
    }
    pub fn prime(&self, extra: E) -> Option<E> {
        let mut lock = self.inner.mutex.lock().expect("mutex poisoned");
        lock.0 = self.num_writers.get();
        self.num_writers.set(UNPRIMED);
        if lock.0 == 0 {
            // no writers exist, just return
            Some(extra)
        } else {
            lock.2.write(extra);
            None
        }
    }

    // /// Calls the function with the internal extra data.
    // /// If all writers have written and the extra data has been moved, the function isn't called and `None` is returned.
    // /// Otherwise, this function returns `Some` with the return value.
    // pub fn extra<R>(&self, f: impl FnOnce(&mut E) -> R) -> Option<R> {
    //     let mut lock = self.inner.mutex.lock().expect("mutex poisoned");
    //     if lock.0 == 0 {
    //         return None;
    //     }
    //     Some(f(&mut lock.2))
    // }

    pub fn reset(&mut self) -> &mut T {
        let pair = self.inner.mutex.get_mut().expect("mutex poisoned");
        pair.0 = 0;

        pair.1.get_mut()
    }
}

struct SyncBufferInner<T, E> {
    mutex: Mutex<(usize, UnsafeCell<T>, MaybeUninit<E>)>,
    condvar: Condvar,
}
impl<T, E> Drop for SyncBufferInner<T, E> {
    fn drop(&mut self) {
        let Ok(mutex) = self.mutex.get_mut() else {
            return;
        };
        if mutex.0 != UNPRIMED {
            // SAFETY: the SyncBuffer is primed so the MaybeUninit is init; drop it!
            unsafe {
                mutex.2.assume_init_drop();
            }
        }
    }
}

pub struct SyncAccessibleReadHandle<'a, T, E> {
    inner: &'a SyncBufferInner<T, E>,
}
impl<'a, T, E> SyncAccessibleReadHandle<'a, T, E> {
    pub fn wait(&self) -> &'a T {
        let mut lock = self.inner.mutex.lock().expect("mutex poisoned");
        if lock.0 == UNPRIMED {
            panic!("SyncAccessibleReadHandle::wait called before SyncBuffer::prime");
        }
        while lock.0 > 0 {
            lock = self.inner.condvar.wait(lock).expect("mutex poisoned");
        }
        // SAFETY: when lock.0 == 0, SyncAccessible won't ever modify the UnsafeCell (unless it's through a mutable reference)
        unsafe { &*lock.1.get() }
    }
}
pub struct SyncAccessibleWriteHandle<'a, T, E> {
    inner: &'a SyncBufferInner<T, E>,
}
impl<'a, T, E> SyncAccessibleWriteHandle<'a, T, E> {
    /// Locks the `SyncBuffer` and writes to it.
    /// If this was the last write handle, returns `Some(extra)` with the extra
    pub fn lock(self, f: impl FnOnce(&mut T)) -> Option<E> {
        self.lock_with_return(f).1
    }

    /// Locks the `SyncBuffer` and writes to it, returning a value as well.
    /// If this was the last write handle, returns `Some(extra)` with the extra
    pub fn lock_with_return<R>(self, f: impl FnOnce(&mut T) -> R) -> (R, Option<E>) {
        let mut lock = self.inner.mutex.lock().expect("mutex poisoned");

        if lock.0 == UNPRIMED {
            panic!("SyncAccessibleWriteHandle::lock called before SyncBuffer::prime");
        }

        let return_val = f(lock.1.get_mut());

        let remaining = {
            lock.0 -= 1;
            lock.0
        };
        let extra = if remaining == 0 {
            // SAFETY: since lock.0 != UNPRIMED (the SyncBuffer is primed), lock.2 is init.
            // Additionally, lock.2 isn't read again until a `reset()` call.
            let extra = unsafe { lock.2.assume_init_read() };
            drop(lock);
            self.inner.condvar.notify_all();

            Some(extra)
        } else {
            None
        };

        (return_val, extra)
    }
}

impl<'a, T, E> std::fmt::Debug for SyncAccessibleReadHandle<'a, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO
        f.write_str("SyncAccessibleReadHandle { .. }")
        // let mut d = f.debug_struct("SyncAccessibleReadHandle");
        // match self.inner.mutex.lock() {
        //     Ok(guard) => match guard.0 {
        //         0 => d.field("data", unsafe {
        //             guard.1.
        //         }),
        //         UNPRIMED
        //     },
        // }
        // <Mutex<()> as std::fmt::Debug>::fmt(&self, f)
    }
}

impl<'a, T, E> std::fmt::Debug for SyncAccessibleWriteHandle<'a, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO
        f.write_str("SyncAccessibleWriteHandle { .. }")
        // let mut d = f.debug_struct("SyncAccessibleReadHandle");
        // match self.inner.mutex.lock() {
        //     Ok(guard) => match guard.0 {
        //         0 => d.field("data", unsafe {
        //             guard.1.
        //         }),
        //         UNPRIMED
        //     },
        // }
        // <Mutex<()> as std::fmt::Debug>::fmt(&self, f)
    }
}

const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<SyncBuffer<u32, ()>>();
    assert_send::<SyncAccessibleReadHandle<'static, u32, ()>>();
    assert_send::<SyncAccessibleWriteHandle<'static, u32, ()>>();
};
