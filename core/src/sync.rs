//! cfg-selected interior-mutability lock primitive shared across the engine.
//!
//! [`MapLock<T>`] selects [`core::cell::RefCell`] on the `no_std` build and
//! [`std::sync::Mutex`] on `std`. It provides [`with`](MapLock::with) — acquire,
//! apply a closure, release — and [`try_with`](MapLock::try_with) — a
//! non-blocking variant.
//!
//! Critical sections are always short and never span `.await` or handler
//! dispatch.

#[cfg(not(feature = "std"))]
use core::cell::RefCell;
#[cfg(feature = "std")]
use std::sync::{Mutex, TryLockError};

/// cfg-selected interior-mutability wrapper.
///
/// On the `no_std` build this is a `RefCell` (single-threaded, zero-cost); on
/// `std` it is a `Mutex` (multi-threaded, brief hold).
#[cfg(not(feature = "std"))]
pub struct MapLock<T> {
    inner: RefCell<T>,
}

#[cfg(feature = "std")]
pub struct MapLock<T> {
    inner: Mutex<T>,
}

impl<T> MapLock<T> {
    /// Creates a new lock wrapping `value`.
    pub fn new(value: T) -> Self {
        #[cfg(not(feature = "std"))]
        {
            Self {
                inner: RefCell::new(value),
            }
        }
        #[cfg(feature = "std")]
        {
            Self {
                inner: Mutex::new(value),
            }
        }
    }

    /// Acquires the lock, runs `f` with exclusive access, and releases.
    ///
    /// On the `no_std` build this panics on a double borrow (re-entrancy); on
    /// `std` it blocks until the lock is free. Callers must never hold another
    /// lock that would cause re-entrancy or deadlock.
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        #[cfg(not(feature = "std"))]
        {
            f(&mut *self.inner.borrow_mut())
        }
        #[cfg(feature = "std")]
        {
            f(&mut *self.inner.lock().expect("clinkz-wot engine lock poisoned"))
        }
    }

    /// Non-blocking variant of [`with`](Self::with).
    ///
    /// Returns `Some(R)` when the lock was acquired and `f` ran, or `None` when
    /// the lock was already held.
    pub fn try_with<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        #[cfg(not(feature = "std"))]
        {
            match self.inner.try_borrow_mut() {
                Ok(mut guard) => Some(f(&mut *guard)),
                Err(_) => None,
            }
        }
        #[cfg(feature = "std")]
        {
            match self.inner.try_lock() {
                Ok(mut guard) => Some(f(&mut *guard)),
                Err(TryLockError::WouldBlock) => None,
                Err(TryLockError::Poisoned(p)) => Some(f(&mut *p.into_inner())),
            }
        }
    }
}
