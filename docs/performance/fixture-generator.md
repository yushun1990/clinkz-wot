# Performance Fixture and Harness Contract

Status: normative implementation-support contract for design revision v4.6.

## Identity

Every performance case has a stable workload id and positive workload version.
The identity tuple is:

```text
(manifest schema version, profile, workload id, workload version,
 resource profile, fixture id, fixture content SHA-256, harness case,
 measurement SHA-256)
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

`clinkz-wot-fixture-generator-v1` is implemented by
`tools/performance-harness`. It produces a deterministic binary bundle with:

1. canonical UTF-8 identity and recipe metadata;
2. an actual deterministic TD-like JSON document of the requested byte size;
3. deterministic payload bytes generated from the locked seed;
4. requested page-entry document bytes; and
5. fixed-width actor and subscriber identity records.

The recipe uses semicolon-separated `key=value` pairs in ascending key order.
The v1 keys are `actors`, `document_bytes`, `forms`, `page_entries`,
`page_item_bytes`, `payload_bytes`, and `subscribers`. Omitted keys are zero.
Unknown or duplicate keys are rejected. Generated documents and payload bytes,
not only recipe parameters, are included in the locked digest.

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

The harness excludes transport I/O when the manifest says so and never silently
removes failed, cancelled, overflowed, or outlier samples. A case marked
`characterization` may report without gating; every other case has `gating =
true` and at least one absolute numeric budget or deterministic invariant.
