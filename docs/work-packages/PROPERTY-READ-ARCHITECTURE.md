# Property-Read Architecture Gate

Status: READY

Gate id: `PROPERTY-READ-ARCHITECTURE`

Manifest: `docs/work-packages/property-read-architecture-gate.toml`

## Scope

This gate is the first executable proof that the v4.9 planning, Core handler,
Protocol Binding, and Servient boundaries compose without an ownership
shortcut. It uses one Thing Description, one readable property, one immutable
logical plan, one binding-owned artifact, one prepared route, one statically
registered synchronous handler, one request, one response, and complete route
and request cleanup.

It is architecture evidence, not a demonstration application or a replacement
for package-local completion. The package order remains
`WP-100 -> WP-200 -> WP-300 -> WP-400`. The executable path contains the
completed WP-200-owned Producer-route correction after WP-300 and requires one
distinct compiler-owned route-reservation correction before WP-400 because
successor reconstruction exposed those two concrete handoff gaps. That second
correction is independently reviewed, exactly admitted, complete, integrated,
and default-validated. The resulting WP-400 slice is also independently
reviewed and exactly admitted; implementation
`a993555f3cbd2bc7026423f34ed5620f3a2e058f` now completes its package-local
lifecycle evidence. Verified integration of that implementation/completion
chain remains the aggregate fixture candidate's successor-release boundary.
ADR-0013 permits only exact tranches registered in the manifest to cross
incomplete package boundaries after their own admission reviews. No manifest
record alone is implementation admission.

The aggregate candidate also depends on D48's validator-convergence boundary.
The gate manifest owns candidate history, review, pre-source, implementation,
completion, and absent/present-source facts as declarative transition records.
The generic transition validator consumes those records; focused validators
and public-boundary fixtures continue to own provenance, lifecycle, resource,
profile-parity, and runtime behavior. The route-reservation and WP-400
validators remain active parallel oracles until the registered equivalence
review passes. A later aggregate candidate must use this generic transition
path and may not add another tranche-specific topology branch.

Evidence claims advance in this order:

1. package-local slice constructibility and ownership;
2. the mock cross-package Property Read architecture gate;
3. a real Zenoh Property Read smoke using the same admitted boundaries; and
4. broad workload and release-readiness evidence.

No earlier rung implies a later one. In particular, the mock gate makes no
production-protocol, multi-route availability, deployment, performance, or
release claim.

The WP-100 slice is one deliberately narrow synchronous seam. It adds only the
root-re-exported `ReadPropertyHandler` trait in `core/src/handler.rs` and
`core/src/lib.rs`, and composes it with the already implemented
`HandlerContext` and `StaticHandlerRegistration` plus the existing production
`InteractionInput` and `InteractionOutput` values. It does not replace those
reused values or add async/step traits, host erasure, storage, or execution.

The WP-300 slice similarly installs one complete bundle rather than a split
compiler or server half, but advertises only Producer Property Read. It covers
immediate and externally visible readiness, committed-closed route ownership,
borrowed-permit acceptance, one response opportunity, and explicit cleanup in
host and application-static forms. Optional client, subscription, emission,
collection, contributor, multi-route, workload, Servient, and production
protocol behavior remains broad work. Default methods for unadvertised roles
must reject before state or side effects and do not constitute behavior
evidence.

The Producer-route projection corrects the only concrete intersection found
while preparing WP-400: the implemented Property Read algorithm is private and
hard-codes `ConsumerCall`, while the complete registration is Producer-only.
The correction preserves that completed Consumer behavior and exposes only a
bounded `ProducerRoute` constructor plus opaque cursor. Its external fixture
must borrow the compiler from the complete WP-300 registration and carry the
real plan output directly into `PrepareInput`; it cannot synthesize any gate
logical plan, artifact, or preparation type in fixture code. Its local proof
does use a fixed mock route reservation and therefore does not by itself
release WP-400.

The route-reservation projection closes that distinct remaining boundary. The
complete mock registration's concrete compiler must attach a canonical
`RouteReservationIdentity` to its real Producer-route artifact, Core must
preserve and role-check it through typed and erased paths, and the runner must
read it only from the admitted envelope. The runner may not construct a
collision domain or endpoint key. This correction keeps broad form
contribution, capability indexing, collision tables, Servient lifecycle, and
protocol behavior outside its scope.

