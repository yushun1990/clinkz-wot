# clinkz-wot Architecture Backbone

Status: active v5.0 authority.

This directory defines the concise architecture backbone for `clinkz-wot`.
It explains the engine from core invariants to edge integration. Detailed API
schemas, state transitions, resource limits, and workloads must project this
backbone without redefining it.

## Reading order

1. [System goals and context](00-system-goals-and-context.md)
2. [Primary data flows](10-primary-data-flows.md)
3. [Module boundaries](20-module-boundaries.md)
4. [Compiled-plan lifecycle](30-compiled-plan-lifecycle.md)
5. [Protocol Binding SPI and deployment](40-protocol-binding-spi-and-deployment.md)
6. [Servient runtime lifecycle](50-servient-runtime-lifecycle.md)

## Architectural invariants

The following rules apply to every profile and implementation strategy:

- W3C WoT TD 1.1 is the default compatibility target. TD 2.0 work is
  experimental and additive.
- TD/TM documents are lossless data contracts. They do not contain runtime
  state, transport behavior, or compiled caches.
- Interaction hot paths execute immutable admitted plans. They do not rescan a
  TD, redo defaulting, or let a binding select another form.
- Core owns protocol-neutral values and SPI semantics. It does not own a global
  emission scheduler, a universal subscription queue, or application handles.
- Servient owns application orchestration, plan-set lifetime, lifecycle
  transactions, scheduling policy, and cleanup ownership.
- A concrete Protocol Binding owns protocol syntax, I/O, correlation,
  protocol-local flow control, and binding-local state. It does not call
  application handlers directly or reinterpret the TD.
- Every operation that can outlive its caller has one generation-bearing owner,
  a bounded retained footprint, an explicit cancellation path, and a terminal
  cleanup disposition.
- All user, provider, codec, and binding callbacks execute outside engine locks
  and constrained critical sections.
- Every queue, cache, cursor set, type-erased object, and external-input buffer
  has an admitted count and byte bound. No profile treats zero as unbounded.
- `no_std + alloc` uses caller-driven progress and bounded storage while
  preserving the same protocol-neutral semantics as host builds.
- Protocol Binding code is composed through ordinary Rust crates and explicit
  registration. Runtime code loading is not a v1 feature.

## Normative hierarchy

The authority order is:

1. `ARCHITECTURE_GOVERNANCE.md` controls technical convergence and change.
2. Accepted ADRs control decisions and explicit supersession.
3. `docs/design.md`, this backbone, and the exact sources registered by
   `docs/spec/v5-authority-reset.toml` own the 62 active requirements.
4. API ownership, state, resource, requirement metadata, gate, performance,
   and evidence artifacts own their exact machine-readable projections.
5. Work packages own migration order, admission, and evidence, not behavior.

`PLAN.md`, task sessions, pull requests, reviews, and thinking notes are
non-normative. A conflict between normative sources is a gate
failure, not a precedence shortcut.

ADR-0018 supersedes residual decomposition and ADR-0014's D3 target DAG.
`docs/requirements.csv` retains metadata and historical source pointers but no
longer selects v5 authority. The transition manifest gives all 121 inherited
identities one disposition and registers the ten owners of the 62 active
definitions. An amendment is active only for an identity assigned to its exact
path; other mentions are refinement or evidence history.

This candidate grants no authority until independent review and separate
mainline integration. A bounded implementation tranche may proceed only through
ADR-0013 admission; the reset itself creates no source-edit permission.

The first executable composition proof is
`PROPERTY-READ-ARCHITECTURE`, defined by the registered work-package gate
manifest. It exercises one property read through real planner, binding,
Servient, handler, response, generation, and cleanup boundaries in host and
manual profiles, with an async/no-std compile projection. It blocks broad
handler, binding, and Servient entry but grants no implementation admission;
fixture code may adapt protocol frames and instrumentation, not replace an
ownership boundary.
