# WP-300 Property-Read Binding Slice Entry Audit

Status: Review pending

Design revision: v5.0

Admission scope: `WP-300-PROPERTY-READ-BINDING-SLICE`

Verdict: Candidate ready for independent review

## Decision and exact scope

This Category C candidate is the finite execution boundary decided by
workspace issues 0025-0029. It implements no product source. Its sole proposed
runtime claim is one complete registration that advertises Producer Property
Read server execution and consumes one matching WP-200 compiler component.

The slice covers:

1. registration identity, generation, configuration, compiler/server
   compatibility, profile, footprint, ingress, status, overflow, and cleanup
   validation before any protocol side effect;
2. immediate and externally visible route readiness;
3. prepare, readiness, activate, commit-to-closed, permit-gated accept, abort,
   shutdown, response delivery, and terminal acknowledgement;
4. one generation-bearing Property Read request and one owned response
   opportunity;
5. input preservation on registration-call, route-call, and response
   pre-acceptance rejection;
6. explicit cleanup settlement and acknowledged cleanup-transfer rejection
   that returns the complete work envelope; and
7. the same public boundary authored as an application-static
   `no_std + alloc` binding and as a Core-erased `std` host binding.

The exact state projections are:

- `binding-route-lifecycle`;
- `binding-route-readiness`;
- `active-route-acceptance`;
- `response-delivery-ownership`; and
- `binding-call-cleanup-transfer`.

## Public API boundary

The exact added or replaced public items are registered in
`docs/work-packages/property-read-architecture-gate.toml`. They comprise:

- cleanup reservation, phase, transfer request/envelope/acceptance/target, and
  binding-call settlement values;
- registration identity, capabilities, execution support, resource, ingress,
  status, footprint, and state-layout values;
- route identity, reservation, preparation, readiness, activation, commit,
  accept-authority, cleanup, terminal, and delivery values;
- owned `RouteInboundRequest`, `RouteResponseOpportunity`, and
  `RouteInboundResponse`;
- typed route, readiness, and response slots plus the exact
  `PollServerBinding` method family;
- host guards, host call erasure, and the exact route-scoped
  `RouteServerBinding`;
  and
- `HostBindingRegistrationInput`/`StaticBindingRegistrationInput<B>` as the
  complete, recoverable author inputs and `HostBindingRegistration`/
  `StaticBindingRegistration<B>` as the only installable bundle
  representations.

`PollServerBinding::Compiler` is the WP-200
`BindingCompilerExtension` implementation paired with the server. Both
registration constructors consume the existing
`HostBindingCompilerRegistration` or
`StaticBindingCompilerRegistration<B::Compiler>` unchanged. WP-300 does not
define another compiler trait, artifact envelope, host-erasure layer, or
artifact payload rule.

The exact host and static signatures are normative in
`docs/spec/binding-spi.md` and compiled by the two external authoring forms.
Host guard constructors consume the prior-stage guard, so transition cannot
silently change route or reservation identity. Static lifecycle results use
`()` for guard payload because the typed route state remains in the
caller-owned `ServerRouteSlot`.

## Active-authority boundary

The candidate maps the slice to the active Property Read requirements plus the
active indispensable storage, cancellation, constrained-progress, resource,
error, overflow, and host-progress requirements needed by these signatures.

It intentionally does not activate retained v1-deferred requirements. In
particular, the narrow implementation has no:

- `PollClientBinding`;
- client request or subscription slot;
- subscription driver or delivery surface;
- `ProducerEmission`, `BindingPublication`, or `BindingEmissionSlot`;
- `B::EmissionState` or `RouteServerBinding::publish`;
- form contributor;
- collection capability; or
- broad binding progress or workload claim.

The corresponding retained API-inventory rows remain domain-entry input for
broad WP-300. They are not implementation authority for this slice.

## Implementation topology

After independent review and the combined pre-source admission checkpoint, the
only permitted product implementation paths are:

- `core/src/binding.rs`;
- `core/src/lib.rs`.

`core/src/binding.rs` will own the new protocol-neutral registration,
lifecycle, slot, host-erasure, permission, delivery, and cleanup-transfer
surface. `core/src/lib.rs` will expose only the reviewed, uniquely named target
items. Existing `core/src/inbound.rs::{ServerBinding, InboundRequest,
InboundResponse, BindingContext}` remains the legacy-generation boundary and
is unchanged by this slice.

