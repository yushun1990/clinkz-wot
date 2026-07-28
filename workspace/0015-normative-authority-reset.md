# 0015 Normative Authority Reset Versus Continued Design Decomposition

Status: MIGRATED

Kind: architecture-governance and implementation-throughput proposal

Target revision: current v4.9 authority model or a deliberately superseding revision

## Scope and authority

This topic asks whether the project should continue the current D3 requirement-by-requirement decomposition of `docs/design.md`, replace it with a normative-authority reset, or adopt a smaller hybrid transition.

It does not itself change current normative authority, deactivate any requirement, delete `docs/design.md`, supersede an ADR, admit implementation work, or predetermine a new design revision. The AI must decide the technical direction from current repository evidence and migrate any stable conclusion into the appropriate authoritative artifacts.

## Context

D3 resolved how to finish decomposing the former monolithic `docs/design.md`:

- current ownership is recorded by `docs/requirements.csv::source_path`;
- final ownership is recorded by `docs/spec/decomposition.csv::target_path`;
- every stable requirement must retain exactly one current owner;
- each target domain is activated through an atomic independently reviewed migration;
- `docs/design.md` remains the residual detailed owner until each requirement moves.

This approach preserves traceability and avoids duplicate, missing, or partially activated normative owners. It also turns document decomposition into a long-running authority migration involving specifications, amendments, registries, work packages, checkers, audits, reviews, and historical evidence.

The current Foundation authority candidate demonstrates the resulting cost. It moves a limited requirement set but requires a frozen base, an immutable candidate, a registered multi-path boundary, independent attestation, a separate integration checkpoint, and active-authority count changes. Many residual domains would still remain afterward.

At the same time, the first executable property-read architecture path is blocked at the WP-200 host-erased/static binding-artifact and compiler-extension boundary. The project therefore needs to determine whether preserving every residual v4.9 requirement through D3 remains the best use of architecture and review capacity.

## Owner concern

The Project Owner is concerned that the current decomposition process may have become disproportionate to the value it preserves.

The concern is not merely that D3 should receive lower priority. If every residual requirement must eventually complete the same migration protocol, postponement only defers the same cost. The deeper question is whether the project should continue treating all residual monolithic requirements as active v1 obligations that must be migrated losslessly.

A simpler alternative may be to archive the current monolithic document as historical design input, establish a small current normative manifest and domain-specification set, and explicitly re-adopt only requirements justified by the active architecture, implementation path, product goals, and verification evidence.

## Problem

The current authority model assumes that residual requirements remain valid and must retain continuous normative ownership until individually migrated. That assumption creates several possible risks:

1. documentation migration can become a dominant execution stream rather than support implementation;
2. historical review and checker compatibility can receive more effort than executable architecture validation;
3. speculative or premature requirements may be preserved merely because they already have identifiers;
4. the project may maintain a high apparent specification coverage while the target Planning, Binding SPI, Servient, and Zenoh execution path remains incomplete;
5. future domain migrations may repeatedly reopen amendments, evidence, and checker changes without materially reducing implementation risk;
6. a large active requirement set may make every later architecture correction expensive even when much of the affected scope has not been implemented.

Conversely, abandoning the current residual authority without a controlled replacement may lose valuable constraints, invalidate evidence silently, leave public APIs or lifecycle behavior underspecified, and recreate the earlier failure mode in which AI implementation outruns coherent architecture.

## Alternatives to investigate

### Alternative A: Continue D3 as currently designed

Retain all active residual requirements and complete the reviewed target-domain migration DAG.

The investigation must determine:

- the remaining number and domain distribution of residual requirements;
- the expected migration and review cost by domain;
- which migrations directly unblock implementation;
- whether the atomic migration protocol can be simplified without weakening single-owner authority;
- whether the current process has a bounded and credible completion path.

### Alternative B: Normative-authority reset

Deliberately supersede the current residual-authority model in a new design revision.

A possible shape, to evaluate rather than assume, is:

- archive the current monolithic `docs/design.md` as historical design input;
- replace it with a small active revision and normative-source manifest;
- define a new active requirement registry containing only explicitly re-adopted requirements;
- preserve old requirement ids and evidence in a historical registry without granting current authority;
- keep only the architecture and domain contracts needed for the current v1 execution path and known product constraints;
- reintroduce deferred requirements only when their domain enters implementation or a real constraint requires them;
- retire the D3 residual/final/amendment completion model and its migration-only checks.

