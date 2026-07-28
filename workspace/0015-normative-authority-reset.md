# 0015 Normative Authority Reset Versus Continued Design Decomposition

Status: OPEN

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