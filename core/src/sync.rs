//! cfg-selected interior-mutability lock primitive shared across the engine.
//!
//! [`MapLock<T>`] selects a backing primitive per build:
//!
//! | Build | Backing primitive | Multi-thread safe |
//! |---|---|---|
//! | `std` | `std::sync::RwLock` | yes (reader/writer) |
//! | `no_std` + `multithread` feature | `UnsafeCell` + `critical_section::with` | yes (RTOS / multi-interrupt) |
//! | `no_std` (default) | `core::cell::RefCell` | no (single-thread only) |
//!
//! Critical sections are always short and never span `.await` or handler
//! dispatch. Read-mostly state should prefer
//! [`with_read`](MapLock::with_read)/[`with_read_recover`](MapLock::with_read_recover)
//! so concurrent readers do not serialize.

// ---------------------------------------------------------------------------
// Backing primitive selection.
// ---------------------------------------------------------------------------

#[cfg(all(not(feature = "std"), not(feature = "multithread")))]
use core::cell::RefCell;

#[cfg(feature = "std")]
use std::sync::{RwLock, TryLockError};

#[cfg(all(not(feature = "std"), feature = "multithread"))]
use core::cell::UnsafeCell;

use core::fmt;

/// Error returned when a [`MapLock`] was poisoned by a panicking thread.
///
/// Only produced on the `std` build; on `no_std` the backing [`RefCell`]
/// panics on re-entrancy instead of poisoning, and `critical_section::Mutex`
/// cannot poison. The poison is healed when observed, so at most one
/// [`MapLock::with`] call after each panic observes this error (subsequent
/// calls succeed once the lock is healed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapLockError(());

impl MapLockError {
    #[cfg(feature = "std")]
    fn new() -> Self {
        Self(())
    }
}

impl fmt::Display for MapLockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("clinkz-wot engine lock poisoned")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MapLockError {}

// ---------------------------------------------------------------------------
// MapLock — three cfg-selected backends.
// ---------------------------------------------------------------------------

/// Interior-mutability wrapper with three cfg-selected backends:
///
/// - `std`: `std::sync::RwLock` (multi-thread, reader/writer, reports
///   poisoning). Read-mostly state should use
///   [`with_read`](Self::with_read)/[`with_read_recover`](Self::with_read_recover)
///   so concurrent readers proceed in parallel.
/// - `no_std` + `critical-section`: `critical_section::Mutex` (multi-thread
///   safe via critical sections; no poisoning; always exclusive).
/// - `no_std` (default): `core::cell::RefCell` (single-thread, zero-cost,
///   panics on re-entrancy; reads use shared borrows).
#[cfg(all(not(feature = "std"), not(feature = "multithread")))]
pub struct MapLock<T> {
    inner: RefCell<T>,
}

/// `multithread` backend: `UnsafeCell` guarded by
/// `critical_section::with`. All access goes through the critical section,
/// which disables interrupts / locks the scheduler — providing mutual
/// exclusion across threads and interrupt contexts.
#[cfg(all(not(feature = "std"), feature = "multithread"))]
pub struct MapLock<T> {
    inner: UnsafeCell<T>,
}

// Safety: all mutable access goes through `critical_section::with`, which
// provides mutual exclusion across all execution contexts. The critical
// section is never held across `.await` or blocking calls.
#[cfg(all(not(feature = "std"), feature = "multithread"))]
unsafe impl<T> Sync for MapLock<T> {}

#[cfg(feature = "std")]
pub struct MapLock<T> {
    inner: RwLock<T>,
}

impl<T> MapLock<T> {
    /// Creates a new lock wrapping `value`.
    pub fn new(value: T) -> Self {
        #[cfg(all(not(feature = "std"), not(feature = "multithread")))]
        {
            Self {
                inner: RefCell::new(value),
            }
        }
        #[cfg(all(not(feature = "std"), feature = "multithread"))]
        {
            Self {
                inner: UnsafeCell::new(value),
            }
        }
        #[cfg(feature = "std")]
        {
            Self {
                inner: RwLock::new(value),
            }
        }
    }

    /// Acquires the lock, runs `f` with exclusive access, and releases.
    ///
    /// Returns [`Err`] if the lock was poisoned by a panicking thread
    /// (`std` only). On `multithread` and `RefCell` builds this always
    /// returns [`Ok`] (neither can poison).
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> Result<R, MapLockError> {
        #[cfg(all(not(feature = "std"), not(feature = "multithread")))]
        {
            Ok(f(&mut *self.inner.borrow_mut()))
        }
        #[cfg(all(not(feature = "std"), feature = "multithread"))]
        {
            // critical_section provides mutual exclusion; cannot poison.
            Ok(critical_section::with(|_| {
                // Safety: critical section guarantees exclusive access.
                let guard = unsafe { &mut *self.inner.get() };
                f(guard)
            }))
        }
        #[cfg(feature = "std")]
        {
            match self.inner.write() {
                Ok(mut guard) => Ok(f(&mut *guard)),
                Err(_) => {
                    self.inner.clear_poison();
                    Err(MapLockError::new())
                }
            }
        }
    }

