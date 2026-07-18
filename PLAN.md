# clinkz-wot Documentation and Refactor Plan

## Active revision

The active target is the v4.9 architecture-closure revision. Runtime migration
is paused. The v4.8 detailed-design candidate did not pass Architecture Review
02 and remains only a migration input until its valid contracts are moved into
the v4.9 modular specification set.

Read active material in this order:

1. `docs/architecture/README.md` and the architecture backbone it indexes;
2. accepted decisions under `docs/ADR/`;
3. `docs/design.md` and registered domain specifications for detailed
   requirements that have already been reconciled with the backbone;
4. the API, state, resource, performance, and requirement artifacts registered
   by `docs/artifacts.csv`; and
5. `docs/work-packages/index.toml` for implementation order.

`PLAN.md` is navigation and status only. It does not define architecture,
public APIs, state transitions, limits, or implementation behavior.

## Current admission status

- GATE-1 API ownership: open.
- GATE-2 lifecycle and cleanup: open.
- GATE-3 Directory client boundary: closed by reviewed carry-forward evidence.
- GATE-4 resource limits: open.
- GATE-5 executable performance contracts: open.
- GATE-6 implementation work-package DAG: open.

No runtime or public-API refactor may resume while a required gate is open.
Documentation restructuring, ADRs, review evidence, checkers, fixtures, and
work-package corrections may proceed.

## Current design milestone

The architecture-closure milestone is complete only when:

1. the architecture backbone freezes primary flows, module boundaries,
   compiled-plan lifecycle, Servient orchestration, and Protocol Binding
   integration/deployment;
2. every accepted ADR is reflected in one non-conflicting domain specification;
3. the v4.8 monolith and temporary amendments are decomposed into registered
   single-owner specifications;
4. the API matrix, exact state models, resource schema, performance manifests,
   requirement index, and work-package DAG all identify the same v4.9 target;
5. executable checks pass; and
6. an independent same-revision review closes each affected gate.

The immediate decisions are:

- an explicit compiled-plan-set lifecycle and binding-artifact boundary;
- Cargo-linked, application-registered Protocol Binding crates for v1;
- startup-only binding composition for one Servient instance;
- engine-orchestrated, route-scoped binding progress with no hidden direct
  handler-dispatch path; and
- atomic serving publication through one Servient-owned activation authority
  and nonretained route-scoped accept permits; and
- a modular normative-document hierarchy that keeps architecture visible.

## Implementation order after gate closure

Implementation follows `IMPL-CONFORM-001` and the machine-readable DAG:

1. foundation resource, work, time, generation, and accounting refresh;
2. core handler, security, codec, and lock-isolation contracts;
3. immutable logical/binding plans, capability indexes, and compiler migration;
4. client/server binding SPI, routes, subscriptions, responses, and emissions;
5. Servient lifecycle, cleanup, application facades, and scheduling policy;
6. Discovery client cleanup;
7. Zenoh and zenoh-pico binding migration; and
8. umbrella composition, obsolete API removal, and final evidence.

The authoritative package dependencies, removals, and evidence keys live in
`docs/work-packages/index.toml` and its package documents. Historical plans and
completed checkpoints remain under `docs/deprecated/` and `docs/evidence/`.
