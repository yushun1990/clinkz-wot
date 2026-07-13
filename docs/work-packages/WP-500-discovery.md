# WP-500 Directory and Discovery Client Runtime

Status: Planned
Design revision: v4.6
Depends on: `WP-300`
Required gates: `GATE-1`, `GATE-2`, `GATE-3`, `GATE-4`, `GATE-5`, `GATE-6`
Owner packages: `clinkz-wot-discovery`, `clinkz-wot-core`

## Scope

Replace the current mixed client/backend Discovery implementation with the frozen engine-side
Directory client contract. The work owns endpoint references, query and publication request
values, result envelopes, opaque revisions and tokens, portable operation slots, lazy Discovery
processes, cancellation, terminal status, incremental page admission, and async adapters over the
portable state machines.

This package is client-only. It does not implement Directory storage, an in-memory Directory,
server-side query evaluation, authorization or redaction policy, token issuance, compare-and-set
execution, snapshot retention, compaction, watch fan-out, endpoint hosting, or service SLOs. Those
concerns remain outside the active design. Work may begin after `WP-300`; it may proceed in
parallel with `WP-400` and `WP-600` once every entry gate is closed.

## Requirements

- `DIR-SCOPE-001`
- `DIR-CONTRACT-001`
- `DIR-AUTH-001`
- `DIR-SNAPSHOT-001`
- `DIR-WATCH-001`
- `API-DIRECTORY-POLL-001`
- `DIR-STREAM-001`
- `API-DISCOVERY-EXEC-001`
- `STATE-DISC-001`
- `CAP-OVERFLOW-001`
- `CAP-STATUS-001`

## Crates and Feature Cells

| Cargo package | Feature cell | Required surface |
| --- | --- | --- |
| `clinkz-wot-discovery` | `--no-default-features` | Owned client values, `PollDirectoryClient`, request and operation slots, and `ThingDiscoveryProcess` manual progress |
| `clinkz-wot-discovery` | `async`, no `std` | `Introducer`, `DirectoryReader`, `DirectorySession`, `DirectoryPublisher`, `DirectoryWatch`, resolvers, and `Discoverer` as adapters over the poll machines |
| `clinkz-wot-discovery` | `std` | Host client adapter conveniences and blocking adapters only where explicitly safe; no service or storage module |
| `clinkz-wot-core` | all required cells | `ProcessEvent`, `ProcessTerminal`, `CleanupOutcome`, limits, errors, deadlines, cancellation, and security selection values |
| `clinkz-wot-td` | all required cells | `TdDocument`, source metadata, validation, and lossless document ownership |

Concrete remote transports live outside `clinkz-wot-discovery`. The package may consume
protocol-neutral core and TD values, but it must not depend on `clinkz-wot-servient`, a concrete
transport, a Directory service, or a storage backend.

## Public API and Data Migration

- Add the frozen identities `EntryId`, `DirectoryRevision`, `EntryRevision`, `LeaseToken`,
  `PageToken`, `WatchCursor`, and `PolicyGeneration`. Secret-bearing `LeaseToken` output is
  redacted from `Debug`, `Display`, errors, source envelopes, page items, and watch items.
- Replace the current query and result carriers with `DiscoveryFilter`, `DirectoryRef`,
  `DirectoryQuery`, `DirectoryPage`, `DirectoryChange`, `DirectoryTerminal`,
  `ExpectedRevision`, `PublicationAuthority`, `PublishOptions`, `LeaseRequest`, and
  `Publication` as assigned in `docs/api-ownership.csv`.
- Add `DirectoryQueryRequest`, `DirectoryWatchRequest`, `DirectoryPublicationRequest`, and
  `ThingResolveRequest`. Each owns its endpoint, operation input, deadline and cancellation view,
  security selection, and immutable resource-profile identity.
- Add caller-owned `DirectorySessionSlot`, `DirectoryWatchSlot`,
  `DirectoryPublicationSlot`, and `ThingResolveSlot` plus the portable
  `PollDirectoryClient`. Slots are bounded and generation-bearing; start failure leaves a slot
  empty, terminal completion consumes its generation, and cancellation retains a complete cleanup
  outcome.
- Replace `ThingDiscoveryProcess` and `DiscoveryProcessState` with the terminal-bearing state
  contract. Portable polling yields `ProcessEvent`; host stream adapters retain the one terminal
  status for an accessor or completion future instead of reducing all endings to `None`.
- Implement host `Introducer`, `DirectoryReader`, `DirectorySession`, `DirectoryPublisher`,
  `DirectoryWatch`, `ThingDescriptionResolver`, `ThingLinkResolver`, and `Discoverer` as adapters
  over the same slots. They must not carry independent cancellation, error, or terminal semantics.
- Return `TdDocument` at the Directory-native boundary. Scripting-compatible adapters may expose a
  bare TD view; source-aware Rust APIs retain endpoint, retrieval, revision, digest, validation,
  freshness, and policy-generation metadata.

The current `Revision` splits into `DirectoryRevision` and `EntryRevision`; the current
`ContinuationToken` becomes the query-bound `PageToken`; `DirectoryBatch` becomes
`DirectoryPage`; and `DirectoryRegistration`/`RegistrationAck` are replaced by typed publication
requests and `Publication`. These are intentional data migrations rather than aliases.

## State and Ownership Migration

