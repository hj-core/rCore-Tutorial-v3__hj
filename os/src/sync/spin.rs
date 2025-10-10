#![allow(dead_code)]

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};
use core::{fmt, hint};

pub struct SpinLock<T> {
    is_locked: AtomicBool,
    data: UnsafeCell<T>,
}

// # Safety
//
// It is safe to share references between threads as long
// as T is Send because:
//
// - is_locked can only be modified in a controlled manner.
// - Only one thread at a time can access the data.
unsafe impl<T> Sync for SpinLock<T> where T: Send {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            is_locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinGuard<'_, T> {
        while self
            .is_locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }
        SpinGuard { lock: self }
    }

    fn unlock(&self) {
        self.is_locked.store(false, Ordering::Release);
    }
}

pub struct SpinGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

// # Safety
//
// It is safe to share the references between threads as long
// as T is Sync because:
//
// - We have exclusive access to the data.
// - We cannot access the lock through SpinGuard.
unsafe impl<T> Sync for SpinGuard<'_, T> where T: Sync {}

impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // # Safety
        // The very existence of this Guard guarantees we've
        // exclusively locked the lock.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // # Safety
        // The very existence of this Guard guarantees we've
        // exclusively locked the lock.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

impl<T> fmt::Debug for SpinGuard<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> fmt::Result {
        (self.deref()).fmt(f)
    }
}
