//! Protocol-neutral Discovery and Thing Description Directory utilities.
//!
//! This crate starts with a deterministic in-memory directory backend that can
//! be used by tests, host runtimes, and the future Servient composition layer.

pub mod directory;
pub mod error;
pub mod query;

pub use directory::{
    BorrowedDirectoryEntry, DirectoryEntry, DirectoryPage, InMemoryThingDirectory, ThingDirectory,
};
pub use error::{DiscoveryError, DiscoveryResult};
pub use query::{DirectoryQuery, QueryFilter, QueryPredicate};