## Tranche DAG and entry points

```text
WP-100-PROPERTY-READ-HANDLER-SLICE
    -> WP-200-PROPERTY-READ-PLAN-SLICE
    -> WP-300-PROPERTY-READ-BINDING-SLICE
    -> WP-200-PROPERTY-READ-PRODUCER-ROUTE-PROJECTION
    -> WP-200-PROPERTY-READ-ROUTE-RESERVATION-PROJECTION
    -> WP-400-PROPERTY-READ-SERVIENT-SLICE
    -> PROPERTY-READ-ARCHITECTURE
```

This diagram reflects the current manifest. Every package-local tranche is now
`complete`/`approved`; the WP-400 implementation is the immediate child of the
tree-equivalent pre-source admission merge and passes its registered static,
host, and portable cells. The aggregate gate is therefore `ready`, while its
planned fixture roots remain absent pending their own exact candidate, review,
and admission lifecycle.

The gate blocks:

- broad `WP-100-HANDLER-ENTRY`, while leaving its current exact prerequisite
  tranches and the property-read handler slice independently reviewable;
- `WP-300-BROAD-ENTRY`, except its named property-read binding slice; and
- `WP-400-BROAD-ENTRY`, except its named property-read Servient slice.

It does not block M1 documentation convergence, corrective work, the current
`WP-100-HANDLER-CONTEXT` candidate lifecycle, WP-200 planning work, or admission
preparation for the registered exact slices. WP-500 and WP-600 remain
downstream of WP-300 and are therefore indirectly protected from becoming the
first architecture proof.

The exact downstream release events are:

- completion of `WP-300-PROPERTY-READ-BINDING-SLICE` releases only
  `WP-200-PROPERTY-READ-PRODUCER-ROUTE-PROJECTION`;
- completion of that projection exposed the missing compiler-owned canonical
  route reservation and releases only
  `WP-200-PROPERTY-READ-ROUTE-RESERVATION-PROJECTION`;
- the independently reviewed completion and verified default integration of
  that production metadata correction, including the real
  `compiler -> admitted envelope -> BindingRouteKey` evidence, jointly release
  only candidate, review, and pre-source-transition preparation for
  `WP-400-PROPERTY-READ-SERVIENT-SLICE`, whose candidate must also close the
  first-entry table below;
- completion and verified default integration of that exact WP-400 slice
  release only aggregate Property Read fixture candidate/review and admission
  preparation; and
- broad WP-300 completion releases broad WP-400, WP-500, and WP-600.

Downstream preparation may reduce uncertainty before those events but grants
neither source admission nor executable vertical-progress credit.

## Exact scenario

Both runtime cells execute the same observable scenario:

```text
TD fixture
  -> shared planner
  -> immutable property-read logical plan
  -> mock binding compiler
  -> binding-owned artifact with canonical route-reservation metadata
  -> prepared and committed-closed route
  -> Servient serving publication
  -> permit-authorized mock acceptance
  -> protocol-neutral accepted request
  -> Servient route and handler selection
  -> one static property-read handler
  -> protocol-neutral response opportunity
  -> mock response delivery
  -> request and route cleanup
```

The `no-default-manual` cell uses caller-owned slots and explicit
`WorkBudget`. The `std-host` cell uses the public object-safe registration and
call boundaries. The `async-no-std` cell is a compile-only projection because
the gate does not select an executor; for the WP-100 slice it proves that the
synchronous trait and borrowed static registration remain available when the
portable async feature is enabled, not that this slice adds the separate
`AsyncReadPropertyHandler` or `StepReadPropertyHandler` contracts.

This gate does not claim the incapable-target build required by the broad
`affordance-target-no-atomics` evidence. Requiring that full-Core proof here
would make the only broad-entry exemption depend on legacy `Arc` migrations
that broad entry itself blocks. The no-atomic proof, `AcceptHint` admission,
and final `InteractionInput` storage migration remain broad WP-100 work and
completion claims. Any later change to a production boundary reused by this
gate triggers the normal change-control impact review and reruns this evidence.