The investigation must determine whether this can be done without losing indispensable safety, lifecycle, resource, portability, W3C compatibility, or protocol-neutrality constraints.

### Alternative C: Bounded core reset with selective carry-forward

Preserve a reviewed core subset and retire the remainder from active v1 authority.

A possible shape is:

- retain accepted cross-module architecture invariants and currently implemented Foundation/Core contracts;
- retain Planning, Binding SPI, Servient, and Property Read architecture requirements needed by the active vertical slice;
- move unrelated or speculative residual domains to historical/deferred status;
- stop requiring complete lossless migration of all 121 current requirements;
- preserve strict ownership and executable checks for the smaller active set.

The investigation must define an objective carry-forward rule rather than selecting requirements opportunistically.

## Required evidence and analysis

Before deciding, inspect at least:

- `docs/design.md` and every current residual requirement family;
- `docs/requirements.csv` and `docs/spec/decomposition.csv`;
- `docs/spec/README.md` and active domain specifications;
- ADR-0013, ADR-0014, and later accepted ADRs that depend on the current authority model;
- active amendments and their affected requirements;
- architecture and requirement checkers that assume `docs/design.md` remains a residual owner;
- completed tranche evidence whose authority claims would be affected;
- current WP-200/WP-300/WP-400 blockers and the Property Read architecture gate;
- current implementation, so that retained contracts are not selected from prose alone.

Classify the active requirement set into at least:

- indispensable current architecture or safety invariants;
- contracts required by the current Property Read vertical slice;
- contracts required for the declared v1 release target but not the immediate slice;
- useful deferred design input;
- duplicate, speculative, premature, or superseded requirements;
- requirements already made redundant by a stronger architecture or domain owner.

Quantify the result. A decision should not rely only on the total requirement count or document size.

## Decision criteria

The chosen direction should optimize for all of the following rather than only documentation simplicity:

1. one understandable current authority model;
2. no silent loss of indispensable architecture, safety, cleanup, generation, resource, or portability constraints;
3. a substantially shorter path to an executable production-boundary Property Read composition;
4. review and checker effort proportional to implementation risk;
5. preserved Git history and recoverable prior reasoning;
6. clear treatment of completed tranche evidence;
7. no duplicate active behavioral owners;
8. no requirement retained merely because it already exists;
9. no implementation freedom created by deleting constraints without replacing the necessary ones;
10. a bounded completion plan that a fresh AI session can understand and execute.

## Questions the decision must answer

1. Is the current D3 decomposition still the technically preferred authority model, or has its preservation cost exceeded its value?
2. Which residual requirements are genuinely required for the v1 release target?
3. Can inactive historical requirements remain searchable without remaining normative?
4. Should a reset deliberately create a new design revision, such as v5.0, rather than pretending to be an ordinary v4.9 migration?
5. What is the minimum active normative set needed to continue WP-200, WP-300, and WP-400 safely?
6. Which current checkers should remain, be simplified, be replaced, or become historical?
7. How should accepted ADRs, amendments, audits, reviews, and completed evidence be classified after a reset?
8. Must the current Foundation authority candidate be integrated, abandoned, or treated as migration input under the selected direction?
9. What exact repository changes activate the chosen authority model atomically?
10. What rollback point exists if the new model proves insufficient?

## Constraints

- Do not physically destroy historical material; Git history or an explicit deprecated location must preserve it.
- Do not treat this Owner proposal as a predetermined instruction to delete `docs/design.md`.
- Do not continue opening additional D3 domain migrations merely to avoid deciding this topic.
- Do not change runtime or public API under this topic unless a separately admitted implementation tranche authorizes it.
- Do not claim that code is the specification by default; the selected model must still identify current normative owners.
- Do not preserve a requirement solely to keep an existing checker passing.
- Do not invalidate completed evidence silently; explicitly reaffirm, supersede, or historicalize it.
- Keep the Property Read architecture path and the declared protocol-neutral v1 target visible throughout the decision.

## Expected output

The AI should produce:

