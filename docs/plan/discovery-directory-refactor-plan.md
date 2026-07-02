> **⚠ DESIGN SOURCE for P1.** This document remains the **design source** for
> the Discovery rewrite. The implementation plan is
> `docs/plan/phase-p1-discovery.md` under the v4.0 baseline. Read this for the
> target shape; follow P1 for sequencing.
>
> **Divergences from this source already locked in P1/baseline (audit F5):**
> - `Discoverer::discover` / `explore_directory` are **sync** here (§Scripting
>   API Mapping) but were changed to **sync returning a lazy process** by AD10
>   (P1 §1.9); async work is deferred to `ThingDiscoveryProcess::next()`.
> - This source uses `ThingDescription` as the result type; the codebase and P1
>   use `Thing`. Treat `ThingDescription` ≡ `Thing` when reading this source.

# Discovery / Directory Refactor Plan

## Goal

Redesign Discovery and Directory from first principles for W3C WoT Discovery
and WoT Scripting API alignment, favoring predictable latency, scalable large-
result handling, and protocol-realistic semantics over compatibility with the
current in-memory-first implementation.

This document supersedes the current transitional discovery shape described in
`PLAN.md` C7 where `ThingDiscovery` is a buffered local process object backed by
the in-memory directory. The new design treats Discovery as a network-shaped
process, Directory as an Exploration service, and pagination as a continuation-
driven session instead of `offset + total`.

## Design Drivers

- Align with WoT Discovery's two-phase model: Introduction then Exploration.
- Align with WoT Scripting API's process-oriented discovery surface.
- Optimize for large directories and remote/external directory services.
- Make exact counts optional and expensive by contract, never mandatory.
- Avoid eager cloning or materializing complete result sets by default.
- Model live, changing directories explicitly instead of pretending to offer a
  cheap stable snapshot everywhere.
- Keep local in-memory directory as a backend, not as the architecture source.

## Core Problems In The Current Model

- `ThingDirectory` is modeled as a local CRUD container with query helpers.
  That is too weak for a real WoT Thing Description Directory service.
- `DirectoryPage { entries, total, offset, limit }` forces full-match counting
  even when callers only need the next batch.
- `ThingDiscovery` stores `VecDeque<Thing>`, so `discover()` eagerly buffers all
  final matches before returning the process object.
- `Servient<D>` owns a concrete directory implementation directly, which fits a
  co-located local directory but does not model external TDD services well.
- Discovery methods (`Local`, `Directory`, `Multicast`, `Everything`) are mixed
  into a single local filter object rather than orchestrated as introduction and
  exploration capabilities.

## Target Architecture

The new architecture is split into four independent layers.

### 1. Introduction

Introduction discovers candidate Exploration endpoints without exposing full TD
metadata by default.

Output is a deduplicated set of endpoints, not a set of `Thing` objects.

```rust
pub struct DiscoveryEndpoint {
    pub url: AbsoluteUri,
    pub kind: EndpointKind,
    pub source: IntroductionSource,
    pub auth_hint: Option<AuthHint>,
}

pub enum EndpointKind {
    ThingDescription,
    ThingDirectory,
    ThingLink,
}
```

Example sources include direct URLs, self-description, mDNS / DNS-SD, DHCP,
QR/BLE beacons, or implementation-defined platform integrations.

### 2. Exploration

Exploration consumes endpoints and yields TDs or directory sessions.

Three capability types are distinct and must not be collapsed back into one
container trait.

```rust
#[async_trait]
pub trait ThingDescriptionResolver {
    async fn request_thing_description(
        &self,
        url: &AbsoluteUri,
    ) -> DiscoveryResult<ThingDescription>;
}

#[async_trait]
pub trait ThingLinkResolver {
    async fn resolve_thing_link(
        &self,
        td: &ThingDescription,
    ) -> DiscoveryResult<DiscoveryEndpoint>;
}

#[async_trait]
pub trait DirectoryReader {
    async fn get(&self, id: &ThingId) -> DiscoveryResult<Option<ThingDescription>>;

    async fn open_search(
        &self,
        query: DirectoryQuery,
    ) -> DiscoveryResult<Box<dyn DirectorySession>>;
}
```

### 3. Directory

Directory is an Exploration service, not a local map. Its primary abstraction is
an authenticated, continuation-driven search session.