## WP-400 first-entry input closure

For this gate, the first legal WP-400 route entry is the exact
`binding-route` transition
`Absent --begin_prepare--> Preparing`. It occurs inside the expose
transaction after `Draft --expose--> Preparing` has transferred transaction
ownership. The transition may invoke `RouteServerBinding::prepare` or
`PollServerBinding::start_prepare` only after its route record and every
pre-side-effect reservation are complete.

`PrepareInput` is therefore a derived WP-300 call operand, not the complete
WP-400 entry boundary. The future public or private Rust helper that assembles
the record remains a WP-400 implementation detail, but it must consume the
following closed production inputs and may not acquire another semantic root
input after binding preparation starts:

| Required class | Semantic creator and production carrier | Entry treatment | Fixture rule |
| --- | --- | --- | --- |
| Producer logical route | Planning, through the real `PlanBuildOutput` logical plan and `BindingArtifactRef` | Retain the Thing id, Property target, `ReadProperty` operation, plan id, and plan-set-qualified reference | The runner uses the planner output object; it does not restate a plan, target, or reference |
| Producer-route artifact metadata | The concrete binding compiler, role-checked and preserved by Core/Planning in the admitted artifact envelope | Retain the opaque artifact and its identity, role, admitted footprint, and canonical route reservation | The route reservation must arrive from the admitted envelope after the route-reservation correction; equal fixture bytes are not provenance |
| Complete binding registration | The binding author through a Core-validated host or static registration, frozen by the Servient registration snapshot | Match binding id/generation, configuration, compatibility, capability, execution form, server, resources, ingress, and status to the artifact identity before side effects | No bare server/compiler or separately recreated registration identity is accepted |
| Produced Thing and generation | The logical plan supplies `ThingId`; WP-400 allocates the produced Thing slot/generation and retains it in `ExposedThingRecord` | Validate one Thing/plan-set generation and use it for every derived record and authority | A test may seed a legal root generation only through the production allocator/constructor; it may not patch a private record |
| Property Read handler coverage | The application supplies the real WP-100 handler and footprint; WP-400 freezes `StaticHandlerRegistration` in the static cell or its own private `PropertyReadHandlerRecord` erasure in the host cell | Prove exact Thing/Property/operation coverage before the route can become publishable | The handler and its result may be fixture implementations, but the runner cannot fabricate handler lookup or `HandlerContext`; the narrow slice does not require the unimplemented broad Core `HostHandlerRegistration` family |
| Servient admission policy | Foundation/Servient configuration supplies resource limits, work/deadline/clock policy, and cleanup/status capacity | Reserve the plan/runtime record, route/guard/readiness/accept, ingress, status, and cleanup obligations required before the first binding side effect | Numeric test policy is a legal root input; implicit or post-side-effect capacity is forbidden |
| Compiled plan-set ownership | WP-400 installs the real plan output in `CompiledPlanSetRecord` and retains a `PlanSetLease` | Keep the referenced logical plan and artifact live through route, request, response, cleanup, and reclamation ownership | The runner cannot keep a detached reference while bypassing the plan-set record/lease |
| Route preparation assembly | WP-400 allocates the route generation and derives one `BindingRouteKey` and `PrepareInput` from the matching plan, artifact, registration, reservation, and admitted route-state footprint | Publish `BindingRouteRecord: Preparing` only after every identity check and reservation succeeds | The architecture runner enters through WP-400 product code; it does not construct the route key or `PrepareInput` itself |
| Activation and cleanup ownership | WP-400 derives one reserved `ServingActivationAuthority`, matching route accept-lease identity/capacity, and complete cleanup reservations from the produced Thing, route, plan-set, and policy | Retain them unavailable until their legal later transitions; no binding work starts without rollback ownership | The runner cannot inject an authority, permit, lease, cleanup record, or status-only substitute |

Host-erased and application-static representations may store these values
differently, but their semantic identities, generation checks, resource
deltas, and transition outcomes must match. An accept lease or other derived
object may be instantiated later when its transition requires it, but all of
its external identity and capacity inputs are closed at this boundary.

