# Architecture Review Findings — 2026-07-19

Status: review input; not authoritative design
Target revision: v4.9 architecture closure
Purpose: preserve external review findings discovered while the primary architecture-design session was interrupted by usage limits.

## How to use this document

This file is a review inbox, not a design decision record and not an execution plan.

Every finding below MUST be triaged against the current repository state, including:

- the v4.9 architecture backbone;
- authoritative domain specifications;
- existing ADRs and amendments;
- `PLAN.md` and current closure gates;
- machine-readable governance artifacts;
- current runtime code where implementation evidence matters.

Do not modify architecture merely because a finding is listed here. For each finding, choose exactly one disposition:

- **Accept** — the finding identifies a real architecture/specification defect. Update the authoritative design/specification and link the change here.
- **Plan** — the issue is valid but belongs to implementation, migration, validation, or a later work package. Add a traceable work item to the appropriate plan and link it here.
- **Reject** — the current design already addresses the issue, or the suggested change would violate a required invariant. Record the reason and the evidence.
- **Defer** — the issue is valid but is intentionally outside the current architecture-closure scope. Record the future phase or trigger.

The primary architecture session retains ownership of architecture continuity and final disposition.

---

## AR-001 — v4.8/v4.9 authority ambiguity

**Status:** Needs Validation

### Observation

The repository is in a v4.9 architecture-closure phase, while legacy or transitional documents may still describe themselves using v4.8-era authority language or contain runtime/API descriptions that predate the current architecture backbone.

### Risk

A human or AI contributor may combine ownership rules from v4.9 with API/runtime contracts from an older revision and accidentally construct a hybrid architecture that was never approved.

### Validation questions

1. Is there exactly one documented authority chain for architecture, domain specs, ADRs, machine-readable artifacts, and legacy `docs/design.md` content?
2. Does every document that still contains superseded v4.8 material clearly identify its status?
3. Can an implementation agent determine which document wins when two artifacts conflict?
4. Are closure gates capable of detecting stale authority declarations?

### Candidate disposition

Prefer resolving this through document authority/governance rather than copying all architecture content into one monolithic document.

### Disposition

Pending.

---

## AR-002 — Protocol Binding SPI: conceptual boundary vs exact Rust contract

**Status:** Needs Validation

### Observation

The architecture appears to have converged on the responsibilities and composition of a Protocol Binding registration: capability declaration, planning/compiler participation, client/server execution, resource declarations, and lifecycle constraints. It is less clear whether the exact Rust-level SPI surface has been fully frozen.

### Risk

Implementation may begin from a correct conceptual model but diverge at the trait/type boundary, especially around:

- registration identity and generation;
- compiler extension ownership;
- logical plan to binding artifact compilation;
- client call execution;
- server route preparation and acceptance;
- subscription drivers;
- emission paths;
- resource and cleanup contracts.

### Validation questions

1. Is there one authoritative exact SPI specification suitable for implementing a second binding without consulting Zenoh internals?
2. Are all cross-boundary types protocol-neutral where required?
3. Can a PB execute without reading TD structure or consulting Servient registries?
4. Is the compiler/runtime version or generation relationship explicit?
5. Are server and client responsibilities symmetric where they should be, and intentionally asymmetric where required?

### Candidate disposition

If the conceptual design is closed but the exact Rust contract is not, keep the architecture unchanged and close the gap in the binding SPI specification before runtime migration.

### Disposition

Pending.

---

## AR-003 — Server Binding lifecycle may overexpose internal state-machine complexity

**Status:** Needs Validation

### Observation

The producer/server lifecycle requires strict preparation, readiness, commit/publication, admission, acceptance, shutdown, and cleanup semantics to prevent partial exposure and ownership races. There is a risk that the theoretical state machine may be mapped too literally onto the public PB authoring surface.

### Risk

If every internal proof state becomes a separate PB-facing guard/type/API step, binding implementations may become unnecessarily difficult while gaining no additional correctness beyond what a Servient-owned internal state machine could enforce.

### Validation questions

1. Which lifecycle states must be visible to PB implementations, and which can remain internal to Servient?
2. Can the required invariants be preserved with a smaller SPI surface such as prepare/readiness/commit-or-activate/accept/shutdown?
3. Does simplifying the surface reintroduce any race, partial publication, permit bypass, or cleanup ambiguity?
4. Are protocol-specific servers such as Zenoh, HTTP, CoAP, and no_std poll loops all expressible without adapter contortions?

### Important constraint

Do **not** simplify the SPI merely for ergonomics. Reject this finding if the extra exposed states are required to preserve a proven invariant.

### Disposition

Pending.

---

## AR-004 — Candidate/form selection policy may be less explicit than ownership

**Status:** Needs Validation

### Observation

The architecture appears to assign form eligibility, candidate construction, and plan compilation to planning, while preventing PBs from rescanning TD or independently selecting forms. The remaining question is whether candidate ordering and runtime fallback policy are fully specified.

### Risk

Multiple valid forms or protocols may compile successfully, but execution behavior can still be ambiguous if ranking policy is not explicit. Examples include HTTP vs HTTPS, Zenoh vs MQTT, TD form order, application preferences, security policy, binding availability, and runtime health/fallback.

### Validation questions

1. What determines candidate ordering?
2. Is TD form order normative, advisory, or ignored after policy evaluation?
3. Can application policy override default protocol/security preference?
4. Which decisions are compile-time eligibility decisions and which are runtime fallback decisions?
5. Can runtime health affect selection without allowing PBs to re-plan?
6. Is fallback deterministic and observable?

