//! Shared storage adapters for `std` Discovery runtimes.
//!
//! Production storage backends that need filesystems, databases, sockets, or
//! async runtimes should live behind `std` features or in separate crates while
//! implementing the protocol-neutral [`ThingDirectory`] trait.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::{DiscoveryError, DiscoveryResult};

/// Shareable Thing Directory handle for `std` runtimes.
///
/// This wrapper lets runtime composition share one directory backend across
/// services without requiring a concrete storage backend type at every call
/// site.
#[derive(Debug)]
pub struct SharedThingDirectory<D> {
    inner: Arc<Mutex<D>>,
}

impl<D> SharedThingDirectory<D> {
    /// Creates a shared directory handle from a concrete backend.
    pub fn new(directory: D) -> Self {
        Self {
            inner: Arc::new(Mutex::new(directory)),
        }
    }

    /// Creates a shared directory handle from an existing `Arc<Mutex<D>>`.
    pub fn from_arc(inner: Arc<Mutex<D>>) -> Self {
        Self { inner }
    }

    /// Returns the underlying shared directory container.
    pub fn inner(&self) -> &Arc<Mutex<D>> {
        &self.inner
    }

    /// Locks the shared directory backend.
    ///
    /// Callers keep control over which directory operations are executed while
    /// lock poisoning remains an explicit error.
    pub fn lock(&self) -> DiscoveryResult<MutexGuard<'_, D>> {
        self.inner
            .lock()
            .map_err(|_| DiscoveryError::SharedDirectoryLockPoisoned)
    }
}

impl<D> Clone for SharedThingDirectory<D> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
