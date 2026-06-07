//! Local Thing Description Directory capabilities.
//!
//! These items are usable with `no_std + alloc`. They provide protocol-neutral
//! directory behavior and deterministic allocation-backed local TD storage.

pub use crate::directory::{
    BorrowedDirectoryEntry, DirectoryEntry, DirectoryPage, InMemoryThingDirectory, ThingDirectory,
};
pub use crate::error::{DiscoveryError, DiscoveryResult};
pub use crate::query::{DirectoryQuery, QueryFilter, QueryPredicate};
