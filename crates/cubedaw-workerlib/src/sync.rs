use std::{
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Condvar, Mutex, MutexGuard, OnceLock,
    },
};

use cubedaw_lib::Buffer;

pub struct SyncCumulativeBuffer {
    locked_inner: OnceLock<Buffer>,
    unlocked_inner: Mutex<Buffer>,

    writes_remaining: AtomicUsize,
    condvar_mutex: Mutex<()>,
    condvar: Condvar,
}

impl SyncCumulativeBuffer {
    pub fn new(length: usize) -> Self {
        Self {
            locked_inner: OnceLock::new(),
            unlocked_inner: Mutex::new(Buffer::new(length)),

            writes_remaining: AtomicUsize::new(0),
            condvar_mutex: Mutex::new(()),
            condvar: Condvar::new(),
        }
    }
    pub fn wait(&self) -> &Buffer {
        loop {
            if let Some(t) = self.locked_inner.get() {
                return t;
            }
            *self
                .condvar
                .wait(self.condvar_mutex.lock().expect("mutex poisoned"))
                .expect("condvar poisoned");
        }
    }
    // TODO not a good idea to allow a thread to lock the same SyncCumulativeBuffer twice. how fix?!?!!
    pub fn lock(&self) -> SyncCumulativeBufferGuard<'_> {
        SyncCumulativeBufferGuard {
            locked_inner: &self.locked_inner,
            writes_remaining: &self.writes_remaining,
            condvar: &self.condvar,
            mutex_guard: self.unlocked_inner.lock().expect("mutex poisoned"),
        }
    }

    pub fn start(&mut self, writes_remaining: usize) {
        *self.writes_remaining.get_mut() = writes_remaining;
    }
    pub fn reset(&mut self) -> &mut Buffer {
        let buffer = self
            .locked_inner
            .take()
            .expect("reset() called on incomplete SyncCumulativeBuffer");
        let unlocked_inner = self.unlocked_inner.get_mut().expect("mutex is poisoned");
        *unlocked_inner = buffer;
        unlocked_inner
    }
}

pub struct SyncCumulativeBufferGuard<'a> {
    locked_inner: &'a OnceLock<Buffer>,
    writes_remaining: &'a AtomicUsize,
    condvar: &'a Condvar,
    mutex_guard: MutexGuard<'a, Buffer>,
}
impl Deref for SyncCumulativeBufferGuard<'_> {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        self.mutex_guard.deref()
    }
}
impl DerefMut for SyncCumulativeBufferGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutex_guard.deref_mut()
    }
}
impl Drop for SyncCumulativeBufferGuard<'_> {
    fn drop(&mut self) {
        let fetched = self.writes_remaining.fetch_sub(1, Ordering::Relaxed);
        if fetched == 1 {
            self.locked_inner
                .set(core::mem::take(&mut self.mutex_guard))
                .unwrap_or_else(|_| panic!("oncelock poisoned"));
            self.condvar.notify_all();
        } else if fetched == 0 {
            // panic and poison the mutex so no further processing can happen
            panic!("SyncCumulativeBufferGuard tried to decrement writes_remaining below 0");
        }
    }
}
