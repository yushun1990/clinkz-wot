# 0073 Shared JSON Value Decode Boundary

Status: DISCUSSING

Kind: executable decoding counterexample and pre-readmission authority impact

Assessment baseline: `605eeb618033a4a797b64f7ce205a3c9aeff89a9`.

## Finding

The strict-entry proof cannot equate JSON syntax kinds with the typed values
produced by current public `Thing` deserialization. Nor can it sort every
input object before interpretation and assume that decoding is unaffected.
Those assumptions fail before Basic validation and before the existing typed
Snapshot proof begins.

The frozen [WP-100 boundary](../docs/work-packages/WP-100-consumer-validated-thing-admission.md)
requires shared typed-field decoding, one TD interpretation, lossless typed
semantics, unchanged ordinary deserialization, and a project-controlled
charged/resumable strict path without a private serde representation adapter
or an intermediate owned `Thing`/Value graph. It has no reserved extension-key
exclusion or explicit strict/serde collision exception. A bounded shared
decoder satisfying that combination has not been demonstrated.

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

## Authority impact and alternatives

The immediate conclusion is an unresolved decoding-authority boundary, not
intrinsic impossibility of normalized storage, a new allocation category, or a
claim that current public TD violates an admission guarantee. Ordinary serde
has no such guarantee. The missing proof concerns the future strict entry.

| Approach | Assessment |
| --- | --- |
| Copy literal JSON kinds directly to Snapshot and sort before decoding | Falsified by the accepted typed values, Basic results and ordering examples above. |
| Deserialize a complete Thing or opaque Value graph first | Supplies the observed oracle but moves synchronous recursive work, nested allocations and cleanup outside the frozen strict entry/catalog. Precharging an entire scan does not make it resumable. |
| Hand-code dispatch on the observed private marker keys | Copies dependency-private representation behavior and adds another interpretation authority; it is not authorized by the existing public-API/semver boundary. |
| Add an undocumented rejection/namespace blacklist to strict input | Changes the wire acceptance boundary; raw-byte matching also misses escaped names. No such exception is currently authorized. |
| Retain current behavior through a genuinely shared bounded decoder | Conforming in principle, but requires a constructive public-API path for embedded JSON, repeated interpretation, order, Number spelling and all charged work/cleanup within the existing arenas. The current proofs do not supply it. |
| Explicitly define project-owned strict JSON value semantics and review the collision delta | A credible simpler authority alternative: keep existing serde and typed compatibility behavior, share TD field rules above a project-owned value-decoding boundary, and explicitly delimit any strict/serde differences. This changes the present unconditional shared-interpretation claim and needs impact review before adoption. |

The preferred investigation direction is the final alternative: make the
project's strict value semantics explicit, instead of freezing dependency-
private wire collisions as a new public compatibility promise. This preference
is not accepted authority. The impact review must decide the precise input
set, collision/default/field-composition behavior and permitted extraction;
demonstrating the fully conforming shared-public-API alternative remains a
valid way to falsify the need for an authority amendment. Neither relaxing
strict/typed agreement nor adopting a blacklist is authorized by this topic.

Resolution has a falsifiable boundary: one explicitly reviewed decoding
contract must classify this corpus and preserve all unaffected field behavior;
one source-level construction must then exercise that contract from both
entries without private-token replication, an uncharged scan or an extra
owning category. It must preserve order until the contract permits sorting,
charge every embedded interpretation through the same lifetime remainder,
apply Number ceilings before projection/lossless retention and integrate the
existing diagnostics/rollback obligations. The frozen public signatures,
resource rows, WorkClasses and allocation catalog remain binding meanwhile.

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
| 3 | New differential wire corpus fixes a real strict/typed semantic boundary before Basic. Full construction/equivalence remains unresolved. |
| 4 | No change; borrowed query and owned post-drop evidence remain reusable. |
| 5 | Wire/Basic observations execute in the existing Host off/on AP/order/no-default/async cells. No new full-surface, downstream/sibling or constrained-runtime claim. |
| 6 | Exposes embedded parsing, repeated interpretation/order and string-origin Number progress obligations. Reuses the existing post-decode numeric guard; supplies no whole-decoder progress/rollback proof. |
| 7 | Exposes hidden semantic nodes and Number input lengths that a syntax-only envelope misses. Supplies no complete supported M, peak or target resource proof. |
| 8 | No authority, production Foundation/Context, resource schema, successor completion or Producer gate change. Passing current checks does not constitute independent full reaffirmation. |

No item is complete as a whole. This topic makes the decoder assumption
reviewable; it does not migrate an authority change, readmit the tranche,
implement production normalization or register a Consumer gate.
