# Phase P1 — Discovery Rewrite

> Baseline: `docs/baseline/engine-architecture-baseline.md` (v4.0) §6.
> Design source: `docs/plan/discovery-directory-refactor-plan.md` (read for the
> target shape; this file sequences the implementation).

## Goal

Rewrite `clinkz-wot-discovery` into the WoT Discovery two-phase
Introduction → Exploration model with continuation-driven directory sessions.
Replace the local CRUD container abstraction with a service-oriented surface.

P1 leaves **`clinkz-wot-discovery` compiling and tested in isolation** on top of
the P0 core identity types. Servient integration (`Servient` holding
`Arc<dyn Discoverer>`) lands in P3.

## Entry Criteria

- P0 `ThingId` (`core/src/identity.rs`) is stable and re-exported.
- v4.0 §6 and the discovery design source are locked.

## Current State (being replaced)

- `discovery/src/directory.rs` (723 lines): `ThingDirectory` CRUD trait,
  `DirectoryPage { entries, total, offset, limit }`, `InMemoryThingDirectory`,
  `DirectoryEntry`, `BorrowedDirectoryEntry`.
- `discovery/src/scripting.rs` (514 lines): transitional `ThingFilter` +
  `DiscoveryMethod { Local, Directory, Multicast, Everything }` +
  buffered `ThingDiscovery { VecDeque<Thing> }`.
- `discovery/src/query.rs` (189 lines): `QueryFilter`, `QueryPredicate`,
  `DirectoryQuery`.
- `discovery/src/local.rs`: local directory convenience.
- `discovery/src/storage.rs` (std): storage adapters.

All of the above are superseded by the v4.0 §6 surface.

## Work Breakdown

### Step 1.1 — Crate root and module layout

Restructure `discovery/src/` by responsibility (no `mod.rs` files; module-name
files per AGENTS.md):

- `lib.rs` — crate root, re-exports, `#![no_std]`.
- `endpoint.rs` — Introduction types (`DiscoveryEndpoint`, `EndpointKind`,
  `IntroductionSource`, `AuthHint`).
- `resolver.rs` — Exploration resolver traits
  (`ThingDescriptionResolver`, `ThingLinkResolver`).
- `directory.rs` — directory reader/session/query model (rewritten).
- `session.rs` — `DirectorySession`, `ThingDiscoveryProcess`.
- `publisher.rs` — `DirectoryPublisher`, lease/revision types.
- `watch.rs` — `DirectoryWatch`, `DirectoryChange` (std-gated).
- `discoverer.rs` — `Discoverer` facade + `DiscoveryFilter`.
- `backend/memory.rs` — in-memory reference backend.
- `error.rs` — `DiscoveryError` reworked.
- `storage.rs` — std-only storage adapters (retained, behind `std`).

### Step 1.2 — Introduction layer (`endpoint.rs`)

```rust
pub struct DiscoveryEndpoint {
    pub url: AbsoluteUri,
    pub kind: EndpointKind,
    pub source: IntroductionSource,
    pub auth_hint: Option<AuthHint>,
}
pub enum EndpointKind { ThingDescription, ThingDirectory, ThingLink }
pub enum IntroductionSource { /* DirectUrl, SelfDescription, DnsSd, Dhcp, Beacon, ... */ }
```

`AbsoluteUri` is reused from `clinkz-wot-td`. An `Introducer` trait
(`async fn discover_endpoints(&self) -> DiscoveryResult<Vec<DiscoveryEndpoint>>`)
lives here; concrete introducers (mDNS, BLE, etc.) are out of scope for v1 —
only a `DirectUrlIntroducer` reference impl is provided.

### Step 1.3 — Exploration resolver traits (`resolver.rs`)

```rust
#[async_trait]
pub trait ThingDescriptionResolver: Send + Sync {
    async fn request_thing_description(&self, url: &AbsoluteUri) -> DiscoveryResult<Thing>;
}
#[async_trait]
pub trait ThingLinkResolver: Send + Sync {
    async fn resolve_thing_link(&self, td: &Thing) -> DiscoveryResult<DiscoveryEndpoint>;
}
```

Distinct traits, never collapsed into one container. v1 ships a resolver that
wraps the in-memory backend; concrete HTTP/CoAP TD fetchers are out of scope
(integration point only).

### Step 1.4 — Directory query model (`directory.rs` query half)

```rust
pub struct DirectoryQuery {
    pub filter: DirectoryFilter,
    pub page_size: u32,
    pub continuation: Option<ContinuationToken>,
    pub count_mode: CountMode,
    pub consistency: ConsistencyMode,
    pub projection: ProjectionMode,
}
#[non_exhaustive]
pub enum DirectoryFilter {
    ByExample(ThingFragment), Text(String), Capability(CapabilityFilter),
    And(Vec<DirectoryFilter>), Or(Vec<DirectoryFilter>),
    // Semantic(SemanticQuery) and Native(NativeQuery) are NOT shipped in v1;
    // they will be added non-breakingly when a real backend needs them
    // (resolved decision A2).
}
#[non_exhaustive]
pub enum CountMode { None, Estimate, Exact }
#[non_exhaustive]
pub enum ConsistencyMode { Live, SessionStable }
#[non_exhaustive]
pub enum ProjectionMode { IdOnly, Summary, FullThingDescription }
```

