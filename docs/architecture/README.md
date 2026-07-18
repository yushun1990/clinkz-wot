# clinkz-wot Architecture Backbone

Status: v4.9 architecture-closure candidate.

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

1. `docs/design.md` selects the active revision and indexes normative sources.
2. This architecture backbone owns cross-module invariants and primary flows.
3. Registered files under `docs/spec/` own detailed behavior and public API
   contracts for one domain each.
4. API ownership, state-machine, resource, requirement, and performance
   artifacts own their exact machine-readable projections.
5. Accepted ADRs record decisions and rejected alternatives; their decision
   must be integrated into the sources above in the same revision.
6. Work packages own migration order and evidence, not behavior.

`PLAN.md`, reviews, audits, and thinking notes are non-normative. A conflict
between normative sources is a gate failure, not a precedence shortcut.

The existing v4.8 `docs/design.md` and amendments remain migration inputs until
their valid content is moved into single-owner v4.9 specifications. Runtime
implementation remains paused during that migration.
