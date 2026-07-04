//! Std-only storage adapters for the in-memory directory (retained behind
//! `std`). A shared directory is an `Arc<InMemoryDirectory>`; the type alias
//! documents the shared-storage idiom for the Servient composition layer.

extern crate alloc;
extern crate std;

use alloc::sync::Arc;

use crate::backend::memory::InMemoryDirectory;

/// A shared, clone-cheap handle to an [`InMemoryDirectory`] for use across
/// async tasks / the Servient. The directory is already `Clone` (`Arc`-backed
/// `WotLock`); this alias documents the shared-storage idiom.
pub type SharedInMemoryDirectory = Arc<InMemoryDirectory>;

/// Creates a fresh shared in-memory directory.
pub fn shared_in_memory_directory() -> SharedInMemoryDirectory {
    Arc::new(InMemoryDirectory::new())
}
