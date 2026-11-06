# Executable reusable structured DataSchema

Implementation is based on master `e482b6a3e8c6f27d8877bcace0db443afa83bb45`
(the merged first reuse increment). This record scopes executable evidence;
it does not accept an architecture gate, close a milestone, or establish general
DataSchema, protocol, security or profile conformance. The PR records exact
validated revisions and command results.

## Architecture assessment and resulting change

TD already exposes recursive SchemaViews, resolved extension names, required
names, integer bounds and string constraints. Its NumberSchema bounds are f64.
Core's former Copy TypeSchema could only capture an outer value kind. Planning
therefore refused the meaningful structured assertions, while the actual
Consumer and finite Producer already had captured validation boundaries before
effects and delivery. JSON already preserved raw numeric tokens. Existing HTTP
I/O and complete Producer publication qualification were sufficient participants.

The implementation replaces TypeSchema with Core `data_schema::PreparedSchema`:
an alloc-only owned constraint tree, borrowed by data services. `ValueAccess`
lends codec-owned value handles. Core checks type/required/bound/length meaning;
JSON checks syntax/media/UTF-8, lends raw subvalues and decodes requested names
and strings. The kernel has no TD parser, JSON value type, std, Arc, protocol
case, application callback, authority or scheduler. A Core unit test evaluates
the same prepared object against native typed values without JSON.

Planning's shared normalizer captures constraints once for each prepared
operation/data-role content owner. Borrowed compatibility compares the actual
TD assertions without allocating another constraint tree. Property Read/Write
now join ReadAllProperties at that reuse boundary. Nested object property and
required-name order is not an execution dependency; collection member order
retains its existing contract. Each actual endpoint still undergoes admission,
service qualification and native lowering. This is not whole-TD normalization
reuse, schema interning, arbitrary post-source compilation or native retargeting.

HTTP Producer final checking borrows the original selected constraint owner
instead of constructing another selected schema merely to compare it. Its
configuration snapshots and accepted program views share immutable schema
backing. Each Producer still prepares/qualifies its own complete final TD and
actual behavior/native routes. General cross-Producer content reuse is unproven.

## Validated subset

| Meaning | Implemented boundary |
| --- | --- |
| Primitive kinds | Any present value and explicit null, boolean, integer, number, string, array and object kinds; absence of a payload role remains different from null/Any. |
| Nested objects | Named property schemas recurse; listed required names must be present; omitted properties are optional and unspecified names are unconstrained. Required names without a property schema receive an Any constraint. |
| Integer bounds | minimum, maximum, exclusiveMinimum and exclusiveMaximum from TD's exact signed i64 fields. Payload comparison preserves mathematical integer meaning for decimal/exponent spellings and integers outside i64. No f64 conversion or integer truncation occurs. |
| String bounds | minLength/maxLength count Unicode code points, including decoded escapes and surrogate pairs, rather than bytes or grapheme clusters. |
| Annotations | TD preserves metadata, default/unit/read/write annotations. Value validation neither applies defaults nor transforms values. Top-level Property access eligibility remains separately checked. |