### Candidate disposition

If ownership is already correct, avoid reopening PB responsibilities. Clarify planning/Servient policy only where behavior remains underspecified.

### Disposition

Pending.

---

## AR-005 — Subscription abstraction must preserve native protocol coalescing

**Status:** Needs Validation

### Observation

A protocol-neutral subscription API must work across Zenoh wildcard subscriptions, MQTT multi-topic subscriptions, WebSocket streams, SSE, CoAP Observe, and other models. Earlier merge-based designs risked implementing "subscribe all" as N independent subscriptions plus a core merge layer even when the protocol can natively coalesce them.

### Risk

An abstraction that owns universal queues, universal merging, or a fixed backpressure strategy can erase protocol capabilities and add avoidable allocation, scheduling, and fairness overhead.

### Validation questions

1. Can one Thing-level subscription plan compile to one native/coalesced PB driver?
2. Is event/property identity recovered through compiled routing metadata rather than N core subscriptions where possible?
3. Does the protocol-neutral driver surface remain minimal, e.g. poll/next, stop/cancel, terminal status, and bounded metadata?
4. Is queue ownership located intentionally rather than implicitly in core?
5. Can protocols that cannot coalesce still implement equivalent semantics without forcing all protocols into the least-capable model?

### Candidate disposition

Preserve protocol-neutral semantics while allowing PB-native subscription topology and flow control.

### Disposition

Pending.

---

## AR-006 — Resource-accounting model may leak excessive complexity into public APIs

**Status:** Needs Validation

### Observation

The architecture places strong emphasis on bounded resources, admission, cleanup ownership, queue/cache/artifact limits, generations, and work budgeting. This is valuable for no_std and constrained systems, but the model may become difficult for normal host applications if internal accounting concepts leak into common APIs.

### Risk

A simple operation such as reading a property could require users or ordinary binding authors to understand resource reservations, work budgets, generation leases, cleanup slots, or other internal proof machinery that should normally be hidden.

### Validation questions

1. Which resource concepts are internal invariants versus public configuration?
2. Can host/std users operate with safe bounded defaults?
3. Can constrained/no_std profiles opt into explicit capacities without changing semantic APIs?
4. Are failure modes for exhausted resources explicit and deterministic?
5. Does hiding defaults weaken any no-allocation or bounded-memory guarantee?

### Candidate disposition

Consider profile-based exposure (host defaults vs constrained explicit capacity) while keeping one semantic execution model.

### Disposition

Pending.

---

## AR-007 — Canonical execution contract may need stronger prominence

**Status:** Needs Validation

### Observation

The architecture contains the essential data-flow model, but contributors may still benefit from one canonical mental model that distinguishes planning, execution, and lifecycle planes.

Suggested conceptual view:

```text
Planning Plane

TD
 -> Logical Plan
 -> Binding Candidate
 -> Binding Artifact
 -> Immutable Compiled Plan Set

Execution Plane (consumer)

Application
 -> Servient orchestration
 -> selected compiled plan
 -> Protocol Binding
 -> transport

Execution Plane (producer)

transport
 -> Protocol Binding
 -> protocol-neutral inbound item
 -> Servient orchestration
 -> handler
 -> Servient
 -> protocol-neutral response/emission
 -> Protocol Binding
 -> transport

Lifecycle Plane

Servient-owned plan generations, route publication, admission,
subscription/emission lifecycle, draining, cleanup, and destruction.
```

### Risk

Without a prominently declared canonical execution contract, future contributors may reintroduce shortcuts such as PB-owned handler dispatch, TD rescanning on the hot path, or hidden direct execution paths.

### Validation questions

1. Is this model already canonical and sufficiently prominent?
2. Can every cross-module API be assigned to one of these planes?
3. Do architecture tests or ownership artifacts catch paths that bypass Servient orchestration?

### Candidate disposition

This may require only documentation/index improvements rather than new architecture.

### Disposition

Pending.

---

## AR-008 — Review findings must not silently become work items or design decisions

**Status:** Needs Validation

### Observation

Architecture review, architecture authority, and execution planning have different roles. External findings can be wrong, redundant, already solved, or intentionally deferred.

### Risk

Adding untriaged findings directly to `PLAN.md` can turn speculative review comments into mandatory implementation work and disturb the architecture-closure sequence.

### Validation questions

1. Is there an explicit triage path from review finding to Accept/Plan/Reject/Defer?
2. Are accepted design changes linked to authoritative documents or ADRs?
3. Are planned implementation items traceable back to the finding?
4. Are rejected/deferred findings retained with rationale so future agents do not repeatedly reopen them?

### Candidate disposition

Keep this review file non-authoritative and perform triage before modifying plans or specs.

### Disposition

Pending.

---

## Triage completion checklist

Before considering this review closed:

- [ ] Every AR item has exactly one disposition.
- [ ] Every **Accept** item links to the authoritative changed artifact(s).
- [ ] Every **Plan** item links to a concrete plan/work-package entry.
- [ ] Every **Reject** item records the invariant/evidence supporting rejection.
- [ ] Every **Defer** item records the intended future phase or trigger.
- [ ] No review finding remains as an implicit requirement outside authoritative artifacts.
- [ ] After triage, the primary architecture-closure workflow resumes from its original interruption point.
