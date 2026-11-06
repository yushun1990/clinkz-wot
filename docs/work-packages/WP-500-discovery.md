# WP-500 — Discovery clients — v6.0

## Target and dependency

Compose introduction, exploration, bounded retrieval and TD admission explicitly.
Protocol adapters own native discovery. A Directory-described interaction may
reuse Consumer capabilities when it actually fits; direct retrieval is not a
Property Read. No implicit Directory service, storage engine or universal
Discovery protocol belongs to the Engine.

Exact package dependencies belong to [index.toml](index.toml). This slice uses
WP-100's existing bounded TD admission. The application-composed Consumer example
and integration evidence additionally use the existing WP-200/300/400/600 Host
HTTP path; Discovery's normal dependency graph has no Planning, Servient or
concrete WoT Binding dependency. Their package completion is not a start barrier.

## Authority and admitted increment

Applicable requirements: DISC-CLIENT-001, WOT-SEM-001, WOT-EXT-001,
EXEC-SECURITY-001, SAFE-AUTH-001, SAFE-CANCEL-001, SAFE-CLEANUP-001,
RES-OWNER-001 and RES-WORK-001. The detailed client contract belongs to
[primary flows](../architecture/10-primary-data-flows.md#host-discovery-client-contract).

The `discovery` crate replaces its local in-memory Discoverer, publication,
watch, query, lease and resolver abstractions with one explicit Host HTTP client.
Those APIs had no external production callers and could not retrieve remote TDs.
Their tests are retired, not relabeled as interoperability evidence. This is a
pre-v1 replacement without compatibility facades. The portable crate surface
retains fixed error/result meaning; native retrieval requires `http` (default),
which enables `std`. The umbrella opts in through `discovery-http` independently
of its neutral async feature.

The admitted workflow includes configured TD GET or `/things` array listing,
explicit socket/TLS trust and Basic audience, bounded navigation and admission,
application selection, then separately authorized existing Engine Property Read.
It does not implement registration, storage, search, watches, introduction
protocols, implicit ThingLink traversal, Directory-TD interpretation or arbitrary
TD-described Directory actions. Unknown metadata remains available to the
existing selected semantic and preparation layers.

## Ownership and evidence

The session owns immutable configuration, trust roots, credentials, navigation
and bounded counters. One future owns native I/O, framing, body and temporary TD
admission; one returned page owns its descriptions. Failure never returns a
prefix of a malformed or interrupted page. Stop/failure/final exhaustion release
session trust/credentials; abandoned futures release their actual native owner
and preserve `Interrupted` until stop or disposal. Application-retained pages
and prepared Engine results release independently.

[The executable evidence](../evidence/remote-discovery.md) identifies exact
network, third-party, authority and release claims and limitations. Scoped tests
are `cargo test --locked --offline -p clinkz-wot-discovery`; the explicit example
is `cargo run -p clinkz-wot-discovery --example discover_read -- ...`.
Existing historical Directory performance recipes remain non-gating inputs and
supply no resource/performance qualification for this implementation.

This increment does not close WP-500, M5B, a general security/profile gate or v1.
Independent architecture/public API acceptance uses a fresh review context over
the exact diff and executable evidence before merge.
