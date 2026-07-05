//! Directory query model, result items, registration carriers, and the
//! directory reader/session contract (baseline v4.0 §6 / phase-p1 §1.4–§1.5).
//!
//! The query model is plain data (`no_std + alloc`); the `DirectoryReader` and
//! `DirectorySession` traits are `async` (behind the `async` feature).

use alloc::{string::String, vec::Vec};

use clinkz_wot_core::{MediaType, ThingId};
use clinkz_wot_td::{data_type::Operation, thing::Thing};

#[cfg(feature = "async")]
use crate::{DiscoveryError, DiscoveryResult};
#[cfg(feature = "async")]
use alloc::boxed::Box;

// ---------------------------------------------------------------------------
// Query request model.
// ---------------------------------------------------------------------------

/// A directory search request: filter + paging + modes.
///
/// Paging is continuation-based (one batch + token), never `offset+total`
/// (audit defect 3). The reader returns a lazy [`DirectorySession`] over the
/// matching set, not a buffered page.
#[derive(Debug, Clone)]
pub struct DirectoryQuery {
    /// Filter predicate tree. `#[non_exhaustive]` so future filter kinds
    /// (`Semantic`/`Native`) are added non-breakingly (decision A2).
    pub filter: DirectoryFilter,
    /// Suggested batch size per `next()`. The backend MAY cap or round it.
    pub page_size: u32,
    /// Continuation token from a previous batch, or `None` for the first.
    pub continuation: Option<ContinuationToken>,
    /// Whether to compute a total count.
    pub count_mode: CountMode,
    /// Session consistency. v1 ships `Live` only.
    pub consistency: ConsistencyMode,
    /// Result projection. `ThingDiscoveryProcess` forces `FullThingDescription`
    /// (AD18); lighter projections apply only to this lower-level API.
    pub projection: ProjectionMode,
}

impl DirectoryQuery {
    /// A query that matches every Thing (`FullThingDescription`, no count).
    pub fn all() -> Self {
        Self {
            filter: DirectoryFilter::Any,
            page_size: 0,
            continuation: None,
            count_mode: CountMode::None,
            consistency: ConsistencyMode::Live,
            projection: ProjectionMode::FullThingDescription,
        }
    }
}

impl Default for DirectoryQuery {
    fn default() -> Self {
        Self::all()
    }
}

/// Filter predicate tree for [`DirectoryQuery`].
///
/// v1 ships `Any`/`ByExample`/`Text`/`Capability`/`And`/`Or` — the complete
/// set the in-memory backend can serve. `Semantic`/`Native` are added later
/// non-breakingly when a real backend needs them (decision A2).
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum DirectoryFilter {
    /// Matches every Thing.
    Any,
    /// Match by partial TD fragment (each field narrows; `None`/empty = "any").
    ByExample(ThingFragment),
    /// Full-text search over the TD's human-readable fields.
    Text(String),
    /// Match by capability/protocol exposure.
    Capability(CapabilityFilter),
    /// All sub-filters must match.
    And(Vec<DirectoryFilter>),
    /// Any sub-filter must match.
    Or(Vec<DirectoryFilter>),
}

/// Partial TD fragment for [`DirectoryFilter::ByExample`]. Each field narrows
/// the match; `None`/empty means "any".
#[derive(Debug, Clone, Default)]
pub struct ThingFragment {
    /// Title substring to match.
    pub title: Option<String>,
    /// Exact Thing id.
    pub id: Option<ThingId>,
    /// `@type` semantic tags to require.
    pub types: Vec<String>,
    /// Property affordance names to require.
    pub properties: Vec<String>,
    /// Action affordance names to require.
    pub actions: Vec<String>,
    /// Event affordance names to require.
    pub events: Vec<String>,
}

/// Capability/protocol filter for [`DirectoryFilter::Capability`].
#[derive(Debug, Clone, Default)]
pub struct CapabilityFilter {
    /// Required affordance name (any kind).
    pub affordance: Option<String>,
    /// Operations the matched affordance form must support.
    pub operations: Vec<Operation>,
    /// Security scheme names to require.
    pub security_schemes: Vec<String>,
    /// Protocol hint (e.g. `"zenoh"`, `"http"`), matched against form schemes.
    pub protocol: Option<String>,
}

/// Whether to compute a total count of matching Things.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CountMode {
    /// Do not compute a count (default).
    #[default]
    None,
    /// Backend MAY return an estimate; MAY upgrade to `Exact`.
    Estimate,
    /// Exact count required; backend that cannot satisfy it returns
    /// [`DiscoveryError::UnsupportedCountMode`]. A backend MAY upgrade
    /// `Estimate → Exact` but MUST NOT silently downgrade `Exact → Estimate`.
    Exact,
}

/// Session consistency.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConsistencyMode {
    /// Live: each batch reads the current matching set with a monotonic cursor
    /// (already-emitted ids never re-emit). v1 ships this only.
    #[default]
    Live,
    // SessionStable is deferred (audit defect AD3).
}

/// Result projection.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionMode {
    /// Yield only Thing ids.
    IdOnly,
    /// Yield lightweight summary fields.
    Summary,
    /// Yield full Thing Descriptions (default; the only mode
    /// [`crate::ThingDiscoveryProcess`] uses — AD18).
    #[default]
    FullThingDescription,
}