1. a repository-grounded comparison of Alternatives A, B, and C;
2. a classified and quantified requirement-retention analysis;
3. one selected technical direction with rationale and rejected alternatives;
4. the exact target authority hierarchy after the decision;
5. an impact map covering requirements, ADRs, amendments, specifications, work packages, checkers, audits, reviews, evidence, `PLAN.md`, and `PROJECT_STATE.md`;
6. a bounded migration or reset sequence with review and rollback boundaries;
7. an explicit decision on the current Foundation authority candidate;
8. an explicit statement of when WP-200 plan-artifact work may resume;
9. if the direction converges, migration of the stable conclusion into the proper authoritative artifacts and movement of this topic through `DECIDED` to `MIGRATED`.

The first execution step should be investigation and decision, not another residual-domain migration candidate.

## Investigation result

The investigation used the v4.9 mainline at
`6c01e07a446f51d413618474554b5eedcf5de23e` and inspected:

- all 121 expanded rows in `docs/requirements.csv`, every residual requirement
  family in `docs/design.md`, and all 14 target domains in
  `docs/spec/decomposition.csv`;
- the active planning and binding specifications, all five WP-100 amendments,
  ADR-0013 through ADR-0017, the architecture backbone, Reviews 02 through 06,
  and the D3 completion decision;
- the exact 21-path Foundation candidate and its 10-requirement authority
  change;
- completed WP-000/WP-100 evidence and their compile/source checkers;
- the Property Read gate, WP-200/WP-300/WP-400 contracts, and the open
  compiler/artifact issue in `workspace/0014-property-read-plan-artifact-boundary.md`;
  and
- the current Foundation, TD, Core, Discovery, Servient, protocol-binding,
  codec, and umbrella implementation rather than selecting contracts from
  prose alone.

The current authority distribution is 34 requirements at final owners, 84
residual in `docs/design.md`, and three in amendments. The Foundation candidate
would change that to 44/76/1, but doing so changes 21 paths, adds about 1,186
lines, and deletes or rewrites about 453 lines without advancing executable
composition. Ten further target domains would still require their own
reconciliation and review boundaries.

### Alternative comparison

| Alternative | Authority safety | Property Read critical path | Bounded completion | Decision |
| --- | --- | --- | --- | --- |
| A — continue D3 | Strong continuous identity and single-owner checking | Does not close the WP-200 compiler/artifact representation; repeats authority-only review before implementation value | Technically finite but operationally dominated by repeated multi-path migrations | Rejected |
| B — full reset | Smallest immediate document set | Short, but only by deleting constraints that completed evidence and lifecycle/resource safety still need | Bounded, but unsafe and evidence-invalidating | Rejected |
| C — bounded core reset | Keeps the exact current safety and vertical-slice set; classifies every omitted id | Removes unrelated residual migrations from the critical path while preserving the WP-200/WP-300/WP-400 boundary | One independently reviewed revision switch plus domain-local re-adoption at later entry | Selected |

Alternative A's preservation cost has exceeded its value because it treats
historical existence as the default reason for continued authority. Alternative
B fails the no-silent-loss and evidence-continuity criteria. Alternative C is
the only option that preserves enforceable current invariants while making
future detail justify itself against implementation and release needs.

## Quantified requirement classification

The carry-forward rule is objective:

1. a requirement remains active when removing it would reopen a current
   ownership, lifecycle, cleanup, generation, resource, security, portability,
   protocol-neutrality, or standards invariant; or
2. the registered Property Read gate or completed implementation evidence
   directly depends on it.

Requirements needed for a later v1 domain remain mandatory entry-review input,
but not current authority. Other historical requirements remain searchable at
the reset-base Git revision without gaining authority from their identifier.

The six classes below expand to all 121 v4.9 requirements exactly once.

### A — indispensable current architecture or safety: 41, active

