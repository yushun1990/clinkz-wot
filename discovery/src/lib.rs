#![no_std]
//!
//! Protocol-neutral Discovery and Thing Description Directory utilities.
//!
//! This crate starts with a deterministic in-memory directory backend that can
//! be used by tests, host runtimes, and the future Servient composition layer.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod directory;
pub mod embedded;
pub mod error;
#[cfg(feature = "std")]
pub mod host;
pub mod query;

pub use directory::{
    BorrowedDirectoryEntry, DirectoryEntry, DirectoryPage, InMemoryThingDirectory, ThingDirectory,
};
pub use error::{DiscoveryError, DiscoveryResult};
pub use query::{DirectoryQuery, QueryFilter, QueryPredicate};