Any needed product change outside these two paths revokes admission pending
an intersecting impact review. This candidate creates none of them and changes
no current product source.

## Legacy separation

The new Core implementation and both fixtures may consume only WP-200 logical
plans, compiler components, artifact envelopes, and compact identities. They
must not:

- depend on `clinkz-wot-protocol-bindings`;
- call `select_form`, `select_affordance_form`, or any other legacy form
  selector;
- receive or rescan a TD or selected Form during execution;
- expose a `Dispatch` capability or handler lookup to a binding; or
- route a target-generation request through legacy `ServerBinding::serve`,
  registration-wide `try_accept`, or `send_response`.

Existing concrete protocols and Servient may retain the legacy generation
until their WP-400/WP-600 migrations. There is no target-to-legacy backflow and
no selection or dispatch adapter in this narrow slice.

## Exact exclusions

This candidate does not claim:

- client invoke or subscribe;
- subscription state, delivery, stop, or terminal behavior;
- Producer emission or publication;
- collection behavior or form contribution;
- broad cancellation/race coverage;
- multiple-route fairness or any performance workload;
- Servient registry, plan-set publication, scheduling, handler dispatch, or
  application orchestration;
- a production protocol, Zenoh, or zenoh-pico implementation;
- broad old-API removal;
- a runtime TD/form-selection path;
- a legacy selector or execution adapter; or
- either cross-package Property Read architecture fixture root.

Optional broad interfaces are omitted rather than implemented as behavior.
Their later addition or replacement requires the relevant domain-entry review.

## Contract fixtures

`tools/design-check/tests/wp300_property_read_binding_schema.rs` is executable
before product implementation. It proves:

- a complete static registration returns the original author input on
  compatibility or footprint rejection;
- immediate readiness reaches committed-closed, permit-gated acceptance,
  response delivery, and explicit route cleanup;
- externally visible readiness preserves state under zero work budget;
- response rejection returns the complete opportunity and payload;
- cleanup-transfer rejection returns the complete work object; and
- host erasure represents both readiness shapes without handler dispatch.

`tools/compile-contracts/wp300-property-read-binding-slice/src/lib.rs` is the
external `no_std + alloc` author. It supplies one real WP-200 compiler,
associated route/readiness/response state, every exact static lifecycle method,
and one complete static registration.

`tools/compile-contracts/wp300-property-read-binding-slice/tests/host.rs` is the
external `std` author. It implements the same server lifecycle through
Core-owned host guards and `HostBindingCallBox`, including an immediate call
and a one-poll external-readiness call, then constructs the complete host
registration.

The fixtures intentionally cannot compile until the reviewed product API
exists. Candidate validation checks their syntax and public-boundary source.
The completion checker must stop first and exactly at the absent
`core/src/binding.rs` boundary.

Neither fixture is a cross-package architecture fixture and neither grants
Servient or production-protocol authority.

## Review and transition contract

The immutable candidate must be one child of
`d8ed500ddba85997d380adc5071818a90150858b` and change exactly the registered
candidate paths. Independent review must:

1. inspect that exact diff and all public signatures;
2. run every registered pre-implementation check;
3. run the executable lifecycle schema;
4. confirm the completion checker fails only because
   `core/src/binding.rs` is absent;
5. mutation-test compatibility mismatch, zero-budget progress, response-input
   preservation, permit lifetime/non-retention, rejected cleanup handoff,
   premature product source, and premature cross-package fixtures;
6. simulate the exact five-file combined pre-source checkpoint; and
7. simulate the exact two-path implementation child and completion-evidence
   boundary.

Review attestation will be
`docs/audits/WP-300-property-read-binding-slice-review.toml`. The attestation
is deliberately absent from this candidate. No product source may begin until
the attestation exists and the exact five-file transition passes
`tools/check-wp300-property-read-binding-slice-entry.sh --admission-ready`.

## Dependency and release verdict

The completed `WP-200-PROPERTY-READ-PLAN-SLICE` is the exact source
predecessor. The narrow WP-300 completion event releases only
`WP-400-PROPERTY-READ-SERVIENT-SLICE`. Broad WP-300 completion remains the
release event for broad WP-400, WP-500, and WP-600.

Open global gates affect this candidate only through a mapped intersection in
requirements, artifacts, state, resources, dependencies, exclusions, or
evidence truth. Compatible changes require named revalidation; disjoint
findings do not trigger broad re-review.