```text
ARTIFACT-AUTH-001 API-OWNERSHIP-001 STD-BASELINE-001
PROFILE-AXIS-001 FEATURE-MATRIX-001 CRATE-DEPS-001
CONCUR-LOCK-001 CONCUR-USER-001 CONCUR-CRIT-001 CONCUR-LIN-001
HANDLER-CANCEL-001..002 PLAN-COST-003 PLAN-BOUND-001 FORM-COVERAGE-001
LIFE-EXPOSE-001 LIFE-EXPOSE-003 BIND-STORAGE-001
BIND-CALL-CANCEL-001 BIND-HOST-CANCEL-001 DIR-SCOPE-001
API-SECURITY-001 CONSTRAINED-STORAGE-001..002
CONSTRAINED-PROGRESS-001 CONSTRAINED-WORK-001 CONSTRAINED-OWN-001
RES-LIMIT-001..003 CAP-OVERFLOW-001 API-SOURCE-TIME-001
API-TYPES-001 API-HOT-ID-001 ERR-TAXONOMY-001 ERR-RETRY-001
ADMIT-TXN-001 ADMIT-MEM-001 HANDLE-DROP-001 HOST-ASYNC-001 TIME-001
```

### B — current Property Read composition: 21, active

```text
IMPL-CONFORM-001 DOC-RUNTIME-001 PLAN-COST-001 PLAN-SET-001
PLAN-ARTIFACT-001 FORM-FINALIZE-001 FORM-OWNER-001 HANDLER-API-001
HANDLER-VALUE-001 API-PAYLOAD-001 BIND-REG-001 BIND-ROUTE-001
BIND-DELIVERY-001 BIND-IO-001 BIND-MEM-001 LIFE-EXPOSE-002
STATE-EXPOSE-001 STATE-BIND-001 STATE-INFLIGHT-001
CLEANUP-RECORD-001 API-RESOURCE-001
```

### C — declared v1, not the current vertical slice: 34, inactive until domain entry

```text
DOC-RUNTIME-003 HANDLER-SUB-001 PLAN-COST-002 PLAN-INDEX-001
PLAN-LAZY-001 PLAN-REQUEST-001 PLAN-CACHE-001 FORM-FINALIZE-002
BIND-OUT-001 BIND-PROGRESS-001 SUB-STORAGE-001 SUB-DATA-001
DIR-CONTRACT-001 DIR-AUTH-001 DIR-SNAPSHOT-001 DIR-WATCH-001
API-DIRECTORY-POLL-001 DIR-STREAM-001 SEC-PERF-001
VALIDATE-COMPILE-001 VALIDATE-REUSE-001 API-CODEC-001
CONSTRAINED-SCHED-001 RES-LIMIT-004 RES-PROFILE-001 CAP-STATUS-001
OBS-PROFILE-001 API-DISCOVERY-EXEC-001 API-OPTIONS-001
STATE-SUB-001 STATE-DISC-001 PRODUCER-EMIT-001
HOST-SHARD-001 HOST-SHARD-002
```

### D — useful deferred design input: 15, inactive

```text
DOC-RUNTIME-002 JSONLD-PREFIX-001 HANDLER-STORAGE-001 TD-MEM-001..002
PERF-BENCH-003 PERF-SCALE-001 PERF-ADMISSION-001 PERF-PEAK-001
PERF-CALL-001 PERF-ACCOUNT-001 PERF-FANOUT-001..002
PERF-COMPLEXITY-001 PERF-INDEX-001
```

### E — premature or superseded freeze: 4, retired

```text
PERF-ALLOC-001 API-SURFACE-001 HOST-DEFAULT-001 HOST-DEFAULT-002
```

`API-SURFACE-001` is the clearest example: it freezes a broad cross-crate API
catalog before the constructible compiler/registration boundary exists, while
the API ownership registry and tranche compile fixtures are more precise.
Absolute zero-allocation and named host-policy freezes are implementation-stage
measurement or configuration decisions, not prerequisites for the current
composition.

### F — redundant under a stronger owner: 6, retired as requirement ids

```text
DOC-GOV-001 CHANGE-CONTROL-001 REFACTOR-GATE-001
PERF-BENCH-001 PERF-BENCH-002 PERF-BUDGET-001
```

Repository governance owns documentation, architecture change, and tranche
admission. The registered performance schemas, manifests, fixture lock, and
checker own executable performance identity and budgets. Repeating those
policies as residual behavioral requirements creates two update surfaces.

The resulting v5.0 core has 62 active requirements, a 49% reduction from 121.
The other 59 identities remain classified and recoverable rather than silently
deleted.

## Decision

Select Alternative C and implement it as a deliberate v5.0 authority reset.
ADR-0018 records the authoritative decision. v4.9 remains active until one
immutable v5.0 candidate passes independent review and a separate integration
checkpoint activates it atomically.

