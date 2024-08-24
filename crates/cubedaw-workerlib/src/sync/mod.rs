use std::{
    cell::UnsafeCell,
    sync::{Arc, Condvar, Mutex},
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
///
pub struct SyncBuffer<T> {
    inner: Arc<SyncBufferInner<T>>,
}

struct SyncBufferInner<T> {
    mutex: Mutex<(usize, UnsafeCell<T>)>,
    condvar: Condvar,
}

impl<T> SyncBuffer<T> {
    pub fn new(t: T) -> Self {
        Self {
            inner: Arc::new(SyncBufferInner {
                mutex: Mutex::new((0, UnsafeCell::new(t))),
                condvar: Condvar::new(),
            }),
        }
    }
    pub fn get_read_handle(&self) -> SyncAccessibleReadHandle<T> {
        SyncAccessibleReadHandle {
            inner: self.inner.clone(),
        }
    }
    // TODO: currently get_write_handle can cause unsoundness:
    // ```
    // let blah: SyncBuffer<Box<u8>> = SyncBuffer::new(Box::new(42));
    // let read = blah.get_read_handle();
    // let ref_to_box = read.wait(); // works because no writers currently
    //
    // let write = blah.get_write_handle(); // !!!
    // write.lock(|b| { *b = Box::new(43); });
    //
    // dbg!(ref_to_box); // UAF
    // ```
    // this will only happen if
    // a) `SyncAccessibleReadHandle` is wait()ed before a call to get_write_handle (which is not the intended usecase)
    // b) the user intentionally does this to prove a point
    //
    // ideally this would be a safe function. how can this be made safe?
    /// # SAFETY
    /// All invocations of `get_write_handle()` have to occur before any associated `SyncAccessibleReadHandle::wait()` invocations.
    ///
    pub unsafe fn get_write_handle(&self) -> SyncAccessibleWriteHandle<T> {
        let mut lock = self.inner.mutex.lock().expect("mutex poisoned");
        lock.0 = {
            let (inc, overflow) = lock.0.overflowing_add(1);
            assert!(
                !overflow,
                "get_write_handle() called >usize::MAX times before a reset. how the heck"
            );
            inc
        };
        SyncAccessibleWriteHandle {
            inner: self.inner.clone(),
        }
    }

    pub fn reset(&mut self) -> &mut T {
        assert!(
            Arc::weak_count(&mut self.inner) == 0,
            "reset() called on incomplete SyncBuffer"
        );
        let inner = Arc::get_mut(&mut self.inner).expect("reset() called on incomplete SyncBuffer");
        let pair = inner.mutex.get_mut().expect("mutex poisoned");
        pair.0 = 0;
        pair.1.get_mut()
    }
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SyncBuffer<u32>>();
};

pub struct SyncAccessibleReadHandle<T> {
    inner: Arc<SyncBufferInner<T>>,
}
impl<T> SyncAccessibleReadHandle<T> {
    pub fn wait(&self) -> &T {
        let mut lock = self.inner.mutex.lock().expect("mutex poisoned");
        while lock.0 > 0 {
            lock = self.inner.condvar.wait(lock).expect("mutex poisoned");
        }
        // SAFETY: when lock.0 == 0, SyncAccessible won't ever modify the UnsafeCell (unless it's through a mutable reference)
        unsafe { &*lock.1.get() }
    }
}
pub struct SyncAccessibleWriteHandle<T> {
    inner: Arc<SyncBufferInner<T>>,
}
impl<T> SyncAccessibleWriteHandle<T> {
    pub fn lock(self, f: impl FnOnce(&mut T)) {
        self.lock_with_return(f)
    }
    pub fn lock_with_return<R>(self, f: impl FnOnce(&mut T) -> R) {
        let mut lock = self.inner.mutex.lock().expect("mutex poisoned");

        f(lock.1.get_mut());

        let remaining = {
            lock.0 -= 1;
            lock.0
        };
        drop(lock);
        if remaining == 0 {
            // this was the last writer - notify the readers!
            self.inner.condvar.notify_all();
        }
    }
}