`#[non_exhaustive]` on `DirectoryFilter` and the mode enums makes the v1 set
forward-compatible: `Semantic`/`Native` are added later without a breaking
change, and callers are forced to write a `_ =>` fallback. `ThingFragment`
reuses the TD's `ExtensionMap`-shaped fragment (closest to today's
`QueryFilter::Fragment`). `CapabilityFilter` covers affordance names,
operations, security schemes, protocol hints. The v1 set
(`ByExample`/`Text`/`Capability`/`And`/`Or`) is the complete implementable
set for the in-memory backend.

### Step 1.5 — Directory reader + session (`directory.rs` reader half, `session.rs`)

```rust
#[async_trait]
pub trait DirectoryReader: Send + Sync {
    async fn get(&self, id: &ThingId) -> DiscoveryResult<Option<Thing>>;
    async fn open_search(&self, query: DirectoryQuery)
        -> DiscoveryResult<Box<dyn DirectorySession>>;
}
#[async_trait]
pub trait DirectorySession: Send {
    async fn next(&mut self) -> DiscoveryResult<Option<DirectoryItem>>;
    async fn stop(&mut self) -> DiscoveryResult<()>;
    fn error(&self) -> Option<&DiscoveryError>;
}
pub struct DirectoryBatch { /* items, continuation, stats — internal to backends */ }
pub struct DirectoryStats { pub has_more: bool, pub count: Option<CountValue> }
pub enum CountValue { Estimate(u64), Exact(u64) }
```

`ContinuationToken` is an opaque owned token (`Vec<u8>` / `String` newtype). The
reader contract: `open_search` returns **one session**, not a buffered page;
the session yields items lazily and advances by continuation, never by offset.

Live-monotonic rules (design source §Live Semantics) are encoded in the
in-memory backend: emitted items never re-emit; inserts before the cursor are
not guaranteed visible; inserts after may appear in later batches when the
backend supports live visibility.

### Step 1.6 — `ThingDiscoveryProcess` (`session.rs`, `discoverer.rs`)

```rust
pub struct ThingDiscoveryProcess { inner: Box<dyn DiscoverySession> }
#[async_trait]
pub trait DiscoverySession: Send {
    async fn next(&mut self) -> DiscoveryResult<Option<Thing>>;
    async fn stop(&mut self) -> DiscoveryResult<()>;
    fn error(&self) -> Option<&DiscoveryError>;
}
```

`ThingDiscoveryProcess` adapts a `DirectorySession` (mapping `DirectoryItem` →
`Thing` per `ProjectionMode`), or wraps a resolver/link flow. It is **lazy**:
construction performs no network work; the first `next()` opens the session.

`remaining()` is removed entirely. `stop()` / `error()` retained.

### Step 1.7 — Publisher side (`publisher.rs`)

```rust
#[async_trait]
pub trait DirectoryPublisher: Send + Sync {
    async fn register(&self, r: DirectoryRegistration) -> DiscoveryResult<RegistrationAck>;
    async fn renew(&self, lease: LeaseToken) -> DiscoveryResult<LeaseState>;
    async fn update(&self, id: &ThingId, patch: DirectoryPatch) -> DiscoveryResult<Revision>;
    async fn unregister(&self, id: &ThingId) -> DiscoveryResult<()>;
}
```

`DirectoryRegistration` carries the TD + optional TTL/lease. `DirectoryPatch` is
a JSON-Merge-Patch-shaped carrier. `Revision`/`LeaseToken`/`LeaseState`/
`RegistrationAck` are typed. v1 in-memory backend supports `register`/`update`/
`unregister` fully and `renew` as a no-op ack (no real TTL aging).

### Step 1.8 — Watch (`watch.rs`, std-gated)

```rust
#[async_trait]
pub trait DirectoryWatch: Send {
    async fn next(&mut self) -> DiscoveryResult<Option<DirectoryChange>>;
    async fn stop(&mut self) -> DiscoveryResult<()>;
}
pub enum DirectoryChange { Added(Thing), Updated(Thing), Removed(ThingId) }
```

Distinct from search. In-memory backend offers an opt-in watch backed by a
version counter + listener list. Gated behind `std` (uses `std::sync`).

### Step 1.9 — `Discoverer` facade (`discoverer.rs`)

```rust
#[async_trait]
pub trait Discoverer: Send + Sync {
    async fn discover(&self, filter: DiscoveryFilter) -> DiscoveryResult<ThingDiscoveryProcess>;
    async fn explore_directory(&self, dir: DirectoryRef, q: DirectoryQuery)
        -> DiscoveryResult<ThingDiscoveryProcess>;
    async fn request_thing_description(&self, url: &AbsoluteUri) -> DiscoveryResult<Thing>;
}
pub enum DiscoveryFilter { /* wraps DirectoryFilter + method hints */ }
pub enum DirectoryRef { Local, Url(AbsoluteUri) }
```

