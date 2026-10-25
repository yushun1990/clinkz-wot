# WP-400 — Logical runtime and optional preparation facade — v6.0

## Target and dependency

Own instances, current grants, publication, captured behavior, accepted-work progress and cleanup. Finish execution/preparation dependency separation through actual owners. Support final endpoint qualification after closed setup and locally atomic Serving.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: SAFE-AUTH-001, SAFE-PUBLISH-001, SAFE-CANCEL-001, SAFE-CLEANUP-001, SAFE-DROP-001, SAFE-CONCUR-001, SAFE-PROFILE-001, SAFE-OVERFLOW-001.

Required evidence follows actual claims: Publication/cancel races, prepublication ingress, behavior coverage, qualification defects, replacement/late delivery, saturation and acknowledged transfer. Shared physical service close must not cancel siblings. Exact Host/static physical evidence remains separate.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.

## Raw finite shared-instance slice

The execution-only Host slice in `servient/finite` separates the bounded logical
registry from typed native runtimes. Registry count/name limits belong to
`RawFiniteRegistryLimits`; per-runtime input/result opportunities belong to
`RawFiniteLimits`. Revocation and complete same-slot replacement occur at the
logical owner. Accepted work retains the original instance through native
settlement and acknowledgement. Registry mutation/destruction has at most 32
entries and 16 behaviors per instance with names at most 256 bytes; linked
callbacks/local destructors remain trusted cooperative code, not preemptible.
Caller-provisioned runtime slots have independent native/dispatch/cleanup budgets
and no growing Engine runtime queue. There is no aggregate process-RAM claim.

The former runtime-owned-instance constructor and runtime-local revocation are
replaced directly. Core settlement retains original Binding/target identity;
the Binding facet remains native and receives no registry/handler capability.
Affected checks are the finite `native_ingress` integration test (including its
`shared_ownership` cases), Core handler contracts, execution-only dependency and
feature checks, and authority/API projections. TCP/datagram test providers are
ownership/I/O evidence, not production protocol qualification. Historical
Property Read gates and PR #229 evidence keep their original revision scope;
independent acceptance of the changed API/ownership remains necessary. TD
qualification/publication, optional Planning, threading, Action/Event and
constrained execution are separate work.

## Captured validation service on the retained HTTP path

The older single-provider Producer root requires an actual output-validation
facet for Serving, beyond numeric response limits. Preparation and runtime
admission reject a missing facet. Funded Serving/dispatch/delivery progress
rechecks the captured provider/configuration and required service before permits,
behavior or native delivery; loss closes acceptance and retains first cause and
funded native cleanup. Installation-only providers omit the facet. Core checks
result shape/backing bounds before the external codec service.

This fixes a concrete prerequisite on the working HTTP path. It does not move
that path into the shared finite registry or implement final-TD qualification/
publication. Those contracts remain governed by SAFE-PUBLISH-001/PREP-COVER-001.