// ---------------------------------------------------------------------------
// Result items.
// ---------------------------------------------------------------------------

/// One directory result item, shaped by [`ProjectionMode`].
#[non_exhaustive]
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum DirectoryItem {
    /// `IdOnly` projection.
    Id(ThingId),
    /// `Summary` projection.
    Summary {
        /// Thing id.
        id: ThingId,
        /// Summary fields.
        summary: SummaryFields,
    },
    /// `FullThingDescription` projection.
    Full(Thing),
}

/// Lightweight summary fields for [`DirectoryItem::Summary`].
#[derive(Debug, Clone, Default)]
pub struct SummaryFields {
    /// Thing title.
    pub title: Option<String>,
    /// `@type` semantic tags.
    pub types: Vec<String>,
    /// Number of property affordances.
    pub property_count: usize,
    /// Number of action affordances.
    pub action_count: usize,
    /// Number of event affordances.
    pub event_count: usize,
}

/// Count value returned in [`DirectoryStats`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountValue {
    /// Approximate count.
    Estimate(u64),
    /// Exact point-in-time count (may be stale on a `Live` set by the next batch).
    Exact(u64),
}

/// Per-batch stats.
#[derive(Debug, Clone, Default)]
pub struct DirectoryStats {
    /// Whether more items may follow (continuation available).
    pub has_more: bool,
    /// Total count if requested (`None` when `CountMode::None`).
    pub count: Option<CountValue>,
}

/// Opaque continuation token anchoring a live session's cursor. Owned so it
/// can cross a future boundary. Opaque to callers; meaningful only to the
/// backend that issued it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationToken(pub Vec<u8>);

impl ContinuationToken {
    /// Returns the raw token bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Publisher-side carriers.
// ---------------------------------------------------------------------------

/// A TD registration request (publisher → directory).
#[derive(Debug, Clone)]
pub struct DirectoryRegistration {
    /// The Thing Description to register.
    pub td: Thing,
    /// Optional lease time-to-live; `None` = no lease (manual lifecycle).
    pub ttl: Option<core::time::Duration>,
}

/// Acknowledgement for a successful [`DirectoryRegistration`].
#[derive(Debug, Clone)]
pub struct RegistrationAck {
    /// Registered Thing id.
    pub id: ThingId,
    /// Monotonic per-Thing revision assigned by the directory.
    pub revision: Revision,
    /// Lease state if a lease was requested.
    pub lease: Option<LeaseState>,
}

/// Monotonic per-Thing revision counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Revision(pub u64);

/// Opaque renewal handle for a lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseToken(pub Vec<u8>);

/// Lease state returned by register/renew.
#[derive(Debug, Clone)]
pub struct LeaseState {
    /// Renewal token.
    pub token: LeaseToken,
    /// Absolute expiry time offset from some epoch (backend-defined).
    pub expires_at: Option<core::time::Duration>,
}

/// Protocol-neutral Merge-Patch carrier (audit round-2 S1/AD49). The patch
/// body is raw bytes plus a declared media type; serialization/deserialization
/// happens at the backend, keeping the `no_std + alloc` discovery root JSON-free.
#[derive(Debug, Clone)]
pub struct DirectoryPatch {
    /// Raw patch bytes (JSON Merge Patch, CBOR, …).
    pub body: Vec<u8>,
    /// Declares the patch representation.
    pub content_type: MediaType,
}

// ---------------------------------------------------------------------------
// Directory reader + session contract (async; behind `async` feature).
// ---------------------------------------------------------------------------

/// Read-side directory service (baseline §6). Returns lazy sessions, not
/// buffered pages.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait DirectoryReader: Send + Sync {
    /// Fetches a single Thing by id.
    async fn get(&self, id: &ThingId) -> DiscoveryResult<Option<Thing>>;

    /// Opens a lazy search session over `query`. Returns one session that
    /// yields items by continuation, never by offset.
    async fn open_search(
        &self,
        query: DirectoryQuery,
    ) -> DiscoveryResult<Box<dyn DirectorySession>>;
}

/// Lazy directory search session yielding [`DirectoryItem`]s. One session per
/// search; advances by continuation. Live-monotonic (already-emitted ids never
/// re-emit, regardless of subsequent updates — see phase-p1 §1.5).
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait DirectorySession: Send {
    /// Yields the next batch of items, or `Ok(None)` at a clean end.
    async fn next(&mut self) -> DiscoveryResult<Option<DirectoryBatch>>;

    /// Stops the session. After this, `next()` returns `Ok(None)`.
    async fn stop(&mut self) -> DiscoveryResult<()>;

    /// Terminal-error accessor: `Some` only after the session terminated due
    /// to an error; `None` while live or on a clean end.
    fn error(&self) -> Option<&DiscoveryError>;
}

/// One batch of directory results.
#[derive(Debug, Clone)]
pub struct DirectoryBatch {
    /// Result items in this batch.
    pub items: Vec<DirectoryItem>,
    /// Continuation token for the next batch, or `None` if this was the last.
    pub continuation: Option<ContinuationToken>,
    /// Per-batch stats.
    pub stats: DirectoryStats,
}
