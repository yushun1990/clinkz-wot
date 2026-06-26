//! Shared storage adapters for `std` Discovery runtimes.
//!
//! Production storage backends that need filesystems, databases, sockets, or
//! async runtimes should live behind `std` features or in separate crates while
//! implementing the protocol-neutral [`ThingDirectory`] trait.

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{DiscoveryError, DiscoveryResult};

/// Shareable Thing Directory handle for `std` runtimes.
///
/// This wrapper lets runtime composition share one directory backend across
/// services without requiring a concrete storage backend type at every call
/// site.
#[derive(Debug, Clone)]
pub struct SharedThingDirectory<D> {
    inner: Arc<RwLock<D>>,
}

impl<D> SharedThingDirectory<D> {
    /// Creates a shared directory handle from a concrete backend.
    pub fn new(directory: D) -> Self {
        Self {
            inner: Arc::new(RwLock::new(directory)),
        }
    }

    /// Creates a shared directory handle from an existing `Arc<RwLock<D>>`.
    pub fn from_arc(inner: Arc<RwLock<D>>) -> Self {
        Self { inner }
    }

    /// Returns the underlying shared directory container.
    pub fn inner(&self) -> &Arc<RwLock<D>> {
        &self.inner
    }

    /// Acquires a shared read lock on the directory backend.
    ///
    /// Multiple concurrent read locks are allowed so read-only operations do
    /// not serialize against each other. Lock poisoning remains an explicit
    /// error.
    pub fn read(&self) -> DiscoveryResult<RwLockReadGuard<'_, D>> {
        self.inner
            .read()
            .map_err(|_| DiscoveryError::SharedDirectoryLockPoisoned)
    }

    /// Acquires an exclusive write lock on the shared directory backend.
    ///
    /// Callers keep control over which directory operations are executed while
    /// lock poisoning remains an explicit error.
    pub fn lock(&self) -> DiscoveryResult<RwLockWriteGuard<'_, D>> {
        self.inner
            .write()
            .map_err(|_| DiscoveryError::SharedDirectoryLockPoisoned)
    }
}
