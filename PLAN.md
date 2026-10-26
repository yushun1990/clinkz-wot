# ClinkZ-WoT roadmap

Design target: v6.0 semantic Engine over Binding-owned native runtimes, selected
by ADR-0025. Mainline authority activates through the independently reviewed
atomic migration; implementation and release qualification are separate.

Release target remains v1 with both WoT roles and Property/Action/Event capability
families, described collections, security, client-only Discovery/Directory,
portable shared semantics and Host HTTP/HTTPS production delivery. Zenoh/pico and
real embedded protocol/hardware qualification remain separate future work.

## Frontier and ordering

The cleaned baseline contains narrow Property Read implementations and historical
scoped evidence. They establish useful current behavior, not the general SPI or
successor acceptance. The v6 architecture replaces the fixed v5 requirement set,
eager per-instance plan-set and route-scoped native model. Exact authorities and
prior-evidence dispositions belong to the registered documents and work-package
index, not this roadmap.

Engine interaction semantics, ownership and lifecycle drive implementation.
Concrete Bindings validate and refine those boundaries; no one protocol sets the
Core abstractions or default development sequence. Action, notification and
publication slices have no fixed internal ordering: select useful work by
semantic need and executable evidence without creating a new gate.

1. Integrate one coherent successor authority before dependent general SPI work.
2. Stabilize semantic/data/security and native capability boundaries, then replace
   narrow physical APIs where needed without compatibility facades. Preserve
   useful independent qualified work, including narrow HTTP HTTPS/security and
   Discovery client progress. No global protocol-generality proof gate applies.
3. Complete optional preparation and source-free execution composition with real
   owner separation. Develop useful interaction slices, including Action lifetime
   and notification control/response/stop, with actual publication/cancellation/
   release evidence. A minimal observation slice is valuable implementation
   evidence for notification work, not a separate global architecture gate.
4. Qualify concrete HTTP/HTTPS supported capabilities, client Discovery and
   supported Host/constrained profiles. Measure actual resources/performance;
   compile-only portability is not hardware fitness.
5. Aggregate WP-400, WP-500 and WP-600 through WP-700, complete independent v1
   conformance/security/release review, then make a technical release claim.

Exact package and capability dependencies are owned by
[the work-package DAG](docs/work-packages/index.toml). Runtime contracts no longer
depend on completed optional preparation. Implementations cannot treat missing
predecessor behavior as available merely because a package interface is specified.

## Milestones

| Milestone | Status | Completion boundary |
| --- | --- | --- |
| M0 — Repository-grounded governance | CLOSED | Stable ownership and recoverable Git/remote workflow |
| M1 — Architecture authority and closure | IN_PROGRESS | One revision, explicit prior dispositions, checks and independent review |
| M2 — Foundation and semantic Core/TD | IN_PROGRESS | Shared semantics, data/security, owner/work/time and supported profiles |
| M3 — Preparation | IN_PROGRESS | Actual pairing, pure/deferred construction, coverage and reuse obligations |
| M4 — Binding capability/lifecycle | IN_PROGRESS | Open native facets, recipient authority and physical retirement |
| M5A — Servient runtime | IN_PROGRESS | Logical publication, dispatch, accepted work and cleanup |
| M5B — Discovery client | OPEN | Bounded trusted retrieval/continuation and exact client scope |
| M5C — Host HTTP/HTTPS production | OPEN | Real supported mapping and security/lifetime qualification |
| M6 — Integration/conformance | OPEN | Joined capability, profile, resource/performance and removal evidence |
| M7 — v1 release | OPEN | Independent technical release acceptance after M6 |

No milestone closes because this target is accepted or historical CI is green.
Source, tests and exact review evidence establish implemented/admitted capability;
GitHub owns live branch/PR/CI state. This roadmap stores no task continuation,
execution checklist or next action.
