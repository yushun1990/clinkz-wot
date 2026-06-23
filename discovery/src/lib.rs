#![no_std]
//!
//! Protocol-neutral Discovery and Thing Description Directory utilities.
//!
//! This crate starts with a deterministic in-memory directory backend that can
//! be used by tests, std runtimes, and the future Servient composition layer.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod directory;
pub mod error;
pub mod local;
pub mod query;
pub mod scripting;
#[cfg(feature = "std")]
pub mod storage;

pub use directory::{
    BorrowedDirectoryEntry, DirectoryEntry, DirectoryPage, InMemoryThingDirectory, ThingDirectory,
};
pub use error::{DiscoveryError, DiscoveryResult};
pub use query::{DirectoryQuery, QueryFilter, QueryPredicate};
pub use scripting::{ThingDiscovery, ThingFilter, discover};
#[cfg(feature = "std")]
pub use storage::SharedThingDirectory;
