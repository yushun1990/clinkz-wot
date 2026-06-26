//! Drain flag for the exposed-Thing registry (baseline v3.0 §7).
//!
//! [`DrainFlag`] is settable without acquiring the per-Thing lock. The shared
//! [`MapLock`] primitive lives in `clinkz-wot-core`.

// ---------------------------------------------------------------------------
// Drain flag — settable without acquiring the per-Thing lock.
// ---------------------------------------------------------------------------

/// Settable flag marking an entry for deferred removal (baseline §7).
///
/// On the `std` and `multithread` builds this is an `AtomicBool`
/// (multi-thread safe); on the default `no_std` build it is a `Cell<bool>`
/// (single-threaded). In both cases the flag can be set without acquiring the
/// per-Thing [`MapLock`], which is essential for the `destroy(own_id)`-from-
/// within-handler scenario: the handler already holds the per-Thing lock, so
/// `destroy` cannot acquire it, but it can still set the drain flag.
#[cfg(all(not(feature = "std"), not(feature = "multithread")))]
pub(crate) struct DrainFlag {
    inner: core::cell::Cell<bool>,
}

#[cfg(any(feature = "std", feature = "multithread"))]
pub(crate) struct DrainFlag {
    inner: core::sync::atomic::AtomicBool,
}

impl DrainFlag {
    /// Creates a flag initialized to `false` (not draining).
    pub(crate) fn new() -> Self {
        #[cfg(all(not(feature = "std"), not(feature = "multithread")))]
        {
            Self {
                inner: core::cell::Cell::new(false),
            }
        }
        #[cfg(any(feature = "std", feature = "multithread"))]
        {
            Self {
                inner: core::sync::atomic::AtomicBool::new(false),
            }
        }
    }

    /// Returns whether the entry is marked for removal.
    pub(crate) fn get(&self) -> bool {
        #[cfg(all(not(feature = "std"), not(feature = "multithread")))]
        {
            self.inner.get()
        }
        #[cfg(any(feature = "std", feature = "multithread"))]
        {
            self.inner.load(core::sync::atomic::Ordering::Acquire)
        }
    }

    /// Marks the entry for deferred removal.
    pub(crate) fn set(&self) {
        #[cfg(all(not(feature = "std"), not(feature = "multithread")))]
        {
            self.inner.set(true)
        }
        #[cfg(any(feature = "std", feature = "multithread"))]
        {
            self.inner
                .store(true, core::sync::atomic::Ordering::Release)
        }
    }
}

impl Default for DrainFlag {
    fn default() -> Self {
        Self::new()
    }
}