```rust
pub struct DirectoryQuery {
    pub filter: DirectoryFilter,
    pub page_size: u32,
    pub continuation: Option<ContinuationToken>,
    pub count_mode: CountMode,
    pub consistency: ConsistencyMode,
    pub projection: ProjectionMode,
}

pub enum CountMode {
    None,
    Estimate,
    Exact,
}

pub enum ConsistencyMode {
    Live,
    SessionStable,
}

pub enum ProjectionMode {
    IdOnly,
    Summary,
    FullThingDescription,
}
```

Search results are returned in batches:

```rust
pub struct DirectoryBatch {
    pub items: Vec<DirectoryItem>,
    pub continuation: Option<ContinuationToken>,
    pub stats: DirectoryStats,
}

pub struct DirectoryStats {
    pub has_more: bool,
    pub count: Option<CountValue>,
}

pub enum CountValue {
    Estimate(u64),
    Exact(u64),
}
```

This replaces `offset + limit + total` with continuation-driven session
progress. Exact counts are opt-in and may be unsupported or expensive.

### 4. Discovery Process

`ThingDiscovery` becomes a session/handle object backed by a live or buffered
session implementation.

```rust
pub struct ThingDiscoveryProcess {
    inner: Box<dyn DiscoverySession>,
}

#[async_trait]
pub trait DiscoverySession {
    async fn next(&mut self) -> DiscoveryResult<Option<ThingDescription>>;
    async fn stop(&mut self) -> DiscoveryResult<()>;
    fn error(&self) -> Option<&DiscoveryError>;
}
```

Important changes:

- `remaining()` is removed.
- The process does not promise a fully buffered local result set.
- A session may wrap a remote directory cursor, a local in-memory iterator, a
  link resolver, or a composite orchestrator.

## Live Semantics

The default semantic target is not a full snapshot. It is a live but monotonic
session.

Rules:

- A session advances by continuation token, not by offset.
- Results already emitted in a session are never re-emitted by pagination.
- New entities inserted "before" the current continuation point are not
  guaranteed to appear in the same session.
- New entities inserted after the current continuation point may appear in a
  later batch when the backend supports live visibility.
- Item updates after emission are represented by a new session or a watch API,
  not by replaying items inside the same search session.

This avoids the worst properties of both extremes:

- not a fake snapshot that is too expensive to guarantee,
- not an unstable "current page over a moving list" model that causes duplicates
  and missing items constantly.

## Query Model

The current fragment-only `ThingFilter` is too narrow. The new directory query
model must support:

```rust
pub enum DirectoryFilter {
    ByExample(ThingDescriptionFragment),
    Text(String),
    Semantic(SemanticQuery),
    Capability(CapabilityFilter),
    Native(NativeQuery),
    And(Vec<DirectoryFilter>),
    Or(Vec<DirectoryFilter>),
}
```

Notes:

- `ByExample` is the closest match to today's fragment filter.
- `Text` is for practical user-facing keyword searches.
- `Semantic` supports SPARQL or other semantic query backends.
- `Capability` supports filters such as affordance names, operations, security
  schemes, protocol exposure, or location hints.
- `Native` allows backend-specific query extensions without polluting the
  protocol-neutral core surface.

## Publisher Side

A real directory also needs a provider-side API. This is not CRUD over a local
container; it is lease- and revision-aware publication.

```rust
#[async_trait]
pub trait DirectoryPublisher {
    async fn register(
        &self,
        registration: DirectoryRegistration,
    ) -> DiscoveryResult<RegistrationAck>;

    async fn renew(&self, lease: LeaseToken) -> DiscoveryResult<LeaseState>;

    async fn update(
        &self,
        id: &ThingId,
        patch: DirectoryPatch,
    ) -> DiscoveryResult<Revision>;

    async fn unregister(&self, id: &ThingId) -> DiscoveryResult<()>;
}
```

Rationale:

- dynamic TDs require revision handling,
- remote directories need leases / TTL / keepalive,
- orphaned Things must age out automatically,
- publisher and reader sides have different performance and consistency needs.

## Watch / Change Tracking

Large dynamic deployments need change tracking separate from search pagination.

```rust
#[async_trait]
pub trait DirectoryWatch {
    async fn next(&mut self) -> DiscoveryResult<Option<DirectoryChange>>;
    async fn stop(&mut self) -> DiscoveryResult<()>;
}
```

This watch capability is distinct from the search session:

- search is for enumerating current matches,
- watch is for receiving later changes.

Keeping them separate preserves predictable pagination and avoids duplicating
items inside a single discovery process just because the directory changes while
it is being read.

