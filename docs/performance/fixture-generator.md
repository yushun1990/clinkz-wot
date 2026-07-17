# Performance Fixture and Harness Contract

Status: normative implementation-support contract for design revision v4.8.

## Identity

Every performance case has a stable workload id and positive workload version.
The identity tuple is:

```text
(manifest schema version, profile, workload id, workload version,
 resource profile, fixture id, fixture content SHA-256, harness case,
 measurement SHA-256, runner fingerprint SHA-256)
```

Changing an operation boundary, fixture bytes, included engine phases, scale
set, or gating metric requires a new workload version or id. Renaming a display
label does not change identity when the tuple and semantics are unchanged.

The v1 measurement SHA-256 covers the profile, workload id, harness case, the
complete inherited `measurement` table, and the case operation boundary. A
contention case uses `contention:<name>` as its explicit boundary identity.

`docs/performance/fixtures.lock.toml` is the single fixture lock. Each entry
records generator version, seed, canonical recipe, and SHA-256 of the generated
fixture bundle. Manifest-level `fixture_digest` is the SHA-256 of the ordered
concatenation of `fixture-id`, NUL, raw fixture bytes, and NUL for every fixture
referenced by that manifest in workload-id order.

## Generator

`clinkz-wot-fixture-generator-v2` is implemented by
`tools/performance-harness`. It produces a deterministic binary bundle with:

1. canonical UTF-8 identity and recipe metadata;
2. an actual deterministic TD-like JSON document of the requested byte size;
3. independent deterministic byte sections for extension, string, and URI-template axes;
4. deterministic payload and requested page-entry document bytes; and
5. fixed-width actor, binding, schema-node, security-branch, subscriber, and TD-node records.

The recipe uses semicolon-separated `key=value` pairs in ascending key order.
The v2 keys are `actors`, `bindings`, `collection_sources`, `document_bytes`,
`extension_bytes`, `forms`, `handler_slots`, `page_entries`,
`page_item_bytes`, `payload_bytes`, `schema_nodes`, `security_branches`,
`string_bytes`, `subscribers`, `td_nodes`, and `uri_template_bytes`. Omitted
keys are zero. Unknown, duplicate, unsorted, or profile-inadmissible values are
rejected against `docs/resource-limits.csv`.
Generated sections, not only recipe parameters, are included in the locked
digest.

`forms` is the total form count for one Thing, not the length of one form
array. The generator emits it as an independent canonical `form-contexts`
section distributed over deterministic Property contexts, each with at most
`forms_per_context_max`; it rejects a total above
`forms_per_thing_max` or a required context count above
`affordances_per_thing_max`. `bindings` supplies independent contributor
identities. A candidate-selection adapter therefore constructs a selected
operation's candidates from the declared per-context forms and binding
contributors rather than placing an inadmissible form array in one context.
`collection_sources` provides exact, stable collection-subscription source
identities and is checked against
`collection_subscription_sources_per_subscription_max`; it never represents
per-affordance binding starts or local merge queues.
The base `document` and every `page-item` section contain no implicit forms, so
substituting the form axis never changes the document-byte or page-entry axis.

The scaling fixture is an axis bundle. Its document and each byte/record
section are independent maximum inputs; an adapter constructs the baseline and
then substitutes exactly one named axis section at a time. It must not
concatenate every maximum into one document while claiming
`vary_one_axis_at_a_time`. The v2 generator validates document, payload,
string, extension, form, page, binding, schema, security, subscription,
TD-node, and URI-template recipe values against the selected named profile.

For the handler workloads, `handler_slots` provides stable handler-slot,
callback, and replacement identities and is checked against
`handler_slots_per_thing_max`. Generic `actors` remains available for process,
caller, or contention identities that are not handler slots. `subscribers`
provides stable Producer subscription identities. The operation-mode,
cancellation, replacement, and transaction case matrices remain explicit
manifest inputs; they do not introduce an implicit fixture-generator default.

The handler closure uses these locked v2 bundles:

| Fixture | Seed | Canonical recipe |
| --- | ---: | --- |
| `FX-GW-020` | 4601020 | `document_bytes=1048576;forms=32;handler_slots=4105` |
| `FX-GW-021` | 4601021 | `document_bytes=1024;forms=1;handler_slots=32;payload_bytes=64` |
| `FX-GW-022` | 4601022 | `document_bytes=16384;forms=32;handler_slots=32;payload_bytes=64;subscribers=1024` |
| `FX-CS-015` | 4603015 | `document_bytes=65536;forms=16;handler_slots=256` |
| `FX-CS-016` | 4603016 | `document_bytes=1024;forms=1;handler_slots=8;payload_bytes=64` |
| `FX-CS-017` | 4603017 | `document_bytes=16384;forms=16;handler_slots=4;payload_bytes=64;subscribers=256` |

