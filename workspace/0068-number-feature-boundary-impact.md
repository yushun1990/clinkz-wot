# 0068 Number Feature Boundary Impact

Status: MIGRATED; ARITHMETIC PORTION SUPERSEDED BY 0069/0070

Kind: ADR-0013 scoped implementation-impact review / TD feature authority amendment

Evidence baseline: master `8dd89736ba629c43bb4e0121957abd98f0a85a85` (#84).
Migration is a reviewable proposal in its independent PR boundary; it is not
independent acceptance or readmission.

Supersession: github-pr:86 accepted workspace topic 0069's bounded-atomic
Number direction. Workspace topic 0070 and
`docs/amendments/WP-100-bounded-atomic-number-v1.md` supersede this topic's
Display-driven exact-decimal base-graph comparison responsibility and any
wording that treats AP as arbitrary-precision arithmetic authority. The
feature-access finding itself remains current: bounded `ValidatedThing`
admission requires explicit `td/validated-thing -> serde_json/arbitrary_precision`
so it can borrow stable lossless Number text before enforcing the lexical
resource ceiling and retaining Number content.

## Finding and selection

#84 falsified direct `Number::as_str()` in a scalar-serde graph, showed that
downstream serde features do not set TD-local cfgs, and rejected a private
Serialize callback shape as stable access authority.

The [executable prototype](../tools/architecture-fixtures/validated-thing-feature-boundary/README.md)
selects explicit opt-in `td/validated-thing`, forwarding serde AP. Bounded
admission then always has the public borrowed text API. Ordinary TD base and
default users keep their existing APIs. Under the current bounded-atomic
authority, capability-off synchronous Basic does **not** need a Display-driven
exact-decimal adapter: it performs the five selected predicates with ordinary
public binary64 projection, while capability-on bounded admission first uses AP
text only to enforce `number_lexeme_bytes_max` and preserve lossless content.

The detailed owner is the amended [admission record](../docs/work-packages/WP-100-consumer-validated-thing-admission.md).
The feature prototype models the manifest edge; production `td/Cargo.toml`
and validated Rust remain unimplemented. This changes availability of an
absent, unadmitted surface, not its signatures or ownership.

## Alternatives challenged

| Alternative | Current disposition |
| --- | --- |
| Force AP on all TD users | Rejected: changes ordinary base/default Number parsing and representation and removes the base graph instead of accounting for it. |
| Local AP flag with scalar formatting when false | Rejected: downstream can enable AP independently while the local flag remains false. |
| Private Serialize callbacks from #84 | Rejected: no stable public dependency contract. |
| Exact Basic only under the capability | Rejected: would create two Basic policies. |
| Materialize Number text inside a charged step | Rejected: adds input-sized temporary output and a second owned representation. |
| Explicit `validated-thing -> arbitrary_precision` capability for borrowed lexical access | Selected and retained: guaranteed borrowed text in bounded graphs with no private callback authority. |
| Synchronous Display-driven exact-decimal source | Superseded by 0069/0070: no longer required after the five predicates use bounded binary64 projection. |

## Scoped impact and authority migration

The affected requirements remain exactly `DOC-RUNTIME-001`, `ADMIT-MEM-001`,
`ADMIT-TXN-001`, `CONSTRAINED-WORK-001`, `CONSTRAINED-PROGRESS-001`, and
`CONSTRAINED-OWN-001`.

| Owner / evidence | Current disposition |
| --- | --- |
| Validated-Thing admission record | Owns capability edge, API floor, requested/resolved graphs, bounded atomic Number rule, lexical resource ceiling, permitted future manifest change and revised evidence obligations. |
| API ownership | Frozen TD validation items retain profile+capability qualification; signatures, paths and removed API dispositions stay intact. |
| Runtime safety / WP-100 core | Project AP as lexical access and bounded atomic numeric progress without duplicating the detailed contract. |
| Foundation specification | Existing `CodecInputBytes` and lifetime work own numeric work; one append-only `number_lexeme_bytes_max` ResourceKind is added, with no new WorkClass or ledger account. |
| Work-package index | Registers 0070 / PR #87 while preserving `planned` / `candidate` / `current`, dependencies, requirements, completion key and separate readmission review. |
| API schema checker / mainline CI | Continue to execute independent graphs including actual thumb compilation. No new gate or acceptance state machine. |
| Dependency compatibility | TD's future semver floor remains `1.0.149`, not an exact-source pin. AP is required for stable borrowed lexical access in validated graphs. |
| TD parsing / serialization | Capability activation can affect resolved serde Number representation; graph-invariant parsing is not promised. Retained Number content is lossless. |
| TD Basic | Existing typed `NumberSchema` `f64` and `IntegerSchema` `i64` rules stay unchanged. Only the five extension predicates use the amended finite-binary64 rule; failed/non-finite projection becomes `InvalidSchema`, non-Number remains absent, and ordinary binary64 rounding is deliberate. |
| #69 Foundation / #70 Context | Existing methods and Context seam remain unchanged except the append-only generated resource field required by 0070. |
| Completed WP-200/WP-300 / passed Producer gate | Their recorded source/evidence/status remain unchanged. Later readmission still requires exact-boundary reaffirmation. |
| Future Planning / Servient | Same view and retained owner; users request the TD capability explicitly through their own admitted manifest changes. |

## Evidence and stop condition

The 32-cell matrix remains current evidence for public source delivery, resolved
features, downstream serde-only negative imports and sibling-capability positive
imports on Host and actual thumb. It does **not** prove the later bounded
numeric algorithm, arena/resource model, frozen signatures, rollback, or
runtime allocator behavior.

None of the eight pre-readmission items is declared complete. Their current
numeric obligations are defined by the admission record, workspace 0070, and
the bounded-atomic amendment: threshold checks, lossless opaque Number
retention, short projection failure, binary64 rounding, step-budget `Pending`,
lifetime `Limit`, and bounded cancellation around one <=256-byte projection.
The prior full synchronous Display-comparison and byte-resumable exact-decimal
proof obligations are no longer current.

If a supported graph cannot provide stable borrowed text under the explicit
capability, or the <=256-byte atomic projection cannot satisfy the supported
constrained target, stop and reopen impact review. Do not fall back to
unbounded `as_f64()`, private callbacks, exact-decimal byte resumability, or a
profile-specific ceiling above the project hard maximum.
