//! Directory watch — change notifications, distinct from search
//! (baseline v4.0 §6 / phase-p1 §1.8). Std-gated (uses `std::sync`).
//!
//! A [`DirectoryWatch`] is independent of any open search session: changes
//! observed via watch do NOT alter a session's monotonicity or replay
//! already-emitted items (audit E10). A watcher wanting the "current set"
//! opens a new search session; watch only delivers subsequent changes.

use clinkz_wot_core::ThingId;
use clinkz_wot_td::thing::Thing;

#[cfg(feature = "std")]
use crate::DiscoveryResult;
#[cfg(feature = "std")]
use alloc::boxed::Box;

/// One directory change observed by a [`DirectoryWatch`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum DirectoryChange {
    /// A Thing was added.
    Added(Thing),
    /// A Thing was updated (new revision).
    Updated(Thing),
    /// A Thing was removed.
    Removed(ThingId),
}

/// A stream of directory changes, independent of any search session.
#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait DirectoryWatch: Send {
    /// Yields the next change, or `Ok(None)` when the watch is done.
    async fn next(&mut self) -> DiscoveryResult<Option<DirectoryChange>>;

    /// Stops the watch.
    async fn stop(&mut self) -> DiscoveryResult<()>;
}
