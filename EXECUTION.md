# Current Execution Contract

Status: PLANNED

Contract revision: 1

Planning base: `0347a418ffc47299df14d6c613f1ab0f80aa63c8`

Task branch: `agent/zenoh-property-read-feedback-probe`

Pull request: #30 (draft)

## Purpose

This file carries one replace-in-place engineering contract. Git history, not
this file, retains completed contracts. The Lead-owned sections are frozen for
execution after the required independent plan challenge is resolved.

## Roles for This Contract

- Technical Lead: Max
- Executor: High
- Acceptance Reviewer: fresh Max context, separate from implementation
- Plan Challenger: ChatGPT
- Periodic Auditor: Ultra only at its repository-level checkpoints
- Project Owner: goals, constraints, counterexamples, external commitments

## Engineering Claim

Determine whether an external binding author can exercise the unchanged public
target Planning, Binding, and Servient Property Read path through real loopback
Zenoh I/O, and produce one bounded external-validity disposition.

A successful disposition carries compiler-produced immutable route data into
the first legal preparation entry, completes a real query/reply round trip,
and closes the required readiness, correlation, failure, cancellation/drain,
cleanup, and multi-Thing/route/form evidence. If the first required public
boundary is not constructible, the claim stops with a minimal reproduction and
an exact affected-authority finding; it does not repair or bypass the SPI.

This is non-authoritative architecture feedback. It grants no WP-600 progress,
does not pass `PROPERTY-READ-ARCHITECTURE`, and cannot establish general
protocol neutrality.

## Authoritative Inputs

- `PLAN.md` — Roadmap Frontier and Critical Path items 1-2.
- `ARCHITECTURE_GOVERNANCE.md` — Frozen Direction for v1.
- `workspace/0056-target-spi-external-validation-and-cross-protocol-neutrality.md`
  — migrated probe decision, constraints, and falsifiers.
- `docs/spec/binding-spi.md` — complete registration, route lifecycle, and
  real-target required evidence.
- `docs/work-packages/WP-300-bindings.md` and
  `docs/work-packages/WP-600-protocol-bindings.md` — probe boundary and
  non-product disposition.
- `docs/work-packages/PROPERTY-READ-ARCHITECTURE.md` and its gate manifest —
  successor ordering and non-substitution rules.
- `core/src/binding{,_compiler}.rs`, `planning/src/property_read.rs`, and
  `servient/src/{builder,handle,property_read,servient}.rs` — implementation
  truth for the public target path.
- Existing WP-300/WP-400 external contract fixtures and Zenoh runtime smoke
  tests — reusable evidence patterns, not substitute outputs.

## Scope

Risk class: architecture-sensitive empirical probe. No product-source or
public-API change is admitted by this contract.

### In scope

- A standalone, locked, non-workspace probe under
  `tools/architecture-probes/zenoh-property-read/` using only public crate APIs.
- `tools/check-zenoh-property-read-feedback-probe.sh` for the exact probe,
  forbidden-edge inspection, and non-skipped network evidence.
- One non-normative result at
  `docs/audits/ZENOH-PROPERTY-READ-FEEDBACK-PROBE.md`, plus only the minimal
  `docs/artifacts.csv` and Property Read gate-manifest registration needed to
  make its review-pending disposition durable.
- `EXECUTION.md` and `PROJECT_STATE.md` lifecycle/handoff updates.

### Out of scope

- Changes to product source, product Cargo manifests, public APIs, accepted
  architecture, or completed tranche evidence.
- Any Core/Planning/Servient/Zenoh carrier correction discovered by the probe.
- Aggregate Property Read fixture roots, aggregate candidate/admission, broad
  WP-300/WP-400 work, WP-600 implementation, zenoh-pico, or a second protocol.
- Legacy `ServerBinding`, `BindingRequest`, form-selection, or `Dispatch`
  adapters; shared artifact side tables; TD/target recomputation at runtime.
- Protocol-neutrality, production-readiness, workload, or release claims.
- The pre-existing dirty `agent/property-read-architecture-candidate` checkout.

## Engineering Plan

1. Build the external authoring preflight at the unchanged planning base. Have
   a Zenoh compiler emit an immutable Producer-route artifact and canonical
   reservation, then attempt to carry that real output through the complete
   host registration and Servient-owned route preparation. Do not let the
   server reconstruct it from the TD, target string, reservation hash, or a
   compiler/server shared side channel.
2. If that carrier is reachable, implement probe-local host calls and route
   state around two explicitly connected loopback Zenoh sessions. Keep route
   declaration, query ownership, correlation, reply opportunity, and cleanup
   inside the public target lifecycle; no binding callback receives handler,
   registry, dispatch, or plan-set authority.
