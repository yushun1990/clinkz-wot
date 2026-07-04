//! The Scripting-API discovery process — a lazy session yielding full `Thing`s
//! (baseline v4.0 §6 / phase-p1 §1.6).
//!
//! [`ThingDiscoveryProcess`] is the WoT Scripting API `ThingDiscovery` analogue:
//! constructed synchronously (no network work), it opens the underlying
//! directory session lazily inside the first async `next()`. It forces
//! [`ProjectionMode::FullThingDescription`](crate::ProjectionMode) (AD18).

use alloc::{boxed::Box, sync::Arc};

use clinkz_wot_td::thing::Thing;

use crate::{
    DirectoryItem, DirectoryQuery, DirectoryReader, DirectorySession, DiscoveryError,
    DiscoveryResult, ProjectionMode,
};

/// The Scripting-API discovery session: yields full `Thing`s lazily.
///
/// Construction is sync and does no network/directory work; the async
/// Exploration (`DirectoryReader::open_search`) is deferred to the first
/// [`next()`](ThingDiscoveryProcess::next) (audit AD10).
pub struct ThingDiscoveryProcess {
    inner: Box<dyn DiscoverySession>,
}

impl ThingDiscoveryProcess {
    /// Wraps a concrete [`DiscoverySession`] impl. Servient/discoverer builds
    /// this from a resolved reader + query (see [`ProcessState`]).
    pub fn new(inner: Box<dyn DiscoverySession>) -> Self {
        Self { inner }
    }

    /// Yields the next Thing, `Ok(None)` at a clean end, `Err` on a terminal
    /// failure (after which the session is `Done` and further `next()` returns
    /// `Ok(None)`).
    pub async fn next(&mut self) -> DiscoveryResult<Option<Thing>> {
        self.inner.next().await
    }

    /// Stops the session.
    pub async fn stop(&mut self) -> DiscoveryResult<()> {
        self.inner.stop().await
    }

    /// Terminal-error accessor: `Some` only after the session terminated due
    /// to an error; `None` while live or on a clean end.
    pub fn error(&self) -> Option<&DiscoveryError> {
        self.inner.error()
    }
}

/// Scripting-API-level session yielding full `Thing`s. Distinct from
/// [`DirectorySession`] (which yields [`DirectoryItem`]s); the two are never
/// interchangeable (AD18).
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait DiscoverySession: Send {
    /// Yields the next Thing, or `Ok(None)` at a clean end.
    async fn next(&mut self) -> DiscoveryResult<Option<Thing>>;

    /// Stops the session.
    async fn stop(&mut self) -> DiscoveryResult<()>;

    /// Terminal-error accessor.
    fn error(&self) -> Option<&DiscoveryError>;
}

/// The concrete inner state of a [`ThingDiscoveryProcess`] (audit D2/H5).
///
/// v1 Introduction is trivially resolved at `discover()` time: the local
/// endpoint IS the in-memory reader, so `Pending` carries the resolved reader
/// + query; `next()` does Exploration only (`open_search`). A future remote-
/// capable variant would additionally carry an `Introducer` (deferred, E6).
pub enum ProcessState {
    /// Resolved reader + query; the session is opened lazily on first `next()`.
    Pending {
        /// The resolved directory reader.
        reader: Arc<dyn DirectoryReader>,
        /// The query (projection will be forced to FullThingDescription).
        query: DirectoryQuery,
    },
    /// Session opened; draining directory items.
    Open(Box<dyn DirectorySession>),
    /// Terminal: after an error or `stop()`. Carries the error if any.
    Done(Option<DiscoveryError>),
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl DiscoverySession for ProcessState {
    async fn next(&mut self) -> DiscoveryResult<Option<Thing>> {
        loop {
            match self {
                ProcessState::Pending { reader, query } => {
                    // Force full-TD projection (AD18) regardless of caller input.
                    let mut q = query.clone();
                    q.projection = ProjectionMode::FullThingDescription;
                    let session = reader.open_search(q).await?;
                    *self = ProcessState::Open(session);
                    continue;
                }
                ProcessState::Open(session) => match session.next().await? {
                    Some(batch) => {
                        for item in batch.items {
                            if let DirectoryItem::Full(thing) = item {
                                return Ok(Some(thing));
                            }
                            // Non-Full items cannot appear (projection forced);
                            // skip defensively.
                        }
                        continue;
                    }
                    None => {
                        *self = ProcessState::Done(None);
                        return Ok(None);
                    }
                },
                ProcessState::Done(_) => return Ok(None),
            }
        }
    }

    async fn stop(&mut self) -> DiscoveryResult<()> {
        match self {
            ProcessState::Open(session) => {
                let result = session.stop().await;
                *self = ProcessState::Done(None);
                result
            }
            ProcessState::Pending { .. } => {
                *self = ProcessState::Done(None);
                Ok(())
            }
            ProcessState::Done(_) => Ok(()),
        }
    }

    fn error(&self) -> Option<&DiscoveryError> {
        match self {
            ProcessState::Done(err) => err.as_ref(),
            _ => None,
        }
    }
}

impl ProcessState {
    /// Constructs a `Pending` process from a resolved reader + query.
    pub fn pending(reader: Arc<dyn DirectoryReader>, query: DirectoryQuery) -> Self {
        Self::Pending { reader, query }
    }

    /// Constructs a terminal `Done` process carrying `err` (for synchronous
    /// construction failures — audit D5 error bridging).
    pub fn done(err: Option<DiscoveryError>) -> Self {
        Self::Done(err)
    }
}
