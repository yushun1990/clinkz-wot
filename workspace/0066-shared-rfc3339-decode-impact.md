# 0066 Shared RFC3339 Decode Impact

Status: MIGRATED

Kind: ADR-0013 scoped implementation-impact review / authority amendment

Audited baseline: `ac3a8ea264af337441136c8da28067a2da58c6b9`

Finding: github-pr:79, counterexample head
`ec8ad531beefbe927b81920a24d5635744e26233`

## Question and bounded finding

Can the normalized ValidatedThing boundary share resumable date decoding with
ordinary Thing deserialization without a new public API, WorkClass, resource
row, allocation category, or broader architecture change?

The [preserved counterexample](../tools/architecture-fixtures/validated-thing-decode-boundary/README.md)
establishes that the accepted future source list omitted the actual semantic
owner, `td/src/rfc3339.rs`. Its synchronous `parse_rfc3339` calls
`Cursor::optional_fraction`, which inspects arbitrarily many fraction digits
even after nine digits determine the retained nanoseconds. The accepted
65,536-digit case performs 131,096 instrumented byte reads in one call. A
late-invalid suffix must still be rejected. These observations invalidate
source-list sufficiency, not the normalized representation or allocation
catalog. They do not establish the eight pre-readmission requirements.

Inspection of `td/src/thing.rs` confirms that `Rfc3339DateTimeField` delegates
both timestamp fields to that owner. `td/src/lib.rs` keeps the module private.
`td/src/rfc3339.rs` holds lexical, calendar/time/offset, precision, and trailing
input rules and returns a fixed-size `OffsetDateTime`; its parse path allocates
no input-sized state. The fraction precision counter already stops at nine.
The missing mechanism is a resumable phase/position boundary, not a need to
retain fraction text. Ordinary serde's synchronous caller contract and typed
compatibility conversion must remain distinct from strict admission progress.

## Alternatives and decision

| Alternative | Assessment |
| --- | --- |
| Keep the list and charge a node or atomically precharge the parser | Rejected: does not expose internal progress; deferred bulk work still executes outside the budgeted steps that supposedly paid for it. |
| Copy date rules into the normalizer, truncate input, or move the module through an allowed path | Rejected: duplicates or evades the semantic owner; prefix-only decoding misses late invalidity. |
| Impose a lexical length cap or switch date libraries/profiles | Rejected: changes accepted semantics and exceeds the finding's scope. Existing resource limits may independently reject work. |
| Permit private resumable extraction in the actual owner | Selected: the fixed scalar output and sequential scan support a bounded internal continuation while both adapters share one rule body. |

The selection is a constructibility judgment from source, not a completed
resumable prototype. The normative result is migrated solely into
[the admission record's shared decoder section](../docs/work-packages/WP-100-consumer-validated-thing-admission.md#shared-resumable-rfc3339-decode)
and its permitted-path list. That section owns semantic preservation,
progress/budget division, fixed state, and serde/from_json/from_thing entry
boundaries. This workspace record preserves the reasoning, not a second
implementation contract. No new ADR is needed: the existing TD ownership,
normalized storage, public entries, and resource model are retained; this
amendment specializes them at one omitted private source seam.

## Authority and dependency impact

The affected requirement set remains `DOC-RUNTIME-001`, `ADMIT-MEM-001`,
`ADMIT-TXN-001`, `CONSTRAINED-WORK-001`, `CONSTRAINED-PROGRESS-001`, and
`CONSTRAINED-OWN-001`.

| Owner inspected | Disposition |
| --- | --- |
| WP-100 admission record | Amend the private date-decoder contract and add only `td/src/rfc3339.rs` to permitted future production paths. |
| `docs/work-packages/index.toml` | Project that path and this impact disposition; retain `planned` / `candidate` / `current`, requirements, APIs, package dependencies, checks, and future evidence key. Prior withdrawal and migration reviews remain history in the admission record and topic 0065. |
| `docs/spec/foundation.md`, `foundation/src/budget.rs` | Existing `CodecInputBytes`, structural classes, unique step budget, and lifetime remainder suffice. No enum or method change; the observation count is not a replacement charging formula. |
| `docs/spec/runtime-safety.md`, `docs/state-machines.toml` | Existing first-cause, bounded cancellation, rollback, fixed diagnostics, and terminal ownership cover private decode state. No lifecycle transition changes. |
| `docs/resource-limits.csv`, Foundation ledger authority | Existing work/input limits and arena/inline-owner accounting cover the extraction. A fixed inline decoder adds no heap category, reservation kind, or cleanup owner. No generated resource projection changes. |
| `docs/api-ownership.csv`, WP-100 core boundary, architecture 10/20/30/50 | TD remains semantic owner, constructors remain fixed-work, and the existing view/handoff is unchanged. The admission record already owns detailed paths and signatures; no new public item or cross-domain projection is required. |
| WP-200 / WP-400 and `docs/spec/planning.md` | Planning still consumes the TD semantic view and Servient still owns admission/publication. No successor source, dependency, admission, or evidence change. |
| `PLAN.md`, Consumer gate | Roadmap and gate prerequisites are unchanged; no gate registration or advancement. |

## Evidence disposition and falsifiable boundary

The #79 fixture remains valid evidence of the **unchanged production parser**
and historical source-list omission. Its README now distinguishes that finding
from this amended future permission. Its exact byte-read assertions must not
be silently treated as a future decoder's required implementation formula;
eventual extraction must deliberately replace/reaffirm affected fixture
evidence while preserving the semantic oracle and long/late-invalid inputs.

The prior assumption that the pre-amendment production path list was sufficient
is superseded. The eight readmission items remain required and incomplete as
a set; this review completes none of them. Sorting, URI/number algorithms,
allocation-catalog sufficiency for the complete pipeline, full equivalence,
supported target execution, and bounded cleanup still need their own evidence.
Date extraction must later demonstrate shared semantic parity, small/zero
step traces, lifetime non-reset, and allocation/resource behavior within those
existing requirements, followed by independent exact-head acceptance and a
separate admission-only transition before production implementation.

Github-pr:69 Foundation behavior and github-pr:70 Context behavior are
structurally disjoint from this docs amendment: no implementation, API,
resource schema, or feature graph changes. Completed WP-200/WP-300 evidence and
the passed Producer Property Read gate retain their existing claims and
statuses; this change alters neither their sources nor their input contracts.
This is a document/source-impact assessment, not runtime reaffirmation or
completion of readmission item 8. No completion evidence manifest is created.

The scoped audit found no required new WorkClass, resource row/account, public
API, allocation category, or cross-domain architecture change. If subsequent
evidence requires any of those, a source outside the amended list, or changed
ownership/lifecycle/semantic behavior, stop at that boundary and open another
impact review. This migration authorizes no production Rust or admission.