The architecture-review closure adds the following stable workload families:

| Profile | Workloads | Contract covered |
| --- | --- | --- |
| Gateway | `PERF-GW-023` through `PERF-GW-027` | Compiled emission-target lookup, exact 1/4/16/64 binding scaling, slow-binding lane isolation, exposure target construction, and one native collection-subscription start. |
| Constrained | `PERF-CS-018` and `PERF-CS-019` | Retained 1/4 binding publication progress and one caller-owned native collection-subscription start. |

The storage-bundle actor identities cover every Gateway handler slot, including
the Thing-level operations, and the complete constrained static-reference
maximum. Their generated TD-like documents remain within the selected
profile's forms-per-context limit; actor count, rather than a single form
array, represents the complete slot identity set. Cancellation bundles carry
one payload and enough actor identities to schedule on-time, late,
replacement, and reentrant cases. The Gateway subscription bundle carries the
profile subscriber maximum. The constrained bundle also carries its profile
subscriber maximum. An active subscription retains its ordinary teardown
obligation in `ProducerSubscriptionOwner`; it consumes `cleanup_items_max` only
if an actual failure transfers that obligation to `HandlerCleanupOwner`, with
durable residual recording when transfer capacity is exhausted. The setup and
teardown flavor arrays in each performance manifest enumerate their complete
cross-product explicitly.

## Harness

The executable harness supports three design-time operations:

- `verify`: parse the fixture lock and manifests, regenerate every fixture,
  verify content and manifest fixture digests, and reject unknown workload,
  fixture, requirement, or harness identities;
- `list`: print stable workload identity tuples in deterministic order;
- `run WORKLOAD_ID ADAPTER`: generate the locked fixture into a temporary file
  and execute an implementation-owned adapter with explicit manifest, workload,
  fixture, and result paths.

An adapter receives no implicit workload defaults. It writes one result JSON
document conforming to `docs/performance/result.schema.json`. The harness rejects
a result whose workload, version, profile, fixture digest, manifest digest, or
measurement identity does not match the selected manifest. Adapters are added by
the applicable implementation work package; design freeze requires the
executable generator/orchestrator and locked identities, while implementation
completion requires every gating adapter and numeric baseline.

Result schema v2 makes every gate executable. Every result records the exact
runner fingerprint object and its canonical SHA-256; changing board, CPU,
frequency policy, memory, OS or firmware, runtime, core topology, or clock
source therefore changes the comparable runner identity. For a manifest field ending in
`_max` or `_min`, the result must contain the numeric metric named by removing
that suffix, and the harness compares it with the declared threshold. A true
`require_*` field requires a same-named metric equal to `1`. Every name in
`report` must also be present. A gating case is accepted only with `status =
"passed"`, exactly the manifest's sample count, zero failed samples, finite
metrics, every required metric present, and every comparison passing. `failed`,
`unavailable`, `characterization`, zero-sample, partial-metric, and over-budget
results cannot close a gate. A non-gating characterization may use
`unavailable` only with zero samples and an empty metric object.

A case with `coverage_dimensions` declares an ordered Cartesian matrix. Each
named dimension must be a nonempty, duplicate-free string array in that case.
The harness computes `coverage_cell_count` and `coverage_sha256` from every
ordered cell using the `clinkz-wot-coverage-v1` identity and treats the
manifest `sample_count` as `samples_per_cell`. A conforming result must echo
those three exact values and report a total sample count equal to
`coverage_cell_count * samples_per_cell`; overflow is rejected. This makes a
matrix case structurally incomplete unless every declared cell is included.
The result's `require_complete_matrix_coverage` metric must also equal `1` when
that manifest gate is present. The coverage SHA-256 is computed in declared
dimension order with the last dimension varying fastest: the preimage begins
with `clinkz-wot-coverage-v1` plus NUL; each cell appends every dimension name,
NUL, selected value, NUL, followed by byte `0xff`.

The harness excludes transport I/O when the manifest says so and never silently
removes failed, cancelled, overflowed, or outlier samples. A case marked
`characterization` may report without gating; every other case has `gating =
true` and at least one absolute numeric budget or deterministic invariant.
