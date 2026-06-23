//! Drain flag for the exposed-Thing registry (baseline v3.0 §7).
//!
//! [`DrainFlag`] is settable without acquiring the per-Thing lock. The shared
//! [`MapLock`] primitive lives in `clinkz-wot-core`.

// ---------------------------------------------------------------------------
// Drain flag — settable without acquiring the per-Thing lock.
// ---------------------------------------------------------------------------

/// Settable flag marking an entry for deferred removal (baseline §7).
///
/// On the sync build this is a `Cell<bool>` (single-threaded, interior
/// mutability through `&self`); on `std` it is an `AtomicBool`. In both cases
/// the flag can be set without acquiring the per-Thing [`MapLock`], which is
/// essential for the `destroy(own_id)`-from-within-handler scenario: the
/// handler already holds the per-Thing lock, so `destroy` cannot acquire it,
/// but it can still set the drain flag.
#[cfg(not(feature = "std"))]
pub(crate) struct DrainFlag {
    inner: core::cell::Cell<bool>,
}

#[cfg(feature = "std")]
pub(crate) struct DrainFlag {
    inner: core::sync::atomic::AtomicBool,
}

impl DrainFlag {
    /// Creates a flag initialized to `false` (not draining).
    pub(crate) fn new() -> Self {
        #[cfg(not(feature = "std"))]
        {
            Self {
                inner: core::cell::Cell::new(false),
            }
        }
        #[cfg(feature = "std")]
        {
            Self {
                inner: core::sync::atomic::AtomicBool::new(false),
            }
        }
    }

    /// Returns whether the entry is marked for removal.
    pub(crate) fn get(&self) -> bool {
        #[cfg(not(feature = "std"))]
        {
            self.inner.get()
        }
        #[cfg(feature = "std")]
        {
            self.inner.load(core::sync::atomic::Ordering::Acquire)
        }
    }

    /// Marks the entry for deferred removal.
    pub(crate) fn set(&self) {
        #[cfg(not(feature = "std"))]
        {
            self.inner.set(true)
        }
        #[cfg(feature = "std")]
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
