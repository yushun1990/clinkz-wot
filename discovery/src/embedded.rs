//! Embedded-ready Discovery APIs.
//!
//! These items are usable with `no_std + alloc`. They describe protocol-neutral
//! directory behavior and provide a deterministic allocation-backed in-memory
//! directory for constrained runtimes that need local TD storage.

pub use crate::directory::{
    BorrowedDirectoryEntry, DirectoryEntry, DirectoryPage, InMemoryThingDirectory, ThingDirectory,
};
pub use crate::error::{DiscoveryError, DiscoveryResult};
pub use crate::query::{DirectoryQuery, QueryFilter, QueryPredicate};