Assertions requiring an explicit kind refuse when that kind is omitted.
NumberSchema bounds, multipleOf, const/enum, oneOf, constrained array items/counts,
pattern/format, content encoding/media transformations and unknown assertions
remain unsupported. Number bounds are deliberately refused at preparation;
reformatting an already rounded f64 cannot recover exact declaration meaning.
Integer bound declarations retain the current TD i64 input domain. This is a
TD 1.1 subset, not a general JSON Schema implementation. The semantic baseline is
the [W3C Data Schema vocabulary](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#sec-data-schema-vocabulary).

For example, one Read/Write Property can describe an object containing a required
`config` object, a required integer `level` with `minimum: 1` and
`exclusiveMaximum: 100`, a required `label` with 1–2 code points and an optional
boolean `enabled`. The HTTP integration writes level 42 at one endpoint while
another retains level 20. A level of 100, a missing required member, a fractional
integer spelling or an excessive label cannot become an application effect.

## Bounds, qualification and ownership

Each schema is limited to 64 nodes, depth 8, 32 object fields, 256 UTF-8 bytes per
name and 4096 total name bytes; collection preparation also checks aggregate
member constraints. A borrowed census checks support/capacity and prepays
variable construction/rollback before acquiring field/name storage. Core
constructors enforce the same shape/backing envelope. Scalar integer/string
constraints remain inline; object vectors/names have actual retained-byte and
bounded destruction accounting. A reused content reference preserves its first
account until final use and disposal. Actual instance records/policy/calls remain
separately funded, including refusal and retained normal/late results.

The retained HTTP Property Read installation path also reports its completed
program's schema Arc, field-table capacities and names. Its largest-allocation
declaration distinguishes individual allocations from total backing, and local
destruction covers both program references and the final constraint tree.
Entry and installation views share one charged schema owner until final disposal.

The JSON callback retains its 4096-byte body envelope and adds an encoded nesting
limit of 32, including unknown subtrees. Described duplicate names, including
escaped equivalents, refuse. Temporary key/string decoding stays within captured
scratch; no JSON DOM or normalized body copy is required. Complete callback work
is prepaid and scalar structured Producer allowances grow accordingly. Calls are
cooperative atomic callbacks with currentness/service checks before and after;
there is no mid-callback preemption or recoverable process-wide OOM claim.

Actual scalar validators explicitly qualify the complete borrowed schema; the
default support method covers only primitive type assertions. A complete
collection-map validator can independently qualify/validate its named member
schemas without a bare-scalar facet it never executes. Continuing Action and
MQTT notification preparation retain their narrower type-only profiles; a shared
kernel does not automatically qualify another native mapping or lifecycle.

The semantic module compiles without std on the Cortex-M0 target
`thumbv6m-none-eabi`, which lacks pointer atomics. It still uses alloc-owned
preparation storage. This is compilation evidence for the semantic module, not
allocation-free/static construction, a constrained Engine runtime, target I/O,
real-time execution or embedded HTTP support. A non-JSON ValueAccess unit test
does not qualify CBOR media interoperability.

## Executable evidence

The [HTTP integration](../../protocol-bindings/protocols/http/tests/structured_data.rs)
uses actual Engine HTTP Consumers, fully qualified Engine Producers and an
independent wire peer. It covers:

- Shared constraint/vector/name backing across independently bound endpoints,
  zero new content constructions during reuse, source/compiler retirement,
  independent policies/issuers, different values and exact final account release.
- Invalid Consumer input returned before I/O; raw invalid Producer input refused
  before handler authorization/effects; declared Producer output validation and
  Consumer rejection of invalid HTTP 200 responses from an independent peer.
- Original late output checked after cancellation/revocation, with rejected
  native evidence and schema funding retained through result disposal.
- Changed bound/required/type meaning, unsupported nested assertions and a
  type-only actual validator refused during preparation. Producer refusal
  precedes listener setup; a selector cannot weaken assertions.
- Cancelled preparation, zero-credit and saturated binding refusal returning the
  original owners; structured ReadAllProperties executes one native root read
  per endpoint with no leaf fallback.

[JSON tests](../../codecs/json/tests/structured.rs) cover optional/unknown members,
escaped duplicate names, Unicode lengths, depth refusal and exact values around
2^53 and i64 boundaries, huge exponents, fractional tails and negative zero.
Existing finite, Action/notification, external owned/incremental compiler and
feature/profile regressions accompany the interface migration; their unchanged
claims do not imply structured reuse for those other lifecycles.

[Retained HTTP installation tests](../../protocol-bindings/protocols/http/tests/producer_installation.rs)
reject a 32-field schema at a primitive program's artifact quota in all three
scopes, preserve the refused inputs for retry, and account for the primitive Arc.
A 64-node nested schema exercises exact and one-byte/credit-short live, compiled,
artifact, slot, cleanup and contiguous-allocation limits. Real listener cancellation
and direct root disposal observe field-table deallocation while the original bank
is still funded, followed by final release. These are retained Property Read
installation/resource claims, separate from complete Producer publication.

```sh
cargo test --locked --offline -p clinkz-wot-codec-json --test structured
cargo test --locked --offline -p clinkz-wot-binding-http --test structured_data -- --test-threads=1
cargo test --locked --offline -p clinkz-wot-binding-http --test producer_installation -- --test-threads=1
cargo check --locked --offline -p clinkz-wot-core --no-default-features --target thumbv6m-none-eabi
```

## Assumptions revised and remaining work

The first reuse increment's fixed-size Copy type assertions, zero scalar-schema
backing/disposal cost and ReadAllProperties-only borrowed comparison were
insufficient for structured values. They are replaced directly. Native program
clones now retain an immutable scalar schema owner; the external group compiler
updates its artifact bound for the larger actual member layout. Full family
validator qualification remains independent of unused scalar capabilities.

The boundary still permits replacing concrete Rust carriers, compilation and
runtime layout. Precision-preserving general number constraints need a suitable
TD representation first. More DataSchema assertions, shared normalized
preparation, other media, continuing Action/notification/Producer reuse, service
ownership across independent raw registries, fleet execution and static/embedded
runtime qualification remain separate work. None is made mandatory by the
presence of repeated work or by these passing tests.
