//! Portable, always-thread-safe interior-mutability lock for the WoT engine.
//!
//! [`WotLock<T>`] is the single lock primitive mandated by the v4.0 baseline
//! (§4.7). It replaces the former three-way `MapLock`/`multithread` split with
//! one always-correct container:
//!
//! | Build | Backing primitive |
//! |---|---|
//! | `std` | `std::sync::RwLock<T>` (reader/writer) |
//! | `no_std` | `critical_section::Mutex<core::cell::RefCell<T>>` (always exclusive) |
//!
//! The handle is itself a cheaply [`Clone`]-able `Arc`-backed wrapper, so the
//! pervasive `Arc<MapLock<T>>` nesting of the prior design collapses to a plain
//! `WotLock<T>`. Critical sections are always short and never span `.await` or
//! handler dispatch.
//!
//! ## Poison handling
//!
//! On `std`, a panicking thread can poison the inner `RwLock`. Every accessor
//! heals the poison when observed (via [`RwLockWriteGuard`]'s
//! `into_inner`/`unwrap_or_else`), so a handler panic never leaves the lock
//! permanently unusable and no public lock-error surface exists.
//! On `no_std` the critical-section backend cannot poison. This honors the
//! baseline §4.2 / AD30 contract: locks stay unpoisoned on every build.

use alloc::sync::Arc;
use core::fmt;

#[cfg(feature = "std")]
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Portable interior-mutability lock, `Arc`-backed and [`Clone`]-able.
///
/// See the module docs for the per-build backing primitive and poison story.
/// All accessors heal poisoning internally and therefore never fail; callers
/// receive the closure's result directly (no `Result` wrapper).
pub struct WotLock<T> {
    inner: Arc<Inner<T>>,
}

#[cfg(feature = "std")]
type Inner<T> = RwLock<T>;
#[cfg(not(feature = "std"))]
type Inner<T> = critical_section::Mutex<core::cell::RefCell<T>>;

impl<T> WotLock<T> {
    /// Creates a new lock wrapping `value`.
    pub fn new(value: T) -> Self {
        #[cfg(feature = "std")]
        {
            Self {
                inner: Arc::new(RwLock::new(value)),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Self {
                inner: Arc::new(critical_section::Mutex::new(core::cell::RefCell::new(
                    value,
                ))),
            }
        }
    }

    /// Acquires the lock, runs `f` with exclusive (`&mut`) access, and
    /// releases. Heals any std poisoning internally; never fails.
    ///
    /// On `no_std` this is a brief critical section (interrupts disabled /
    /// scheduler locked for the duration of `f` only).
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        #[cfg(feature = "std")]
        {
            let mut guard = self.heal_write();
            f(&mut *guard)
        }
        #[cfg(not(feature = "std"))]
        {
            critical_section::with(|cs| {
                let guard = self.inner.borrow(cs);
                let mut mut_ref = guard.borrow_mut();
                f(&mut *mut_ref)
            })
        }
    }

    /// Shared (read) variant of [`with`](Self::with).
    ///
    /// On `std`, concurrent readers proceed in parallel (read lock). On
    /// `no_std` this is still a brief critical section — there is no blocking
    /// `RwLock` for `no_std`, so the read path is serialized like the write
    /// path (baseline §4.7).
    pub fn with_read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        #[cfg(feature = "std")]
        {
            let guard = self.heal_read();
            f(&*guard)
        }
        #[cfg(not(feature = "std"))]
        {
            critical_section::with(|cs| {
                let guard = self.inner.borrow(cs);
                let shared_ref = guard.borrow();
                f(&*shared_ref)
            })
        }
    }

    /// Panic-healing alias of [`with`](Self::with).
    ///
    /// Retained as the documented entry point for paths that must be robust to
    /// a panic in the surrounding code (e.g. around handler dispatch). It has
    /// identical semantics to [`with`](Self::with); the name documents intent.
    pub fn with_recover<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.with(f)
    }

    /// Shared panic-healing alias of [`with_read`](Self::with_read). See
    /// [`with_recover`](Self::with_recover).
    pub fn with_read_recover<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.with_read(f)
    }

    #[cfg(feature = "std")]
    fn heal_write(&self) -> RwLockWriteGuard<'_, T> {
        match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                self.inner.clear_poison();
                poisoned.into_inner()
            }
        }
    }

    #[cfg(feature = "std")]
    fn heal_read(&self) -> RwLockReadGuard<'_, T> {
        match self.inner.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                self.inner.clear_poison();
                poisoned.into_inner()
            }
        }
    }
}

impl<T> Clone for WotLock<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> fmt::Debug for WotLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WotLock").finish_non_exhaustive()
    }
}

impl<T> Default for WotLock<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(test)]
mod tests {
    use super::WotLock;
    use alloc::string::String;

    #[test]
    fn exclusive_access_mutates_value() {
        let lock = WotLock::new(String::from("a"));
        lock.with(|s: &mut String| s.push_str("bc"));
        assert_eq!(lock.with_read(|s: &String| s.clone()), String::from("abc"));
    }

    #[test]
    fn clone_shares_underlying_state() {
        let lock = WotLock::new(5u32);
        let lock2 = lock.clone();
        lock.with(|v: &mut u32| *v += 10);
        assert_eq!(lock2.with_read(|v: &u32| *v), 15);
    }

    #[cfg(feature = "std")]
    #[test]
    fn poison_is_healed_after_panic() {
        let lock = WotLock::new(0u32);
        let panic_lock = lock.clone();
        // A panicking closure must not leave the lock poisoned: the next
        // accessor still observes a usable lock (value unchanged by the
        // panicked call after the `panic` point in this construction).
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            panic_lock.with(|_v: &mut u32| panic!("boom"));
        }));
        assert!(result.is_err());
        // Subsequent access heals the poison and proceeds normally.
        lock.with(|v: &mut u32| *v = 42);
        assert_eq!(lock.with_read(|v: &u32| *v), 42);
    }
}