- Implement `Created -> Running -> Stopping` and the retained terminal states `Completed`,
  `Cancelled`, `TimedOut`, `Overflowed`, and `Failed`. The first poll starts work; cancel and drop
  prevent new backend work and transfer any remaining cancellation to a bounded cleanup owner.
- Give each query, watch, publication, and resolve operation exactly one slot generation and one
  cleanup owner. Outer errors are reserved for invalid calls that do not change state; process
  failure is returned once as `ProcessEvent::Terminal(ProcessTerminal::Failed(_))`.
- Validate page-token reuse against endpoint identity, query digest, authorization-context
  generation, projection, snapshot mode, and previous response metadata. Never reinterpret or
  regenerate an opaque remote token.
- Incrementally decode, validate, and transfer one page item at a time. Complete encoded page
  bytes, a generic decoded page DOM, and lossless copies of every admitted TD must not coexist.
  Partial pages remain private until metadata and ordering are complete and are discarded on
  failure.
- Terminate a watch on compaction or authorization-context generation change. Retain ordered
  directory and entry revisions, and require a new snapshot before a caller may claim gap-free
  continuity.
- Apply count, byte, scratch, page, queue, and cleanup limits before publication. Default overflow
  is terminal and non-lossy; an explicitly lossy profile records loss without switching to an
  unbounded queue.

## Old API Removal

- Remove public `backend` and `storage` modules and the `InMemoryDirectory`,
  `SharedInMemoryDirectory`, and `shared_in_memory_directory` exports from
  `clinkz-wot-discovery`.
- Remove `LocalDiscoverer`, `DirectoryRef::Local`, and any constructor or default that assumes an
  in-process Directory. Discovery introduction must resolve an engine-side client endpoint.
- Remove the service-shaped `DirectoryReader::get` and `open_search` contract and the current
  backend-owned `DirectorySession::next -> Option<DirectoryBatch>` contract. Replace them with the
  frozen query/session request and progress operations.
- Remove `DirectoryFilter`, `DirectoryItem`, `DirectoryBatch`, `DirectoryRegistration`,
  `RegistrationAck`, `DirectoryPatch`, `ContinuationToken`, and the undifferentiated `Revision`
  from the public target surface after callers migrate to the frozen values.
- Remove `ThingDiscoveryProcess::next -> Result<Option<Thing>, _>`, `ProcessState::{Pending, Open,
  Done}`, and error-only terminal access. Completion, cancellation, timeout, overflow, domain
  terminal, and failure must remain distinguishable.
- Remove `DiscoveryError` variants and retry behavior that encode a local storage implementation or
  collapse the frozen `CoreError` categories.

No deprecated feature may retain a Directory service or storage SPI inside this engine package.

## Evidence

- `directory-client-public-surface`: compile fixtures for owned values, all operation slots, the
  poll contract, and its async adapters in every required feature cell.
- `directory-client-scope`: dependency and public-item inspection proving that no service,
  backend, storage, local-Directory default, or server policy remains in the active engine crates.
- `directory-incremental-admission`: fragmented input, one-over-limit, partial-page rollback,
  bounded resume, no duplicate decode, and peak-residency evidence.
- `directory-terminal-state`: exhaustive normal, domain, timeout, cancellation, overflow, remote
  failure, and post-terminal polling evidence with retained terminal status.
- `directory-cancel-and-overflow`: cancellation races, full queue behavior, loss accounting,
  cleanup ownership, and stale slot generation evidence.

Scripted remote-client fixtures must additionally cover exact revision encoding, typed publication
authority, token rotation and redaction, stable ordering, query-context token rejection, weak and
strong snapshots, empty intermediate page rejection, compaction, lease-expiry reporting, and
policy-generation termination. They must not test or imply a server implementation.

## Performance Workloads

- `PERF-DIR-001`: Directory request planning.
- `PERF-DIR-002`: bounded 128-item page admission.
- `PERF-DIR-003`: incremental 128-item page admission and peak residency.
- `PERF-DIR-004`: hierarchical client accounting contention.
- `PERF-DIR-005`: watch-change admission.
- `PERF-DIR-006`: publication request planning.
- `PERF-DIR-007`: cancellation to retained local terminal state.
- `PERF-DIR-008`: bounded-buffer overflow.
- `PERF-DIR-009`: byte and structural scaling.
- `PERF-DIR-010`: independent progress for unrelated sessions.
- `PERF-DIR-011`: query, watch, and publication client characterization.

These workloads measure only engine-side client construction, decoding, admission, progress,
cancellation, and overflow against the deterministic scripted adapter. Network latency, Directory
execution, storage, snapshots, and service SLOs are excluded. `PERF-DIR-011` is characterization
only.

## Completion Conditions

- `WP-300` is complete, all entry gates remain closed, and every public Directory item has one
  frozen owner and path in `docs/api-ownership.csv`.
- The three feature cells expose the documented values and adapters; no-default builds have useful
  poll progress and `async` alone pulls no executor.
- All client state machines, token-context rules, typed authority rules, incremental admission,
  cleanup, and terminal behavior pass the named evidence with no unbounded collection or second
  full-page staging copy.
- `clinkz-wot-discovery` has no service/storage public surface or dependency, and Servient
  construction can operate without a Directory capability.
- Every listed performance workload has a fixture-locked result accepted by
  `tools/performance-harness` and satisfies its applicable absolute budget or characterization
  contract.
- Compile-fail and dependency inspections prove the removed local Directory, backend, storage, and
  implicit-end APIs are unavailable.