    /// Shared (read) variant of [`with`](Self::with).
    ///
    /// Acquires a **read** lock, runs `f` with shared access, and releases.
    /// Concurrent readers proceed in parallel; only writers block. Returns
    /// [`Err`] if the lock was poisoned by a panicking thread (`std` only).
    ///
    /// Prefer this over [`with`](Self::with) for read-only accessors (cache
    /// lookups, registry `get`, length/empty checks) so read-heavy paths do not
    /// serialize against each other.
    pub fn with_read<R>(&self, f: impl FnOnce(&T) -> R) -> Result<R, MapLockError> {
        #[cfg(all(not(feature = "std"), not(feature = "multithread")))]
        {
            Ok(f(&*self.inner.borrow()))
        }
        #[cfg(all(not(feature = "std"), feature = "multithread"))]
        {
            Ok(critical_section::with(|_| {
                // Safety: critical section guarantees exclusive access.
                let guard = unsafe { &*self.inner.get() };
                f(guard)
            }))
        }
        #[cfg(feature = "std")]
        {
            match self.inner.read() {
                Ok(guard) => Ok(f(&*guard)),
                Err(_) => {
                    self.inner.clear_poison();
                    Err(MapLockError::new())
                }
            }
        }
    }

    /// Best-effort variant of [`with`](Self::with) that recovers from
    /// poisoning.
    ///
    /// Runs `f` on the (possibly inconsistent) data and returns the result.
    /// On the `std` build the poison is healed after recovery so subsequent
    /// [`with`](Self::with) calls succeed. **Reserved for read-only
    /// accessors and teardown paths.** Mutating code that builds live engine
    /// state must use [`with`](Self::with) so poison is not silently applied.
    pub fn with_recover<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        #[cfg(all(not(feature = "std"), not(feature = "multithread")))]
        {
            f(&mut *self.inner.borrow_mut())
        }
        #[cfg(all(not(feature = "std"), feature = "multithread"))]
        {
            critical_section::with(|_| {
                let guard = unsafe { &mut *self.inner.get() };
                f(guard)
            })
        }
        #[cfg(feature = "std")]
        {
            let mut guard = self
                .inner
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let result = f(&mut *guard);
            drop(guard);
            self.inner.clear_poison();
            result
        }
    }

    /// Best-effort shared (read) variant that recovers from poisoning.
    ///
    /// Like [`with_recover`](Self::with_recover) but acquires a **read** lock
    /// and hands `f` a shared reference. Use this for read-only accessors that
    /// should not fail (and should not block writers from other readers).
    pub fn with_read_recover<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        #[cfg(all(not(feature = "std"), not(feature = "multithread")))]
        {
            f(&*self.inner.borrow())
        }
        #[cfg(all(not(feature = "std"), feature = "multithread"))]
        {
            critical_section::with(|_| {
                let guard = unsafe { &*self.inner.get() };
                f(guard)
            })
        }
        #[cfg(feature = "std")]
        {
            let guard = self
                .inner
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let result = f(&*guard);
            drop(guard);
            self.inner.clear_poison();
            result
        }
    }

    /// Non-blocking variant of [`with`](Self::with) that recovers from
    /// poisoning.
    ///
    /// Returns `Some(R)` when the lock was acquired and `f` ran, or `None`
    /// when the lock was already held (`std` only). On `multithread` and
    /// `RefCell` builds this always returns `Some` (neither can fail to
    /// acquire).
    pub fn try_with<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        #[cfg(all(not(feature = "std"), not(feature = "multithread")))]
        {
            match self.inner.try_borrow_mut() {
                Ok(mut guard) => Some(f(&mut *guard)),
                Err(_) => None,
            }
        }
        #[cfg(all(not(feature = "std"), feature = "multithread"))]
        {
            // critical_section always acquires immediately (brief critical
            // section — disable interrupts/scheduler). There is no "try" or
            // "would block"; it always succeeds.
            Some(critical_section::with(|_| {
                let guard = unsafe { &mut *self.inner.get() };
                f(guard)
            }))
        }
        #[cfg(feature = "std")]
        {
            match self.inner.try_write() {
                Ok(mut guard) => Some(f(&mut *guard)),
                Err(TryLockError::WouldBlock) => None,
                Err(TryLockError::Poisoned(p)) => {
                    let mut guard = p.into_inner();
                    Some(f(&mut *guard))
                }
            }
        }
    }
}
