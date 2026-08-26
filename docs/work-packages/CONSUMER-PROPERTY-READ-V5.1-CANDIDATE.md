# Consumer Property Read v5.1 Candidate Package Projection

Status: docs-only activation-candidate projection; not implementation admission.

Current active revision remains v5.0. This file records the exact package slices
that become eligible for ADR-0013 admission only after the independently
reviewed v5.1 authority candidate is activated. It deliberately does not edit
the active v5.0 WP-100/WP-200/WP-300/WP-400 package documents before that
checkpoint.

The projection is derived from workspace/0061 and ADR-0019. It introduces no
new requirement identity and no source work.

## WP-100 Consumer call values and response validator

Candidate requirements:

- `API-OPTIONS-001` plus already-active `API-PAYLOAD-001`, `BIND-IO-001`,
  `BIND-DELIVERY-001`, `BIND-CALL-CANCEL-001`, `BIND-STORAGE-001`, and
  `BIND-MEM-001` as applicable to their existing owners.

Minimum scope:

- migrate the Consumer Property Read target call boundary to the narrowed owned
  `InteractionOptions` selection/control contract;
- operation payload is a separate operation input and is not read from the
  target-path `InteractionOptions`;
- provide the Core-owned narrow binding-origin Property Read response validator
  defined by `docs/spec/interaction-core.md`;
- preserve host/static semantic parity for response identity and terminal
  classification without opening subscription progress.

Explicitly excluded:

- write/action migration merely to remove legacy `InteractionOptions::data`;
- advanced options, broad defaults merging, media/subprotocol/security branch
  selectors, validation profiles;
- codec/schema compiler work;
- subscriptions/emissions.

Pre-code evidence must prove the narrowed options/value surface is constructible
in the required Core feature cells and that untrusted response metadata cannot
bypass the shared validator.

## WP-200 Consumer Property Read planning and selection

Candidate requirements:

- `PLAN-REQUEST-001` plus active `PLAN-SET-001`, `PLAN-ARTIFACT-001`,
  `PLAN-BOUND-001`, `PLAN-COST-001`, and `PLAN-COST-003`.

Minimum scope:

- make the existing bounded Property Read compiler path constructible for
  `BindingArtifactRole::ConsumerCall` through a reviewed public Planning entry;
- build/freeze/publish one consumed Property Read plan using an eager admitted
  Consumer artifact;
- select only inside that immutable plan set using the narrowed options kernel;
- preserve the plan/binding/artifact generations required to build one
  `OutboundRequest`;
- prove the TD and compiler build inputs can be dropped before the call path.

Explicitly excluded:

- `PLAN-INDEX-001`, `PLAN-LAZY-001`, `PLAN-CACHE-001`, `PLAN-COST-002`;
- automatic candidate fallback;
- additional-response breadth;
- multi-binding fairness or performance closure.

The target-path negative fixture must poison call-time TD/Form scanning and raw
binding support probing.

## WP-300 selected OutboundRequest and ClientBinding call

Candidate requirements:

- `BIND-OUT-001` plus active `BIND-REG-001`, `BIND-STORAGE-001`,
  `BIND-MEM-001`, `BIND-DELIVERY-001`, `BIND-IO-001`,
  `BIND-CALL-CANCEL-001`, and `BIND-HOST-CANCEL-001`.

Minimum scope:

- add one Consumer Property Read client capability to a complete registration;
- construct one `OutboundRequest` only after selection/security commit;
- invoke only the selected client execution component, with no `Thing`, raw
  `Form`, mutable `InteractionOptions`, `supports_with_thing`, or reselection
  authority in the binding input;
- Host execution returns one owned cancellation-aware call before protocol side
  effects;
- constrained execution uses one admitted generation-bearing request slot with
  the same accepted/rejected/terminal semantics;
- pre-acceptance rejection returns the exact request; caller drop, cancellation,
  timeout, late result, and cleanup preserve one owner;
- binding-origin success remains untrusted until Core validation.

Explicitly excluded:

- `BIND-PROGRESS-001`;
- subscription driver/start APIs as active implementation work;
- broad retry/fallback;
- concrete Zenoh production implementation.

Legacy `BindingRequest` and raw-form selection may remain only for legitimate
unmigrated capabilities. The Consumer Property Read target path must have zero
edges to them.

## WP-400 consumed plan-set and call ownership

Candidate requirements:

- active plan-set/lifecycle/cancellation/drop/resource contracts plus the three
  v5.1 Consumer entry identities consumed from their owning packages.

Minimum scope:

- `consume` publishes one immutable consumed Property Read plan generation
  before returning the handle;
- the target `read_property` path obtains a generation-bearing plan lease,
  applies the narrowed options, constructs/admits one call owner or static slot,
  invokes the selected binding, validates the binding-origin response, and
  releases call and plan ownership exactly once;
- dropping caller interest never drops the accepted binding call as the cleanup
  protocol;
- drain closes new call admission while already admitted calls retain the plan
  generation through terminal settlement;
- Host and constrained/static traces preserve the same semantic terminal
  outcomes.

Explicitly excluded:

- broad ConsumedThing facade migration;
- subscriptions/ObserveProperty;
- collection operations;
- scheduler-wide fairness beyond the separately owned WP-400 early checkpoint;
- production Zenoh.

## Cross-package architecture gate after the four slices

After v5.1 activation and independent ADR-0013 admission/completion of the exact
slices above, the repository may create and register
`CONSUMER-PROPERTY-READ-ARCHITECTURE`.

The gate must compose real outputs through:

```text
consumed plan publication
 -> immutable Consumer selection
 -> owned OutboundRequest
 -> selected ClientBinding call/static slot
 -> untrusted binding-origin result
 -> shared Core validation
 -> InteractionOutput
 -> terminal call/plan cleanup
```

It must prove:

- no runtime TD/Form rescan;
- no `supports_with_thing` or legacy `BindingRequest` edge;
- exact plan/binding/generation identity;
- caller drop, cancellation, timeout and late-result ownership;
- response-validator non-bypass;
- host/static semantic parity; and
- zero retained call, cleanup obligation, or plan lease at terminal completion.

The gate deliberately excludes fallback, lazy/cache/index, subscriptions,
collections, write/action breadth, production Zenoh, and broad performance
closure.

## Post-gate production feedback

Only after the architecture gate passes should WP-600 claim the first real Host
Zenoh Consumer Property Read production slice. The existing Producer Property
Read remains regression evidence and then participates in the bidirectional
closed loop. zenoh-pico runtime evidence remains required before any matching
constrained capability/parity completion claim.

## Activation migration

The v5.1 activation checkpoint must migrate these package-slice statements into
the active WP-100/WP-200/WP-300/WP-400 documents and update their revision
projections without changing the independently reviewed semantic boundary.
This candidate file may then become historical migration evidence or be removed
in that same activation change.
