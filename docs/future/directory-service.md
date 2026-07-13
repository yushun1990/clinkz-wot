# Deferred Directory Service Design Inputs

Status: non-normative input for a future design revision. This file is not part
of the active v4.6 engine contract and does not authorize a Directory service or
storage implementation in `clinkz-wot-discovery` or `clinkz-wot-servient`.

The active engine revision freezes only the remote-client request, response,
progress, cancellation, resource, and trust boundary. A future Directory service
design must select service crates, deployment topology, storage ownership, and
performance profiles before any item below becomes an implementation
requirement.

## Query and Authorization

The future design must specify:

- construction of a caller-authorized searchable view before filter evaluation;
- fragment, semantic, capability, security-posture, and query-language
  evaluation rules;
- projection and redaction order;
- prevention of hidden-field cardinality and match oracles;
- physical indexes, planner limits, work accounting, and unsupported-query
  behavior.

## Publication and Leases

The future design must specify:

- compare-and-set enforcement for create, replace, renew, and delete;
- authenticated publisher authority and lease-capability validation;
- lease-token issuance, rotation, invalidation, recovery, and secret storage;
- lease expiry, reclamation, persistence, and publication backpressure;
- canonical digest and validation-evidence ownership.

## Snapshots and Pagination

The future design must specify:

- snapshot retention and weak-snapshot eligibility;
- page-token generation, cryptographic binding, expiry, and replay prevention;
- token binding to query, authorization context, projection, ordering, and
  snapshot identity;
- stable ordering and total-count authorization;
- storage and memory budgets for retained versions and active sessions.

## Watches and Service Operations

The future design must specify:

- revision assignment, compaction, and resumability;
- authorization-generation changes and resnapshot behavior;
- server-side watch fan-out, overflow, and slow-watcher policy;
- persistence, replication, high availability, and recovery;
- endpoint hosting, process lifecycle, observability, and production SLOs;
- reference backend scope and conformance evidence.

The active client contract may be used as an input, but the future service
design must not redefine client-visible types silently. Any incompatible change
requires a new active design revision and migration review.