## Scripting API Mapping

The scripting-facing API should expose three top-level discovery capabilities:

```rust
#[async_trait]
pub trait Discoverer {
    async fn discover(
        &self,
        filter: DiscoveryFilter,
    ) -> DiscoveryResult<ThingDiscoveryProcess>;

    async fn explore_directory(
        &self,
        directory: DirectoryRef,
        query: DirectoryQuery,
    ) -> DiscoveryResult<ThingDiscoveryProcess>;

    async fn request_thing_description(
        &self,
        url: &AbsoluteUri,
    ) -> DiscoveryResult<ThingDescription>;
}
```

Mapping:

- `discover()` orchestrates Introduction then Exploration.
- `explore_directory()` explicitly starts a TDD search session.
- `request_thing_description()` handles direct TD retrieval.

This is closer to the WoT Discovery model than encoding every mode inside a
single `ThingFilter.method`.

## Servient Refactor

`Servient` should no longer be generic over a concrete directory type.

Target shape:

```rust
pub struct Servient {
    discovery_client: Arc<dyn Discoverer>,
    directory_publisher: Option<Arc<dyn DirectoryPublisher>>,
    local_catalog: Option<Arc<dyn LocalCatalog>>,
    // existing binding registries, security providers, payload codecs...
}
```

Implications:

- consuming side uses a discovery capability, not a local directory object,
- exposing side optionally republishes to a directory publisher,
- a local catalog may still exist for local-only scenarios or tests,
- remote TDD integration stops being a bolt-on afterthought.

## In-Memory Backend Role

`InMemoryThingDirectory` should be demoted from architecture driver to backend.

Its role after the refactor:

- test backend,
- embedded/local-only directory backend,
- reference implementation of `DirectoryReader` and `DirectoryPublisher`,
- optional support for `SessionStable` sessions,
- optional support for `watch()` behind `std`.

It must not dictate the public API surface anymore.

## Performance Contract

The new design must make the cheap path cheap by contract.

- Default search returns one batch plus continuation, not total count.
- Default discovery process buffers at most one batch, not the whole result set.
- Default pagination is continuation-based, not offset-based.
- Projection allows `IdOnly` / `Summary` to avoid full TD fetch when possible.
- Exact counts are explicit and may be refused or estimated.
- Reader and publisher APIs are separated so write-path concerns do not burden
  the read hot path.

## Migration Strategy

This is a breaking redesign. Compatibility with the current surfaces is not a
goal.

### Phase 1: Domain API Reset

- Remove `ThingDirectory` as the core public abstraction.
- Remove `DirectoryPage`.
- Replace `ThingDiscovery` with `ThingDiscoveryProcess`.
- Introduce `DirectoryReader`, `DirectoryPublisher`, `DirectorySession`,
  `DirectoryWatch`, `Discoverer`.

### Phase 2: Process Semantics

- Implement live monotonic continuation sessions.
- Remove exact `remaining()` semantics from the scripting process.
- Add projection and count modes to search.

### Phase 3: Backend Refactor

- Rebuild the in-memory backend behind the new traits.
- Add a remote directory client surface for HTTP / CoAP TDD exploration.
- Add direct TD request and Thing Link resolution paths.

### Phase 4: Servient Integration

- Rebuild `Servient::discover` on top of `Discoverer`.
- Rebuild directory publishing around `DirectoryPublisher`.
- Keep local catalog support as an optimization, not as the primary model.

## Immediate File-Level Consequences

The following current abstractions should be considered transitional and marked
for replacement:

- `discovery/src/directory.rs`
- `discovery/src/scripting.rs`
- `discovery/src/query.rs`
- `servient/src/servient.rs` discovery integration

The following documents must be updated after implementation begins:

- `PLAN.md`
- `docs/technical-spec.md`
- `docs/baseline/servient-design-baseline*.md` with superseded notes where
  needed

## Non-Goals

- Preserving `offset + total` as the default directory query contract.
- Preserving `ThingDiscovery::remaining()` exact semantics.
- Keeping `Servient<D>` generic over a directory implementation.
- Forcing remote directories into a local CRUD container abstraction.

## Decision Summary

- Discovery is a process, not a buffered result list.
- Directory is an Exploration service, not a local map.
- Pagination is continuation-based, not offset-based.
- Exact count is opt-in, not mandatory.
- Live monotonic sessions are the default consistency target.
- Local in-memory directory is a backend, not the architecture source.