The target v5.0 authority hierarchy is:

1. a concise `docs/design.md` revision and source manifest;
2. the registered architecture backbone for cross-domain flows and invariants;
3. accepted ADRs for durable rationale;
4. a small registered domain-specification set plus only still-needed narrow
   amendments;
5. machine-readable API, state, resource, requirement, performance, and
   work-package projections; and
6. source/tests as implementation truth and executable conformance.

The active requirement registry will carry status as well as ownership. It
will keep all 121 identities classified, but only the 62 active identities may
authorize implementation or satisfy a gate. The 34 v1-deferred identities must
be re-adopted, replaced, or retired at their domain's work-package entry. The
D3 final-target DAG and migration-only checkers retire at activation.

## Impact map

| Area | Disposition |
| --- | --- |
| Requirements | Replace continuous 121-way active ownership with 62 active plus 59 explicitly inactive/classified identities |
| `docs/design.md` | Replace the residual monolith with a concise v5.0 revision/source manifest; preserve v4.9 at the reset-base Git commit |
| Domain specifications | Retain reviewed planning/binding contracts only for active ids; add the minimum Foundation/Core/runtime owners required by the 62-id set |
| ADRs | ADR-0001..0013 and ADR-0015..0017 remain decision input; ADR-0014 becomes superseded transition history; ADR-0018 owns the reset |
| Amendments | Retain only clauses still owning active completed contracts; otherwise historicalize after explicit evidence disposition |
| Work packages/gates | Property Read's 21 requirements stay active; later packages may reference v1-deferred ids only as entry-review obligations, not implementation authority |
| Checkers | Replace the 121-active/D3-target checker with classification, active-owner, inactive-admission, and exact-candidate checks; retain API/state/resource/work-package and completed-tranche checks where still truthful |
| Audits/reviews | Reviews 02/03 remain historical blocking evidence for v4.9; a new independent v5.0 activation review is required |
| Completed evidence | Reaffirm, supersede, or historicalize every WP-000/WP-100 record explicitly; do not rewrite historical evidence blobs |
| Plan/state | Replace D3 continuation with D7 v5.0 activation and keep the Property Read/WP-200 blockers visible |

## Activation sequence and rollback

1. Freeze the mainline decision checkpoint and exact v5.0 candidate path set.
2. Build the complete non-implementation candidate: concise manifest, active
   specifications, classified registry, checker replacement, revision
   projections, evidence dispositions, and removal of D3 activation machinery.
3. Run candidate-boundary, classification/ownership, ADR, API, resource,
   state, work-package, completed-tranche, aggregate design, workspace, and
   valid feature checks.
4. Obtain an independent review of the immutable candidate. Any path or content
   change creates a new candidate.
5. Integrate the exact candidate in a separate checkpoint; only then does v5.0
   become active.

Before activation, rollback means declining the candidate and retaining v4.9.
After activation, the frozen mainline parent is the recovery point; restoring
v4.9 must restore its registry and checker set atomically rather than reverting
individual files.

## Foundation candidate and WP-200 disposition

Foundation candidate `2494f33fdfe49ec3c7ae850d20990e446e628865` is
abandoned as a D3 activation candidate. Its Foundation specification may be
reused as v5.0 input, but the 21-path D3 boundary and prospective 44/76/1
authority state must not be integrated. The commit remains a searchable audit
and rollback reference.

WP-200 plan/artifact implementation may resume only after:

1. the exact v5.0 reset candidate activates; and
2. issue 0014 freezes one host-erased/static compiler and artifact Rust
   representation, assigns Core SPI implementation once, and passes paired
   third-party authoring fixtures.

The authority reset does not itself admit planning/Core/TD source, a WP-200
compile-contract root, or either Property Read architecture fixture root.

## Migration projection

The stable conclusion is migrated through ADR-0018, the exact classified
transition manifest in `docs/spec/v5-authority-reset.toml`, the D7 plan entry,
architecture-governance target update, artifact/ADR registration, and the
continuation checkpoint. The next execution item is the exact v5.0 activation
candidate. This topic is therefore `MIGRATED`; candidate construction and
independent activation are tracked as project execution rather than an
unresolved choice.