3. Exercise one registration/Servient with at least two distinct Thing ids,
   two route keys/key expressions, and multiple valid forms including a shape
   that prevents success through a hard-coded first fixture. Prove no request
   is accepted before publication and each request matches its real admitted
   route and handler.
4. Run a successful Property Read query/reply with a non-empty payload, a
   handler-failure/error-reply case, cancellation while readiness is pending,
   and serving drain followed by rejected/no new acceptance. Drive all engine
   work with explicit budgets and wakeups; make terminal Zenoh undeclaration
   and zero live route/query/reply/cleanup counts observable.
5. Record every required public declaration and helper, diagnostic quality,
   cleanup-library mapping, repeated workaround, private/unsafe dependency,
   generic/monomorphization pressure, and measured probe binary/layout cost.
   Classify each concrete finding against its exact authoritative owner; do
   not convert subjective field count into a defect.
6. Add source/dependency checks proving the probe neither modifies nor calls
   legacy/product bypasses and never fixture-constructs a logical plan,
   artifact reference, route key, prepare input, permit, accepted request,
   response opportunity, or cleanup owner that production must supply.
7. Run the task check, locked probe tests with the real network case confirmed
   as executed, scoped formatting/lints, the default-branch validation matrix,
   and diff hygiene. Record exact commands and results for fresh Max review.

## Plan Challenge

Disposition: REQUIRED — PENDING INDEPENDENT CHALLENGE

Challenge the claim boundary, the strict multi-Thing/route/form interpretation,
the reliability of isolated loopback evidence, the no-substitution rules, and
whether any acceptance criterion accidentally authorizes a shared SPI repair.
High must not change status to `EXECUTING` until Max records the challenge
findings and disposition here, incrementing the contract revision if the
frozen contract changes.

## Acceptance Criteria

A `REVIEW_READY` handoff requires all of the following; a blocker is not a
partial pass:

1. The diff stays inside the in-scope paths and changes no product source,
   product manifest, normative contract semantics, completed evidence, or
   roadmap status.
2. The standalone public-author probe has its own lockfile and compiles without
   workspace membership, private APIs, unsafe code, legacy execution, or a
   fixture-owned substitute for any production carrier.
3. Compiler-produced Zenoh route data and reservation reach the exact route
   preparation that declares the matching live endpoint; identity and
   generation checks remain intact from plan through response and cleanup.
4. A non-skipped, explicit loopback network test proves two routed Thing
   shapes, correct handler selection, exactly one success reply with expected
   payload/correlation, one protocol-visible failure, and no pre-publication or
   cross-route acceptance.
5. Pending-readiness cancellation and post-publication drain both terminate;
   after explicit cleanup there are zero live routes, accepted requests,
   correlations, response opportunities, Zenoh queryables/queries, and cleanup
   obligations, and post-drain traffic is not accepted.
6. The audit answers every observation category in step 5, gives each concrete
   finding an owner and disposition, and explicitly limits the result to
   Zenoh-family external feedback. The aggregate gate remains `ready`, never
   `passed`; no broad or WP-600 progress is claimed.
7. `tools/check-zenoh-property-read-feedback-probe.sh`, the required mainline
   matrix, scoped format/lint checks, and `git diff --check` pass at the exact
   review head. The task check fails if the network test is skipped.

## Escalation and Stop Conditions

- Stop at the first public-boundary contradiction, including inability to
  obtain the admitted artifact payload from its reference at route prepare,
  inability to represent the required multi-Thing/route shape, or loss of an
  owned guard/request/reply/cleanup object. Preserve a minimal reproduction,
  name the exact authority, set status `BLOCKED`, and do not run later claims.
- Stop rather than introduce a product/API change, shared mutable artifact
  side channel, TD or target reparse, reservation reversal, private/unsafe
  access, legacy adapter, hidden dispatch, unbounded queue/task, blocking
  engine poll, or cleanup-by-drop workaround.
- Stop and report an environmental blocker if isolated real loopback I/O
  cannot run reproducibly; a skipped or mock-only test cannot satisfy the
  claim.
- Any required widening beyond the listed paths or evidence changes is a Lead
  revision and a new challenge boundary, not an Executor choice.
- After `REVIEW_READY`, stop for fresh Max acceptance. After accepted merge and
  reconciliation, stop before any SPI correction or aggregate-gate claim.

## Executor Handoff

Not started. Execution is forbidden while the Plan Challenge is pending.

## Acceptance Review

Verdict: NOT REVIEWED

The fresh Max reviewer must reconstruct the exact result and distinguish a
successful external-validity disposition from a correctly reported blocker.
