# 0073 Shared JSON Value Decode Boundary

Status: MIGRATED — value contract selected and projected; independent review required, no readmission

Kind: executable decoding counterexample and pre-readmission authority impact

Initial finding baseline: `605eeb618033a4a797b64f7ce205a3c9aeff89a9`.
Decision investigation baseline: `ba87d2933392442032c53bf3c217cbb7f71bc3b0`
(#116 integrated; exact master mainline validation passed).

The initial finding below is preserved as evidence history. The selected
decision and rejected alternatives are owned by
[ADR-0020](../docs/ADRs/0020-strict-json-value-decoding.org); the sole detailed
contract is [WP-100's strict value-decoding section](../docs/work-packages/WP-100-consumer-validated-thing-admission.md#strict-json-value-decoding-contract).
Migration of this candidate is not independent acceptance of the revision or
completion of its construction obligations.

## Finding

The strict-entry proof cannot equate JSON syntax kinds with the typed values
produced by current public `Thing` deserialization. Nor can it sort every
input object before interpretation and assume that decoding is unaffected.
Those assumptions fail before Basic validation and before the existing typed
Snapshot proof begins.

At the initial baseline the frozen WP-100 boundary required shared typed-field
decoding, one TD interpretation, lossless typed
semantics, unchanged ordinary deserialization, and a project-controlled
charged/resumable strict path without a private serde representation adapter
or an intermediate owned `Thing`/Value graph. It had no reserved extension-key
exclusion or explicit strict/serde collision exception. A bounded shared
decoder satisfying that combination had not been demonstrated.

The new [public-entry corpus](../tools/architecture-fixtures/validated-thing-schema-kernel/tests/support/decode_boundary_cases.rs)
identifies the missing semantic boundary. It calls real `serde_json::from_str`
into real TD types, compiling identically against unchanged production and
the existing #115 public-source candidate. Candidate generation still changes
only Basic semantics; no decoder substitution is introduced. The selected
fixture lock resolves serde_json 1.0.149, serde 1.0.228 and serde_with 3.18.0.

For this build, the literal input keys `$serde_json::private::Number` and
`$serde_json::private::RawValue` can activate the dependency's Value decoding
behavior. They are adversarial external JSON member names in this corpus,
not constants imported from private dependency code, a proposed TD vocabulary,
or stable product dispatch tokens. The observations below are build/graph
facts, not semver guarantees about those private spellings.

## Public-entry observations

| Input at root extension `x` | Decoded value or result |
| --- | --- |
| `{"ordinary":"true"}` | Ordinary object/string; reordering ordinary members preserves the typed associations. |
| `{"$serde_json::private::RawValue":"true"}` | Boolean `true`, rather than the source object. |
| `{"$serde_json::private::RawValue":"[true,false]"}` | Array of two booleans. A plain JSON string containing `[true,false]` remains a string. |
| `{"$serde_json::private::Number":"2"}` | Number with AP; ordinary object/string without AP. |
| `{"$serde_json::private::RawValue":"true","ordinary":true}` | Decode rejection in every tested graph. Reversing the two members succeeds and retains the object associations. |

The RawValue behavior exists even without AP: TD already enables `raw_value`.
AP additionally enables the Number case. Both capability-on graphs and
capability-off downstream AP therefore matter; enabling only the TD capability
is not the cause of the entire finding. Escaped spelling of the dollar sign in
the source member name still produces the same result after key decoding.

There is a second interpretation boundary inside TD's field composition.
With the ordinary member first, the two-member RawValue object is accepted at
root extension `x`. Placing that same object in a Property's `const` fails in
the sorted-Map graph but succeeds with `preserve_order`. The actual path is
`Thing` map buffering, `flat::take`/`from_value`, Property/DataSchema RawValue
dispatch and schema-context `flat::take::<Value>`. A later Value pass sees a
different first key after sorted storage. This is observed graph-local behavior;
it does not require graph-invariant parsing, which existing authority does not
promise. It does require a deliberate rule for when input order may be erased.

RawValue payloads can be interpreted again recursively. Three nested wrappers
in the corpus end at one Boolean. One object containing one string can produce
an array with 257 Boolean children: the source `x` shape has two JSON value
nodes, while the resulting `x` has 258 semantic value nodes. A syntactically
valid outer object with payload `true trailing` is rejected only when its
string payload is interpreted. These tests do not measure work, allocations,
stack use or target execution; they show why source syntax counts alone cannot
stand in for the complete decode work/structural/resource envelope.

## Basic and Number consequences

In an actual decoded Property, a RawValue string `2` used as `minimum`, with
`maximum: 1`, produces a Number and fails the unchanged bound check. A directly
constructed typed object carrying that same key and string is a non-Number,
so the same Basic predicate ignores it. RawValue string `0` similarly reaches
the existing `multipleOf` positivity check. A grammar-only strict decoder that
retains those literal objects would silently accept different input through an
otherwise correctly shared Basic kernel.

In AP graphs both wrapper kinds can decode string `1e309` into Number. All
five extension numeric predicates then reach #114/#115's deliberate amended
failure rule in the candidate. The unchanged production validator retains its
already recorded failed-projection-as-absent result. This is the authorized
numeric delta, not a new Basic rule or a failure of the shared Basic evidence.

Opaque wrapper strings also yield lossless Number text at lengths 63/64/65,
255/256/257 and 4096, despite having no Number token in that source object. The
corpus reuses #114's actual `projection_step.rs`: a decoded Number above the
fixture ceiling returns `Limit` before projection, with unchanged step and
lifetime remainders; within-ceiling projection pays its existing full debit.
The zero ceiling rejects decoded `0`. These are post-decode typed observations:
ordinary public deserialization has already allocated its input graph. They
prove neither a charged strict decode nor a pre-copy guard for string-origin
Numbers. An eventual strict adapter cannot rely solely on #95's outer Number
token guard if it intends to preserve this decoding behavior.

## Decision investigation

The initial preferred direction was project-owned strict value semantics with
an explicit collision delta. That preference supplied neither a decision nor
implementation permission. The follow-up investigation compared full legacy
wire preservation, a public generic Visitor, changing both serde and strict,
rejecting a namespace, and the separate literal strict boundary against the
current goals and actual field composition. ADR-0020 records the resulting
choice and the rejected alternatives rather than treating the initial
preference as evidence for itself.

The new [decision probes](../tools/architecture-fixtures/validated-thing-schema-kernel/tests/value_decode_contract.rs)
use public APIs in both actual TD source models, without modifying their
decoders or the existing candidate generator:

- In AP graphs, `deserialize_any` reports identical map/string Visitor events
  for actual tokens `1e+309` / `18446744073709551616` and a literal object
  carrying the corresponding Number-looking member. An always-object custom
  Visitor therefore loses Number-token provenance. A separate project lexer
  could keep provenance; this is not a proof that every public-API preservation
  path is impossible.
- RawValue-looking objects can supply the public Thing's known `title`, root
  `security` list and Property `observable` by becoming String/Array/Boolean.
  The wire difference cannot honestly be described as affecting only opaque
  extension storage. Equal literal Objects still reach the existing known
  field's kind checks; Basic itself need not change.
- Real public Thing decoding resolves escaped duplicate names to the last
  occurrence before field projection. A superseded wrong-kind `forms` or
  `minimum` does not invalidate its later replacement, and the last `type`
  selects the Integer variant. Null/default behavior differs across metadata,
  optional lists, schema opaque values and typed strings/booleans. A single
  global null or duplicate-rejection rule would create unrelated TD changes.
- Public AP Number decoding yields `0` for `-0`, `1e+0` for `1E0`, and
  `1e+309` for `1e309`, while retaining `1.00`. These results survive the
  real root/Property field paths. Three wire bytes `1e0` become four Number
  content bytes. A raw-token-only length proof cannot be extrapolated to the
  decoded content ceiling.

The literal algebra reference first captures public RawValue input, then
chooses public scalar or raw-member APIs by JSON syntax kind. It constructs
Objects directly with arbitrary names, never dispatches on the observed
private-looking keys and never reparses string content. It classifies the
wrappers, malformed embedded payload, escaped/reordered members, recursive
wrappers and 257-child string without creating those hidden semantic children.
Directly supplied literal typed Objects still use both actual Basic validators
as non-Numbers. This establishes the selected value rule's meaning with a
public-API reference.

That reference is deliberately synchronous, recursively allocated and
test-only. It is **not** the conforming strict decoder, shared field-policy
extraction, charged arena builder or constrained execution proof. It cannot
replace any full-construction readmission obligation.

Public API documentation consulted:
[serde's Deserializer implementation contract](https://serde.rs/impl-deserializer.html),
[serde_json Deserializer](https://docs.rs/serde_json/1.0.149/serde_json/struct.Deserializer.html),
[Value](https://docs.rs/serde_json/1.0.149/serde_json/value/enum.Value.html), and
[Number](https://docs.rs/serde_json/1.0.149/serde_json/struct.Number.html).
[RFC 8259](https://www.rfc-editor.org/info/rfc8259/), especially sections 3–4,
7–9, supplies the JSON kind/order/duplicate/interoperability context. Duplicate
last-wins and the public AP Number-content oracle are deliberate project
choices based on actual Thing behavior, not mandates of that RFC. The locked
dependency implementation was inspected to locate the observed conversions;
no private source or protocol was extracted into a candidate.

## Frontier and authority migration

0073 blocked the **complete** wire-to-typed construction, equivalence and
resource/progress proofs, because their input algebra, hidden nodes and Number
origins were unresolved. It did not invalidate already-typed arena/Basic/query
proofs or make disjoint signature/lifetime work impossible. Such work was
lower priority: it could not establish that the strict builder was building
the right values or that its full envelope counted the right work. More local
typed readback would not resolve that earlier dependency.

The selected literal strict boundary preserves ordinary serde, supplied typed
compatibility and shared TD field/Basic rules. Its explicit wire difference
comes from representation-driven Value/RawValue interpretation, including
propagation to known fields; it is not a hidden blacklist or a general parity
waiver. The exact contract, corpus classification and construction duties now
live in the admission record. Choosing that boundary removes the authority
uncertainty; demonstrating the bounded construction remains a blocker.

| Authoritative owner | Migration responsibility |
| --- | --- |
| ADR-0020 and ADR index | Decision rationale, alternatives and precise supersession |
| WP-100 admission record | Sole detailed strict value/field/parity contract, permitted flexible-bool extraction, differential and decoded-Number-length evidence |
| Runtime-safety specification; architecture goals, module and flow documents | Project extension preservation, the two value inputs and shared TD semantic owner without unconditional wire equality |
| Existing Number amendment | Clarify both token and decoded-content ceilings; retain computation/resource policy |
| WP-100 package and tranche manifest | Register the decision and exact source boundary; keep planned/candidate and all dependencies |
| Normalization state machine | Input/equivalence refer to the supplied typed value or literal/shared-field result; no new phase or terminal |

API signatures/removals, Foundation/Context, resource rows, WorkClasses,
Planning/Servient ownership and existing gate/evidence statuses require no
change. The decision probes run in the existing fixture/CI cells. There is no
new checker, task-state file or Consumer gate. Independent review must accept
this migrated authority revision before a later constructive candidate can be
assessed against it.

## Relation to prior evidence

The finding does not falsify #94's checked Layout/grow/seal mechanics,
#96–#109's already-typed fieldwise storage, #99's normalization of an already
constructed typed Object, #108's timestamp decoder, #110/#111's build-time
semantic queries, #112's build-scoped authority or #113's owned handoff.
#114/#115 correctly validate the typed values they receive. #93's narrow
configuration/precharge and #95's genuine JSON Number token stop also remain
valid within their stated boundaries. They simply do not resolve this earlier
wire-to-typed-value interpretation gap. No prior completion/gate claim is
reopened on the basis of an inference beyond those boundaries.

The eight pre-readmission obligations are affected as follows:

| Item | Actual contribution of this finding |
| ---: | --- |
| 1 | No signature or removed-surface proof; frozen API unchanged. |
| 2 | Exposes why the full decoder must demonstrate the existing catalog; establishes no new allocation proof or necessary new category. |
| 3 | Selects and migrates the explicit strict value contract; the existing wire corpus plus decision probes fix the permitted difference and unaffected field controls. Full shared field construction/equivalence remains unproved. |
| 4 | No change; borrowed query and owned post-drop evidence remain reusable. |
| 5 | Wire/Basic observations execute in the existing Host off/on AP/order/no-default/async cells. No new full-surface, downstream/sibling or constrained-runtime claim. |
| 6 | Strict eliminates embedded parsing and retains explicit duplicate/order/decoded-Number duties; compatibility retains string-origin Number guards. No whole-decoder progress/rollback proof. |
| 7 | Exposes hidden semantic nodes and Number input lengths that a syntax-only envelope misses. Supplies no complete supported M, peak or target resource proof. |
| 8 | Value authority is corrected; production Foundation/Context, resource schema, successor completion and Producer gate remain disjoint. Passing current checks does not constitute independent full reaffirmation. |

No item is complete as a whole. This topic's decision is migrated as a
reviewable authority candidate. It does not readmit the tranche, implement
production normalization or register a Consumer gate. Full signatures,
allocation/configuration/construction/progress/terminal/resource evidence,
supported-cell execution, owned Planning integration and independent prior-
evidence reaffirmation remain required at one exact head before a separate
admission-only transition.
