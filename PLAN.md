# clinkz-wot Execution Plan

## Purpose

This file defines the stable execution direction for the active refactor.

It provides:

- the active target;
- durable execution and admission rules;
- high-level milestone ordering;
- navigation to authoritative planning artifacts.

It does not record live session state.

The following belong in `PROJECT_STATE.md` instead:

- the currently active work item;
- temporary blockers and uncertainties;
- recent progress;
- exact stopping points;
- unverified changes;
- next-session actions;
- the agent's current compact project understanding.

This file does not define architecture, public APIs, state transitions,
resource limits, or implementation behavior.

## Active Target

The active target is the v4.9 architecture-closure revision.

The v4.8 detailed-design candidate is migration input only. Valid contracts
must be reconciled into the v4.9 modular specification set before they are
treated as active requirements.

The architecture-closure effort exists to produce one coherent, reviewable,
executable target across architecture, APIs, state, resources, performance,
requirements, work packages, implementation, and evidence.

## Authority and Navigation

Read authoritative material through its registered indexes rather than treating
this plan as a specification.

Primary navigation:

1. `docs/architecture/README.md` for the architecture backbone;
2. accepted decisions under `docs/ADRs/`;
3. registered domain specifications and reconciled detailed requirements;
4. `docs/artifacts.csv` for API, state, resource, performance, requirement, and
   evidence artifacts;
5. `docs/work-packages/index.toml` for authoritative work-package identities,
   dependencies, admission, completion contracts, removals, and evidence keys;
6. implementation, tests, audits, and evidence for conformance.

Historical plans and completed checkpoints belong under `docs/deprecated/` and
`docs/evidence/` as appropriate.

## Execution Rules

Runtime and public-API changes may proceed only through the scoped admission
policy defined by the accepted governance artifacts, including ADR-0013 and
`REFACTOR-GATE-001`.

A scoped admission does not close or waive global convergence gates.

Documentation, governance, checkers, fixtures, reviews, evidence preparation,
and work-package definition may proceed without runtime admission when their
own rules permit it.

Do not infer admission, completion, or readiness from prose in this file.
Consult the authoritative work-package index, package documents, audits,
reviews, and exact evidence.

## Architecture-Closure Completion

The v4.9 architecture-closure milestone is complete only when:

1. the architecture backbone freezes primary flows, module boundaries,
   compiled-plan lifecycle, Servient orchestration, and Protocol Binding
   integration and deployment;
2. every accepted ADR is reflected in one non-conflicting authoritative domain
   specification;
3. the v4.8 monolith and temporary amendments are decomposed into registered
   single-owner specifications;
4. the API matrix, exact state models, resource schema, performance manifests,
   requirement index, and work-package DAG identify the same v4.9 target;
5. executable checks pass; and
6. independent same-revision review closes every affected gate.

## Frozen Direction for v1

The active architecture direction includes:

- an explicit compiled-plan-set lifecycle and binding-artifact boundary;
- Cargo-linked, application-registered Protocol Binding crates;
- startup-only binding composition for one Servient instance;
- engine-orchestrated, route-scoped binding progress with no hidden direct
  handler-dispatch path;
- atomic serving publication through one Servient-owned activation authority
  and nonretained route-scoped accept permits;
- a modular normative-document hierarchy that keeps the architecture visible.

The authoritative definitions and exact contracts live in the registered
architecture, ADR, API, state, SPI, resource, performance, and requirement
artifacts.

## High-Level Work Order

Subject to the authoritative machine-readable DAG and scoped admission rules,
the intended convergence order is:

1. foundation resource, work, time, generation, and accounting refresh;
2. core handler, security, codec, and lock-isolation contracts;
3. immutable logical and binding plans, capability indexes, and compiler
   migration;
4. client and server binding SPI, routes, subscriptions, responses, and
   emissions;
5. Servient lifecycle, cleanup, application facades, and scheduling policy;
6. Discovery client cleanup;
7. Zenoh and zenoh-pico binding migration;
8. umbrella composition, obsolete API removal, and final conformance evidence.

This list communicates durable direction only. It is not a substitute for
package-level dependencies, admission state, or current execution context.

## Project Continuation

The current project phase, active work, blockers, progress, stopping point, and
next safe actions are maintained by AI agents in `PROJECT_STATE.md`.

Agents must update that file continuously according to `AGENTS.md`, so a new
conversation can resume from the repository without reconstructing the entire
project.