Prepared/readiness/active/committed guards, inbound match and request,
correlation, security result, handler context/input, response opportunity,
and serving publication state are later lifecycle outputs. `AcceptHint`, final
`InteractionInput` storage, subscription/emission, multi-route availability,
production protocol, and workload maturity remain excluded broad work rather
than missing inputs to this single-route closure.

This is a one-time closure review owned by the
`WP-400-PROPERTY-READ-SERVIENT-SLICE` candidate and its normal independent
admission review. It is not a new tranche or a permanent cross-package gate.
Any unowned row, fixture-only substitute, illegal recomputation, generation
loss/mismatch, resource or cleanup reservation after side effects, or
host/static semantic divergence blocks WP-400 source admission.

The active candidate is frozen by
`docs/audits/WP-400-property-read-servient-slice-entry.md`. It selects explicit
`Context`/`WorkBudget` progress through `StaticServient::step` and
`Servient::step`, returning `StepStatus<()>` in this narrow proof. That unit
value deliberately avoids activating the v5-deferred runtime-event/status
family; runtime evidence observes the real binding request/response state and
public lifecycle views instead. Static construction consumes one complete
typed registration and `StaticHandlerRegistration`; host construction
consumes one complete erased binding registration while Servient privately
owns the synchronous handler erasure.

## Declarative transition validation

`docs/work-packages/property-read-architecture-gate.toml` carries schema-v1
transition records for the route-reservation and WP-400 slices. Each record
declares its immutable candidate/correction chain, exact changed paths, review
commit paths, pre-source topology, exact implementation delta, completion
evidence linkage, and candidate/implementation source boundary. The generic
engine checks these facts against Git history, the artifact and governance
registries, review attestations, and current completion evidence.

The equivalence claim has four required dimensions: valid current state,
negative mutations, commit topology, and current completion evidence. The
legacy instance branches reached through `check-work-packages` and the focused
entry/completion validators remain executable and run in parallel; this
convergence does not retire their topology, semantic, or behavioral checks.
Once the independent equivalence attestation is registered, adding another
transition that uses the same invariant categories changes declarative records
and focused behavior evidence, not generic-engine control flow.

## Fixture topology

The future fixture has two package roots:

- `property-read-binding` depends only on the TD/Core/Planning interfaces
  required to implement a binding compiler and producer server role. It cannot
  depend on Servient or application handler modules.
- `property-read-runner` composes the TD, planner, mock registration, Servient,
  and handler, and owns the runtime assertions.

Only protocol-frame values, deterministic I/O state, and instrumentation
probes are fixture adapters. Separately, the fixture may choose deterministic
values at a real production root input, including TD contents, binding
configuration and registration seed, initial generations, handler behavior,
and explicit resource policy. A fixture may not replace or restate a
production-carried logical plan, binding artifact/metadata, complete
registration, compiled plan-set lease, route key/preparation input, route
guard or activation
capability, accepted request, handler context/input/output, response
opportunity, or cleanup owner. If any one of those production boundaries is
unavailable or unconstructible, the owning slice remains blocked and the
difficulty becomes design feedback.

The canonical collision domain and endpoint key are binding compiler output,
not runner fixture data. The binding fixture may derive its deterministic mock
identity from protocol-local configuration; the runner obtains it only from
the real admitted artifact envelope.

The fixture directories are not created until their owning tranche has an exact
reviewed candidate. Placeholder crates would falsely imply a constructible
boundary. The WP-200 compiler authoring contract is a package-local external
compile fixture, not either cross-package architecture fixture root.

## Mandatory runtime evidence

The completion check must prove:

- the TD fixture is read during planning and not at runtime;
- the logical plan is immutable after admission, and the binding artifact is
  sufficient without the TD;
- every row in the first-entry closure is supplied through its production
  carrier before the first binding side effect;
- no acceptance occurs before Servient publication;
- only Servient selects the admitted route and handler;
- the static handler is invoked exactly once with protocol-neutral input;
- its protocol-neutral output consumes one response opportunity and reaches
  the mock binding exactly once;
- Thing, plan, binding, compiler-produced route reservation,
  request/correlation, and generation identities remain consistent;
