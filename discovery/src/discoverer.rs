//! The `Discoverer` facade tying Introduction → Exploration → session
//! (baseline v4.0 §6 / phase-p1 §1.9).
//!
//! `discover()` and `explore_directory()` are **synchronous** entry points
//! returning a lazy [`ThingDiscoveryProcess`]; the async work happens inside
//! the first `next()`. `request_thing_description()` is async (a concrete TD
//! fetch is a network round-trip).

use alloc::{boxed::Box, sync::Arc};

use clinkz_wot_td::{AbsoluteUri, thing::Thing};

use crate::{
    CountMode, DirectoryFilter, DirectoryPublisher, DirectoryQuery, DirectoryReader,
    DiscoveryError, DiscoveryResult, ProcessState, ProjectionMode, ThingDiscoveryProcess,
};

/// A caller-facing discovery filter: wraps a [`DirectoryFilter`] plus
/// discovery-level hints (count mode). The Servient's `discover(filter)`
/// accepts this.
#[derive(Debug, Clone)]
pub struct DiscoveryFilter {
    /// The directory filter predicate.
    pub filter: DirectoryFilter,
    /// Whether to compute a total count.
    pub count_mode: CountMode,
}

impl DiscoveryFilter {
    /// A filter matching everything, no count.
    pub fn all() -> Self {
        Self {
            filter: DirectoryFilter::Any,
            count_mode: CountMode::None,
        }
    }

    /// Creates a discovery filter from a directory filter (no count).
    pub fn new(filter: DirectoryFilter) -> Self {
        Self {
            filter,
            count_mode: CountMode::None,
        }
    }

    /// Requests a total count be computed.
    pub fn with_count(mut self, mode: CountMode) -> Self {
        self.count_mode = mode;
        self
    }

    /// Returns the wrapped directory filter.
    pub fn into_filter(self) -> DirectoryFilter {
        self.filter
    }

    /// Returns the requested count mode.
    pub fn count_mode(&self) -> CountMode {
        self.count_mode
    }
}

impl Default for DiscoveryFilter {
    fn default() -> Self {
        Self::all()
    }
}

/// A reference to a directory to explore.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum DirectoryRef {
    /// The local (in-process) directory.
    Local,
    /// A remote directory URL (v1: unsupported without a fetcher backend, E6).
    Url(AbsoluteUri),
}

/// The Discoverer facade: the single entry point the Servient holds as
/// `Arc<dyn Discoverer>` (P3).
///
/// `discover`/`explore_directory` are sync and return a LAZY
/// [`ThingDiscoveryProcess`]; the async Exploration is deferred to the first
/// `next()` (audit AD10).
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait Discoverer: Send + Sync {
    /// Synchronous: returns a lazy [`ThingDiscoveryProcess`] built from
    /// `filter` against the local backend. No network work here.
    fn discover(&self, filter: DiscoveryFilter) -> DiscoveryResult<ThingDiscoveryProcess>;

    /// Synchronous, same lazy semantics as [`discover`](Self::discover), but
    /// against a specific directory.
    fn explore_directory(
        &self,
        dir: DirectoryRef,
        query: DirectoryQuery,
    ) -> DiscoveryResult<ThingDiscoveryProcess>;

    /// Async: a concrete TD fetch IS a network round-trip, so it stays async.
    async fn request_thing_description(&self, url: &AbsoluteUri) -> DiscoveryResult<Thing>;
}

/// A `Discoverer` backed by a local [`DirectoryReader`] (and optional
/// [`DirectoryPublisher`]). v1's reference discoverer: Introduction is
/// trivially local (E6 — no remote fetcher).
#[cfg(feature = "async")]
pub struct LocalDiscoverer {
    reader: Arc<dyn DirectoryReader>,
    publisher: Option<Arc<dyn DirectoryPublisher>>,
}

#[cfg(feature = "async")]
impl LocalDiscoverer {
    /// Creates a local discoverer over a directory reader.
    pub fn new(reader: Arc<dyn DirectoryReader>) -> Self {
        Self {
            reader,
            publisher: None,
        }
    }

    /// Attaches a publisher (for Servient directory publishing).
    pub fn with_publisher(mut self, publisher: Arc<dyn DirectoryPublisher>) -> Self {
        self.publisher = Some(publisher);
        self
    }

    /// Returns the directory reader.
    pub fn reader(&self) -> &Arc<dyn DirectoryReader> {
        &self.reader
    }

    /// Returns the publisher, if any.
    pub fn publisher(&self) -> &Option<Arc<dyn DirectoryPublisher>> {
        &self.publisher
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl Discoverer for LocalDiscoverer {
    fn discover(&self, filter: DiscoveryFilter) -> DiscoveryResult<ThingDiscoveryProcess> {
        let count_mode = filter.count_mode();
        let query = DirectoryQuery {
            filter: filter.into_filter(),
            page_size: 0,
            continuation: None,
            count_mode,
            consistency: crate::ConsistencyMode::Live,
            projection: ProjectionMode::FullThingDescription,
        };
        Ok(ThingDiscoveryProcess::new(Box::new(ProcessState::pending(
            Arc::clone(&self.reader),
            query,
        ))))
    }

    fn explore_directory(
        &self,
        dir: DirectoryRef,
        query: DirectoryQuery,
    ) -> DiscoveryResult<ThingDiscoveryProcess> {
        match dir {
            DirectoryRef::Local => Ok(ThingDiscoveryProcess::new(Box::new(ProcessState::pending(
                Arc::clone(&self.reader),
                query,
            )))),
            DirectoryRef::Url(_) => Err(DiscoveryError::UnsupportedEndpoint),
        }
    }

    async fn request_thing_description(&self, _url: &AbsoluteUri) -> DiscoveryResult<Thing> {
        // v1 has no remote TD fetcher backend (E6).
        Err(DiscoveryError::NotImplemented)
    }
}
