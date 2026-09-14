# 0068 Number Feature Boundary Impact

Status: MIGRATED

Kind: ADR-0013 scoped implementation-impact review / TD feature authority amendment

Evidence baseline: master `8dd89736ba629c43bb4e0121957abd98f0a85a85` (#84).
Migration is a reviewable proposal in its independent PR boundary; it is not
independent acceptance or readmission.

## Finding and selection

#84 falsified direct `Number::as_str()` in a scalar-serde graph, showed that
downstream serde features do not set TD-local cfgs, and rejected a private
Serialize callback shape as stable access authority.

The [executable prototype](../tools/architecture-fixtures/validated-thing-feature-boundary/README.md)
selects explicit opt-in `td/validated-thing`, forwarding serde AP. Bounded
admission then always has the public borrowed text API. Ordinary TD base and
default users keep their existing APIs. Their synchronous validators must
still migrate all five exact-decimal Basic predicates through the same TD
kernel; public Display supplies synchronous decimal bytes without assuming a
private serialization shape. It may not enter a bounded step.

The detailed owner is the amended [admission record](../docs/work-packages/WP-100-consumer-validated-thing-admission.md).
The feature prototype models its manifest edge; production `td/Cargo.toml`
and Rust stay unchanged. This changes availability of an absent, unadmitted
surface, not its signatures, ownership or resources. No new ADR is needed:
TD retains the semantic owner and both admission entries.

## Alternatives challenged

| Alternative | Disposition |
| --- | --- |
| Force AP on all TD users | Rejected: changes ordinary base/default Number parsing and representation and removes the base graph instead of accounting for it. |
| Local AP flag with scalar formatting when false | Rejected: downstream can enable AP independently while the local flag remains false. |
| Private Serialize callbacks from #84 | Rejected: no stable public dependency contract. |
| Exact Basic only under the capability | Rejected: two Basic policies, abandoning #83's synchronous responsibility. |
| Materialize text or replay Display inside a charged step | Rejected: forbidden temporary output or uninterruptible externally sized work. |
| Explicit capability plus synchronous public Display source | Selected: guaranteed borrowed text in bounded graphs, ordinary synchronous compatibility, no new Number API, input cap or allocation category. Full comparison proof remains required. |

## Scoped impact and authority migration

The affected requirements remain exactly `DOC-RUNTIME-001`, `ADMIT-MEM-001`,
`ADMIT-TXN-001`, `CONSTRAINED-WORK-001`, `CONSTRAINED-PROGRESS-001`, and
`CONSTRAINED-OWN-001`.

| Owner / evidence | Disposition |
| --- | --- |
| Validated-Thing admission record | Owns capability edge, API floor, requested/resolved graphs, base synchronous responsibility, permitted future manifest change and revised evidence obligations. Scalar-graph bounded admission is superseded. |
| API ownership | All 62 frozen TD validation items acquire profile+capability qualification; signatures, paths and removed API dispositions stay intact. |
| Runtime safety, WP-100 core, architecture 20 | Project the capability and base responsibility without duplicating the detailed contract. |
| Foundation specification | Clarifies Number byte charging applies during admission. No Foundation production behavior, WorkClass or ledger method changes. |
| Work-package index | Registers this review, adds narrowly scoped future `td/Cargo.toml` permission and the feature check. Profiles, dependencies, requirements, completion key and `planned` / `candidate` / `current` stay intact. |
| API schema checker / mainline CI | Recognize qualified cells and execute independent graphs including actual thumb compilation. No new gate or acceptance state machine. |
| Dependency compatibility | TD's future semver floor is `1.0.149`, the baseline publicly verified here; it is not an exact-source pin or an assertion that earlier releases lack the API. A lock alone cannot guarantee an available public method to a downstream resolver. Other workspace declarations need no change. |
| TD parsing / serialization | No production source changes. Capability activation has upstream AP unification effects on the shared serde package instance, including more exact/larger Numbers and presentation/rounding differences. The prototype tests this explicitly; graph-invariant parsing is not promised. Ordinary capability-off graphs retain their parsing. |
| TD Basic | Code inspection locates the five extension predicates in `data_schema.rs`; current float behavior is historical evidence. All four large-integer bound-pair deltas exist even in scalar base. Both public Thing and schema validation must migrate all five predicates in every supported graph together with future admission entries. Other Basic rules and typed numeric comparisons stay intact. |
| #69 Foundation / #70 Context | Production methods/files are unchanged. The source scan uses existing budget classes. No resource row, ledger account, allocation site or cleanup category is added. This is scoped non-intersection, not item 8 acceptance. |
| Completed WP-200/WP-300 / passed Producer gate | Their recorded source/evidence/status remain unchanged. Existing runs do not retroactively prove AP-graph acceptance. Later readmission still requires exact-boundary reaffirmation and the registered gate commands. |
| Future Planning / Servient | Same view and retained owner; users must request TD capability explicitly through their own admitted manifest changes. No successor readmission or implementation here. |
| Resource schema, state machines, architecture 10/30/50 and PLAN | No resource, lifecycle, ownership, gate, or roadmap delta. |

## Evidence and stop condition

The 32-cell matrix tests public source delivery, resolved features, downstream
serde-only negative imports and sibling-capability positive imports on Host and
actual thumb. Source scanning covers zero/small budgets, monotonic position,
lifetime exhaustion and sticky terminal causes. Synchronous source tests cover
long exponents, both decimal-cancellation spellings, signed zero, base typed
rounding and equality with AP's public borrowed text. The shared positivity
consumer is fixed-state; test byte buffers are assertion storage outside the
adapters, not proposed production allocations.

This removes #84's specific feature-access blocker for the selected boundary.
It does not prove the full exact comparator, strict decoder, normalized arenas,
frozen public signatures, ledger, Planning consumer, rollback or target
runtime/allocator behavior. None of the eight pre-readmission items is declared
complete. In particular items 3/6 must still demonstrate full synchronous
Display comparison passes and all-five-predicate parity, as well as bounded
borrowed-text comparison. Source delivery and positivity alone do not prove
those obligations.

If any supported graph's full shared construction cannot meet the unchanged
fixed-state, three-retained/four-temporary, stable-public, progress or API
constraints, stop and report a blocker. Do not retain float Basic on base,
introduce input-sized adapter output, hide Display in charged work, depend on
private callback partitioning, narrow the graph matrix, or broaden resource,
public signature or lifecycle authority. Independent acceptance and a separate
admission-only transition remain necessary before production work.
