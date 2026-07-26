# Property-Read Architecture Gate

Status: BLOCKED

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
`WP-100 -> WP-200 -> WP-300 -> WP-400`. ADR-0013 permits only the four exact
slice tranches in the manifest to cross incomplete package boundaries after
their own admission reviews. No manifest record is implementation admission.

## Tranche DAG and entry points

```text
WP-100-PROPERTY-READ-HANDLER-SLICE
    -> WP-200-PROPERTY-READ-PLAN-SLICE
    -> WP-300-PROPERTY-READ-BINDING-SLICE
    -> WP-400-PROPERTY-READ-SERVIENT-SLICE
    -> PROPERTY-READ-ARCHITECTURE
```

The gate blocks:

- broad `WP-100-HANDLER-ENTRY`, while leaving its current exact prerequisite
  tranches and the property-read handler slice independently reviewable;
- `WP-300-BROAD-ENTRY`, except its named property-read binding slice; and
- `WP-400-BROAD-ENTRY`, except its named property-read Servient slice.

It does not block M1 documentation convergence, corrective work, the current
`WP-100-HANDLER-CONTEXT` candidate lifecycle, WP-200 planning work, or admission
preparation for the four exact slices. WP-500 and WP-600 remain downstream of
WP-300 and are therefore indirectly protected from becoming the first
architecture proof.

## Exact scenario

Both runtime cells execute the same observable scenario:

```text
TD fixture
  -> shared planner
  -> immutable property-read logical plan
  -> mock binding compiler
  -> binding-owned artifact
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
the portable poll surface remains authoritative and the gate does not select an
executor.

## Fixture topology

The future fixture has two package roots:

- `property-read-binding` depends only on the TD/Core/Planning interfaces
  required to implement a binding compiler and producer server role. It cannot
  depend on Servient or application handler modules.
- `property-read-runner` composes the TD, planner, mock registration, Servient,
  and handler, and owns the runtime assertions.

Only protocol-frame values, deterministic I/O state, and instrumentation probes
are fixture adapters. A fixture may not replace the logical plan, binding
artifact, route guard or activation permit, accepted request, handler
context/input/output, response opportunity, or cleanup owner. If any one of
those production boundaries is unavailable or unconstructible, the owning
slice remains blocked and the difficulty becomes design feedback.

The fixture directories are not created until their owning tranche has an exact
reviewed candidate. Placeholder crates would falsely imply a constructible
boundary.

## Mandatory runtime evidence

The completion check must prove:

- the TD fixture is read during planning and not at runtime;
- the logical plan is immutable after admission, and the binding artifact is
  sufficient without the TD;
- no acceptance occurs before Servient publication;
- only Servient selects the admitted route and handler;
- the static handler is invoked exactly once with protocol-neutral input;
- its protocol-neutral output consumes one response opportunity and reaches
  the mock binding exactly once;
- Thing, plan, binding, route, request/correlation, and generation identities
  remain consistent;
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
- the mock cannot rescan the TD or construct a runtime logical plan; and
- the `async-no-std` portable surface compiles without choosing an executor.

Runtime tests own state-transition and cleanup claims. Compile-fail tests own
capability absence and construction boundaries. Dependency/source inspection
owns crate direction and the absence of hidden dispatch imports.

## Admission and completion

Every slice is Category B or C according to its actual candidate impact and
requires its own exact paths, contract fixtures, impact analysis, independent
review, and ADR-0013 admission. The manifest intentionally records all four as
`planned` and `blocked`; it grants no source-edit authority.

The integration gate becomes `ready` only after all four slice completion
records pass. It becomes `passed` only when the planned completion check is
registered and executable, both runtime cells pass, the compile-only cell
passes, all mandatory assertions are represented, and an independent
cross-package review attests the exact fixture revision. Broad entry points may
then change from `blocked` to `approved`; package completion still requires all
of each package's original evidence.
