# Reusable Plan semantics versus Thing-generation ownership

Status: OPEN

## Question and counterexample

Does the current one-Plan-Set-per-produced/consumed-handle-generation contract
silently turn a sound **lifecycle owner** into a mandatory **physical-copy and
compilation unit**? A Plan describes executable interaction semantics; a Thing
identifies a logical physical or virtual affordance-bearing object. They can
coincide for a small deployment without being intrinsically the same identity.

Consider 10 million otherwise homogeneous electricity meters reporting through
Zenoh. An application may expose fleet-level virtual Things, individual meter
Things, or both, depending on discovery, permissions and interaction semantics.
The engine must not prescribe a physical-device-to-Thing cardinality, nor infer
one transport session or one independent complete compiled image per meter.
Conversely, using one fleet Thing must not erase per-meter identity, provenance,
authorization, routing or independently observable WoT meaning.

The scalability question is not whether a correct single-Thing Runtime can
allocate and release its Plan Set. It is whether its public/retained identity
structure makes identical immutable execution work inherently unreusable when
many compatible logical instances are needed.

## Current evidence and boundary

- [Planning's active `PLAN-COST-001` and `PLAN-SET-001`](../docs/spec/planning.md)
  compile one logical plan per effective form/operation, sharing it among
  binding candidates **within the build**, while assigning each published
  handle generation one Servient-owned Plan Set lifecycle record. Neither
  clause demonstrates or forbids separate cross-Thing physical storage reuse.
- The [compiled-plan lifecycle](../docs/architecture/30-compiled-plan-lifecycle.md)
  correctly scopes publication, pin/operation leases, draining and reclamation
  to an owning generation. Plan identities and artifact references carry
  generation/target/binding context. Moving invariant bytes into a shared object
  must not accidentally share these distinct authorities.
- The accepted [WP-200 Consumer aggregate evidence](../docs/evidence/WP-200-consumer-property-read-aggregate.toml)
  measured 4,550 retained Host output bytes (4,238 static) for **four properties
  and three coordinates**, in one unpublished fixture. At 100,000 separately
  materialized identical-size Host outputs, that conditional multiplication is
  455 MB decimal, *before* Servient records, registrations, transport, active
  calls or allocator overhead. This is not a deployed per-device measurement,
  a generic Plan size, or proof that a fleet needs 100,000 Things.
- The [Binding SPI](../docs/spec/binding-spi.md) already assigns native session
  multiplexing, protocol-side routing and correlation to a binding. Thing
  count, transport connections, active operations and physical device count
  are different dimensions.
- The active [WP-400 Consumer Runtime candidate, PR #165](https://github.com/yushun1990/clinkz-wot/pull/165)
  deliberately proves **one consumed generation and one active call**.
  Its scope cannot establish or rule out multi-Thing program sharing or fleet
  deployment costs.

## Architectural risk

A design can satisfy every per-Thing resource cap and exact cleanup invariant
yet impose O(N) repeated immutable program bytes for N Thing instances,
where only O(N) minimal instance metadata is intrinsically necessary.
Encoding Thing/generation identity, authority, or concrete endpoint ownership
directly into every reusable byte may make that duplication hard to remove later.

The missing potential boundary is between:

1. **Reusable execution definition**: operation/form semantics and immutable
   compiled portions that are demonstrably identical under a checked
   compiler/schema/security/binding/configuration dependency identity.
2. **Execution binding/context**: logical Thing identity, resolved per-instance
   target and variable values, authorization/credentials, routing, and
   configuration or source-dependent details that cannot safely be shared.
3. **Runtime ownership**: plan-set publication, exact generation, leases, calls,
   capacity reservations, cancellation, cleanup and retirement.

These are investigation categories, **not** selected Rust types, API changes, or
a finding that any particular existing field is misplaced.

## Alternatives to test rather than assume

- Independent materialized Plan Sets per Thing: simplest ownership, but measure
  repeated immutable storage and rebuild cost before declaring it sufficient.
- Aggregate/virtual fleet Thing with parameterized operations/events: can
  preserve one externally meaningful Thing over many physical sources, but
  must explicitly preserve source attribution, identity, security and
  semantics; it is not a replacement for applications requiring distinct Things.
- Distinct per-Thing generation/lease owners referencing a checked shared
  immutable compiled program plus per-instance bindings: potential reuse without
  merging Thing identities; requires exact compatibility keys, safe teardown,
  immutable storage accounting, and Host/static evidence.
- Later bounded lazy materialization or eviction: only if admission,
  source availability, dependency generations and deterministic failures can
  be proved; inactive PLAN-LAZY/CACHE requirements are **not** reactivated.

Same model/type name alone never proves safe sharing. Endpoint URIs, template
values, policy, schema, security, binding configuration or compiler generation
may distinguish otherwise identical meters.

## Future decision and falsification questions

- Which bytes are invariant under a change of physical device, Thing identity,
  route, authority, configuration, security policy and generation? Measure the
  smallest legitimately per-instance retained portion, not merely `size_of`.
- Can 1, 1,000, 100,000 compatible instances reuse identical immutable
  execution work without duplicating those bytes while retaining independent
  Thing/operation semantics? Which allocations and CPU costs actually scale
  with instance count, type count, active-call count and network traffic?
- Can the runtime retain one generation/cleanup owner per Thing while sharing
  immutable program storage safely? How are shared bytes charged once,
  referenced, invalidated and finally reclaimed without leaking ownership or
  crossing security/binding/deployment boundaries?
- Can a fleet-level Thing and individually addressable Things both be expressed
  without custom raw-protocol bypasses? How is native Zenoh source identity
  authenticated, routed and exposed at the WoT semantic boundary?
- Does a first-slice API or data layout accidentally *require* physical
  exclusivity of plan/program bytes, precluding a future compatible
  representation? Challenge this during relevant reviews, without assuming the
  sharing implementation is an admission prerequisite.

## Scope, trigger and owners

**Non-blocking for PR #165 and the first single-Consumer WP-400 proof** unless
its exact public or normative contracts positively rule out future compatible
representation. No modification to active PlanId/PlanSetGeneration semantics,
PLAN-SET-001, source authority, API ownership, resource defaults, work-package
admission, CI gate, milestone or production code is authorized here.

Revisit with actual published WP-400 Runtime/Host Zenoh measurements **before**
freezing broad multi-Thing scalability claims or an irreversible shared-plan
representation. If evidence demonstrates a material contradiction, escalate
through the existing architecture process. Exact normative owners to evaluate
then are Planning semantics/identity, the Servient plan-set lifecycle and
resource accounting, Binding multiplexing and source attribution, plus any
affected work-package/architecture evidence.

Related: [topic 0076](0076-composable-capabilities-and-runtime-boundaries.md)
covers broader product/runtime representation, but not this explicit
Thing-identity versus reusable-program boundary. This OPEN topic records the
counterexample, not a chosen architecture or extra gate.