- deactivation prevents new acceptance; and
- route/request counts return to zero with no retained hidden handler, request,
  permit, response, or cleanup owner.

The first version does not exercise subscription, write/action payloads,
security execution, fallback, retry, cancellation races, multiple responses,
production networking, or performance budgets.

## Mandatory compile and source evidence

Positive fixtures prove that both public binding profiles can construct the
required registration and that the runner uses production boundary types.
Negative compile/source checks prove:

- the mock binding package has no Servient dependency;
- binding construction accepts no handler, dispatch callback, registry view,
  or mutable plan-set capability;
- activation permits cannot be independently constructed, cloned, copied, or
  retained beyond their borrow;
- the runner cannot construct `BindingRouteKey`, `PrepareInput`, an activation
  authority/lease, or cleanup ownership outside WP-400 product code;
- fixture-restated artifact metadata, a dropped generation, an unrelated
  reservation, a partial registration, and pre-reservation side effects are
  rejected in both runtime profiles;
- the mock cannot rescan the TD or construct a runtime logical plan; and
- the runner cannot construct a route reservation, collision domain, or
  endpoint key; and
- the `async-no-std` portable surface compiles without choosing an executor.

The target runner also poisons the legacy form selector,
`ServerBinding::serve`, and `Dispatch` boundaries and records zero calls. This
is the slice-level no-backflow proof; WP-600 later removes concrete call edges
and WP-700 proves final source and public-surface absence.

Runtime tests own state-transition and cleanup claims. Compile-fail tests own
capability absence and construction boundaries. Dependency/source inspection
owns crate direction and the absence of hidden dispatch imports.

## Global-gate impact

The open global gates and this scoped path are not separate truth systems. Each
slice maps exact requirements, authoritative artifacts, state/API/resource
claims, dependencies, exclusions, and completion evidence. A global finding
blocks or reopens a slice only when an explicit impact record intersects that
map. A change that preserves the completed contract may require named
revalidation without invalidating completion; a disjoint finding cannot trigger
an undifferentiated re-review.

The WP-300 candidate must close the exact GATE-1, GATE-2, GATE-4, and GATE-6
contract findings it consumes. Its deterministic resource declarations are
pre-code contract evidence; it claims no broad GATE-5 workload result.
Applicable workload identities and measurements remain broad WP-300
completion evidence. All aggregate gates remain open for their broader domains
and must close before final integration and release.

## Admission and completion

Every slice is Category B or C according to its actual candidate impact and
requires its own exact paths, contract fixtures, impact analysis, independent
review, and ADR-0013 admission. All six slices are now
`complete`/`approved`. The route-reservation claim binds exact implementation
`b47899150aa957b1dea8d844aa49852e3e6aa356`; its verified integration released
the independently reviewed WP-400 lifecycle. Pull request #23 then integrated
the exact five-file WP-400 admission checkpoint as tree-equivalent merge
`63015a6e4528a0f9e7d1b677ea987aa2ba1c8781`, and default-branch validation run
`31472025404` passed. Exact implementation
`a993555f3cbd2bc7026423f34ed5620f3a2e058f` proves that canonical compiler
metadata reaches the first legal Servient route entry and closes the static,
host, response, deactivation, and cleanup cells. Each status record grants no
successor source-edit authority. Specifically, each status record
grants no source-edit authority. The aggregate fixture roots remain absent
until their own exact reviewed candidate and admission checkpoint.

For `WP-100-PROPERTY-READ-HANDLER-SLICE`, the candidate and completion record
must claim only `ReadPropertyHandler` and its composition with the four reused
values named above. The property-read slice cannot claim the final
`InteractionInput` schema, `AcceptHint` resource admission,
`AffordanceTarget` relocation/no-atomic evidence, async/step handler traits,
host registration, sparse storage, or dispatch ownership.

All six slice completion records now pass, so the integration gate is `ready`.
It becomes `passed` only when the planned completion check is
registered and executable, both runtime cells pass, the compile-only cell
passes, all mandatory assertions are represented, and an independent
cross-package review attests the exact fixture revision. Broad entry points may
then change from `blocked` to `approved`; package completion still requires all
of each package's original evidence.