`discover()` orchestrates Introduction then Exploration. A default
`LocalDiscoverer` composes the in-memory reader + publisher + a
`DirectUrlIntroducer`.

### Step 1.10 — In-memory reference backend (`backend/memory.rs`)

`InMemoryDirectory` implements **all four** capability traits
(`DirectoryReader`, `DirectoryPublisher`, `ThingDescriptionResolver`,
`DirectoryWatch`-gated). It keeps the secondary indexes (Title, Property,
Action, Event, Fragment) from the current hardening pass for O(log n)
filtering, but serves via continuation sessions instead of `offset+total`.
`SessionStable` sessions snapshot the matching id set at open time; `Live`
sessions re-scan with a moving cursor.

`get_ref` (borrowed lookup, no clone) is retained for internal use.

### Step 1.11 — Error taxonomy (`error.rs`)

Rework `DiscoveryError` to match the new surface:
`UnsupportedCountMode`, `UnsupportedConsistency`, `UnsupportedProjection`,
`SessionClosed`, `InvalidContinuation`, `UnknownEndpoint`, `ResolverFailed`,
`PublisherConflict { id, revision }`, `LeaseExpired`. Drop variants tied to
`offset+total` pagination.

### Step 1.12 — Removals

Delete from the crate: `ThingDirectory` trait, `DirectoryPage`, `DirectoryEntry`/
`BorrowedDirectoryEntry` (replaced by `DirectoryItem`), `QueryFilter`/
`QueryPredicate` (folded into `DirectoryFilter` + backend predicate helpers),
`ThingFilter`/`DiscoveryMethod`/buffered `ThingDiscovery` (replaced by
`DiscoveryFilter`/`ThingDiscoveryProcess`). The `local.rs` convenience API is
folded into `backend/memory.rs`.

### Step 1.13 — `no_std + alloc` boundary

Crate root + `endpoint.rs` + `resolver.rs` + `directory.rs` + `session.rs` +
`publisher.rs` + `discoverer.rs` + `backend/memory.rs` + `error.rs` are
`no_std + alloc`. `watch.rs` and `storage.rs` are `std`-gated. `#[async_trait]`
without `Send` bounds where a session must be `!Send`-portable is avoided —
sessions are `Send` to stay spawnable; the in-memory backend is `Send + Sync`.

## Resolved Decisions

- **A2 (Semantic/Native filters).** `DirectoryFilter`, `CountMode`,
  `ConsistencyMode`, and `ProjectionMode` are marked `#[non_exhaustive]`. v1
  ships only the implementable set: `DirectoryFilter::ByExample`/`Text`/
  `Capability`/`And`/`Or`. `Semantic(SemanticQuery)` and `Native(NativeQuery)`
  are **not** shipped in v1; they will be added non-breakingly when a real
  backend (HTTP TDD with SPARQL, or a backend-specific query escape hatch)
  needs them. Rationale: shipping typed carriers that the only v1 backend (in-
  memory) cannot serve would be dead, untested code that misleads callers into
  runtime `Unsupported` failures; `#[non_exhaustive]` gives the same forward-
  compatibility without the dead surface.

### Open Questions

1. **`AbsoluteUri` source.** `AbsoluteUri` currently lives in `clinkz-wot-td`
   (TD crate) with a cached `fluent_uri` parse (PLAN §Performance Hardening).
   P1 depends on it being re-exported and `Clone`able cheaply. Verify the TD
   crate exposes it publicly; if not, P0 §3 TD cleanup surfaces it.

## Deliverables

- `clinkz-wot-discovery` rewritten per v4.0 §6.
- `InMemoryDirectory` reference backend implementing all four capability traits.
- Continuation-session local discovery round-trip + publisher register/update/
  unregister + lease-renew no-op covered by tests.

## Exit Criteria

- Crate compiles `no_std + alloc` (root) and `std` (storage + watch).
- `cargo test -p clinkz-wot-discovery` covers: filter→batch→continuation→next
  batch; `get`/`open_search`; publisher CRUD; projection modes; count modes
  (None default, Exact opt-in); `SessionStable` monotonicity; `ThingDiscoveryProcess`
  laziness.
- No `ThingDirectory`/`DirectoryPage`/`DiscoveryMethod`/buffered-`ThingDiscovery`
  references remain.

## Risks

- The continuation-cursor design for the in-memory `Live` session must avoid
  re-emitting updated items; a `(id, revision)` cursor or a high-water-mark id
  cursor is needed. Pick one and document it in `backend/memory.rs`.
- `#[async_trait]` `Box` per `next()` call on a large scan could allocate; the
  backend yields batches internally and the session drains a local buffer, so
  `next()` is usually a cheap buffer pop — verify in tests.
