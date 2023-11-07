#![feature(negative_impls)]

use std::{
    cell::UnsafeCell,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr::{addr_of, addr_of_mut},
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};

use libc::{
    pthread_mutex_destroy, pthread_mutex_init, pthread_mutex_lock, pthread_mutex_t,
    pthread_mutex_unlock, pthread_mutexattr_init, pthread_mutexattr_setpshared, pthread_self,
    PTHREAD_PROCESS_SHARED,
};

/// This might not work in signal handlers/interrupts.
pub struct ReentrantSharedMutex<T> {
    mutex: SharedMutex<T>,
    guard_count: AtomicUsize,

    // We assume that a thread id of 0 is impossible.
    tid: AtomicU64,
}

impl<T> ReentrantSharedMutex<T> {
    pub fn new(inner: T) -> Self {
        Self {
            mutex: SharedMutex::new(inner),
            guard_count: AtomicUsize::new(0),
            tid: AtomicU64::new(0),
        }
    }

    pub fn lock(&self) -> ReentrantSharedMutexGuard<'_, T> {
        let guard = if self.reentrant() {
            unsafe { self.mutex.guard_unchecked() }
        } else {
            let guard = self.mutex.lock();
            self.tid.store(unsafe { pthread_self() }, Ordering::SeqCst);

            ManuallyDrop::new(guard)
        };

        self.guard_count.fetch_add(1, Ordering::SeqCst);

        ReentrantSharedMutexGuard {
            tid: &self.tid,
            guard_count: &self.guard_count,
            inner: guard,
        }
    }

    pub fn reentrant(&self) -> bool {
        self.tid.load(Ordering::SeqCst) == unsafe { pthread_self() }
    }
}

pub struct ReentrantSharedMutexGuard<'a, T> {
    tid: &'a AtomicU64,
    guard_count: &'a AtomicUsize,
    inner: ManuallyDrop<SharedMutexGuard<'a, T>>,
}

impl<'a, T> Deref for ReentrantSharedMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, T> Drop for ReentrantSharedMutexGuard<'a, T> {
    fn drop(&mut self) {
        if self.guard_count.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.tid.store(0, Ordering::SeqCst);

            unsafe { drop(ManuallyDrop::take(&mut self.inner)) };
        }
    }
}

pub struct SharedMutex<T> {
    pmutex: pthread_mutex_t, // perhaps this too should be in an unsafe cell
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for SharedMutex<T> {}

impl<T> SharedMutex<T> {
    pub fn new(inner: T) -> Self {
        let pmutex = unsafe {
            let mut attr = MaybeUninit::uninit();
            assert_eq!(pthread_mutexattr_init(attr.as_mut_ptr()), 0);

            let mut attr = attr.assume_init();
            assert_eq!(
                pthread_mutexattr_setpshared(addr_of_mut!(attr), PTHREAD_PROCESS_SHARED),
                0
            );

            let mut pmutex = MaybeUninit::uninit();
            assert_eq!(pthread_mutex_init(pmutex.as_mut_ptr(), addr_of!(attr)), 0);

            pmutex.assume_init()
        };

        Self {
            pmutex,
            inner: UnsafeCell::new(inner),
        }
    }

    pub fn lock(&self) -> SharedMutexGuard<'_, T> {
        unsafe {
            assert_eq!(pthread_mutex_lock(addr_of!(self.pmutex) as *mut _), 0);
            SharedMutexGuard(&self)
        }
    }

    unsafe fn guard_unchecked(&self) -> ManuallyDrop<SharedMutexGuard<'_, T>> {
        ManuallyDrop::new(SharedMutexGuard(&self))
    }
}

impl<T> Drop for SharedMutex<T> {
    fn drop(&mut self) {
        unsafe {
            assert_eq!(pthread_mutex_destroy(addr_of_mut!(self.pmutex)), 0);
        }
    }
}

pub struct SharedMutexGuard<'a, T>(&'a SharedMutex<T>);

impl<'a, T> !Send for SharedMutexGuard<'a, T> {}

impl<'a, T> Deref for SharedMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.inner.get() }
    }
}

impl<'a, T> DerefMut for SharedMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.inner.get() }
    }
}

impl<'a, T> Drop for SharedMutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            assert_eq!(pthread_mutex_unlock(addr_of!(self.0.pmutex) as *mut _), 0);
        }
    }
}
