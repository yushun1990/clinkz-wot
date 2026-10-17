# Host HTTP Consumer Property Read: admission

This records the separate ADR-0013 admission of
`WP-600-HTTP-CONSUMER-PROPERTY-READ` for design revision 5.1.
[The index](index.toml) owns its state, dependencies, source scope, executable
prechecks and completion keys. This document owns the scoped entry amendment
and completion contract.

[Independent scoped acceptance on PR #191](https://github.com/yushun1990/clinkz-wot/pull/191#issuecomment-6230142316)
accepted exact head `ca71f7ca356a483e76b3a2b989de180e6f2d3d31` against base
`78a7a7c99861fa48ac22fda2ae13c6d02aaca89f`, with no blocking findings and an
independent rerun of all ten indexed prechecks. The merged admission basis
`f26c686ea99c6aa5e4954d23c7499790da21e7f9` has the same tree as that reviewed
head. This transition grants bounded implementation admission; production
completion still requires the evidence and independent acceptance below.

## Tranche and predecessor boundary

Deliver one optional Host HTTP/1.1 Consumer through public
`ConsumerPropertyReadRuntime`: one complete Consumer-only registration, one
eager consumed generation, one active call/result opportunity, two distinct
property targets in the same TD, and repeated reads after terminal release.
HTTP execution is std-only; portable cells verify the shared amendment and
regressions, not an embedded HTTP backend.
The TD/Core/Planning eligibility amendment below and the HTTP adapter belong to
this one tranche. Implement the shared amendment first inside the admitted
tranche, then the binding. This admission PR contains no production edits.

The direct predecessor is completed/current
`WP-400-CONSUMER-RESOURCE-OWNERSHIP`. Its transitive closure contains all ten
completed/current Consumer predecessors: borrowed TD, call values, selection,
closed compiler, aggregate/output ownership, Consumer-only registration, binding,
Host construction and manual Runtime. WP-000 is complete. Reuse their accepted
evidence and the passed Consumer/Producer gates at their recorded revisions.
The successor's accepted schema-2 boundary governs HTTP; neither SDK qualification
nor broad WP-300/WP-400 completion is an extra tranche predecessor. WP-600's
package-level WP-300 completion dependency remains unchanged.

[ADR-0022](../ADRs/0022-first-production-binding-and-runtime-environments.org)
selects HTTP and preserves HTTPS/security as a release dependency.
[ADR-0023](../ADRs/0023-resource-ownership-and-policy-boundaries.org) supersedes
its SDK choice: choose, pin and qualify the client/parser and application
readiness runtime during implementation. No advance SDK fork/qualification
package, engine executor or generic retirement interface is selected.

PLAN's architecture-fixture audit checkpoint remains parallel work, becoming
an HTTP dependency only for a demonstrated intersecting defect. Preserve unique
negative probes and accepted evidence. Topics 0076/0081's broader facade/image/
reuse questions are outside this first-operation scope; migrated 0085–0087 supply
the accepted ownership/configuration boundary. No audit or fixture retirement
is added to this tranche's acceptance criteria.

## Owners, source and dependency edits after admission

| Owner | Exact change |
| --- | --- |
| TD / WP-100 | `td/src/validated.rs`: one paid scalar on existing semantic lending. Necessary exports only in `td/src/lib.rs`. Preserve ordinary Form construction, round trips and Basic validation. |
| Core compiler/plan / WP-200 | `core/src/plan.rs`, `core/src/binding_compiler.rs`, necessary `core/src/lib.rs` exports: scalar carrier and opt-in closed restriction below. No open compiler attestation or new artifact/callback family. |
| Planning / WP-200 | `planning/src/consumer_property_read.rs`: carry the scalar through existing preflight/materialization. Preserve all-bounds-before-start, source-free output and rollback. No protocol branch or raw TD access. |
| HTTP / WP-600 | Create `protocol-bindings/protocols/http/{Cargo.toml,src/,tests/,examples/}` as `clinkz-wot-binding-http`. Core/Foundation are normal shared dependencies; TD/Planning/Servient are test/example dependencies. SDK/runtime dependencies remain private to this crate. |
| Runtime / WP-400 | Use existing public Host provision/consume/read/cancel/drain/cleanup. No production Servient or Foundation changes. A demonstrated insufficiency requires a reproducer and scoped readmission. |

Root Cargo membership/lock may change only for the real optional binding member
and dependencies. Add its WP-600 crate inventory when its manifest exists;
no placeholder crate is created for admission. No umbrella facade/feature,
Directory, Producer, Zenoh, codec, workflow or generated-resource edit. Necessary
API-row updates and focused tests at indexed paths are included; no second
planner/runtime or per-task checker is authorized.

The new HTTP public surface is checked immutable `HttpConsumerConfig`,
`HttpConsumerConfig::for_cleartext_ip() -> CoreResult<Self>` selecting the fixed
entry limits below, and
`try_consumer_registration(config: HttpConsumerConfig, binding_id: BindingId,
binding_generation: BindingGeneration, diagnostic_ordinal: u32)
-> CoreResult<ConsumerCompilerRegistration<HostConsumerBindingRegistration>>`.
The explicit cleartext constructor is the permission boundary. Capture actual
configuration in the complete client/compiler bundle using Core's supported
Host adapter and restricted closed compiler. Its private-policy identity covers
actual wire/read/parser limits, URI/status/media restrictions and selected SDK
configuration; it is neither a caller-supplied final binding key nor an assertion
that qualifies an SDK. No mutable/raw installed-client
escape. Registration construction performs no network I/O. The application
supplies and keeps alive the chosen runtime and any required entered context;
missing services reject before request acceptance. The SDK-specific deployment
recipe is implementation completion evidence, not a new shared runtime API.

## Paid metadata and protocol-neutral compiler amendment

The current lending projection loses Form extensions/response overrides; the
closed compiler only copies targets. Keep its existing source-reviewed support
boundary. These are the complete shared API/semantic changes:

1. TD adds `ValidatedPropertyReadForm::has_additional_form_metadata(&self)
   -> bool`, true exactly for nonempty `_extra_fields` or presence of `response`,
   `additional_responses`, `content_coding`, `subprotocol` or `scopes`. Presence
   is conservative: explicit GET/identity coding, empty scopes and
   `additionalResponses: []` also reject. No extension key/value interpretation.
   Existing inspection still pays complete validation. Compute/store the fact
   under a paid `DocumentNodes` semantic transition before Form Ready. The new Ready
   getter/retries inspect no source and refill no allowance. Preserve first
   cause, cancellation, rewind and source lifetime.
2. Core adds private `Option<bool>` to `LogicalInteractionPlan`.
   Existing `try_property_read` initializes `None` (unknown). Add
   `with_additional_form_metadata(self, present: bool) -> Self` and
   `has_additional_form_metadata(&self) -> Option<bool>`. Planning always sets
   `Some(actual_paid_fact)` during materialization, for every registration.
   The scalar owns no TD reference/map/allocation. Existing
   `BindingCompilerInput::logical_plan()` carries it. Unknown never means empty;
   an application validation promise cannot replace the paid Planning path.
3. Add `ConsumerBindingConfiguration::with_default_json_forms(self)
   -> CoreResult<Self>` and `requires_default_json_forms(&self) -> bool`.
   Original `try_new` retains unrestricted behavior and exact v1 key. The new
   method is idempotent and captures one immutable restriction. Let `K` be that
   configuration's original 32-byte v1 key. The restricted key is SHA-256 of
   `b"clinkz-consumer-default-json-forms-v1\0" || K`; reject equality with `K`
   or any of the 64 strict keys. All original contract/cell/capacity/private-policy/
   result-bound operands remain included through `K`. This additive binding
   encoding is `closed-consumer-default-json-forms-v1`; resource schema 2,
   complete-policy encoding and existing identity vectors are unchanged.
4. Client, complete registration, native compiler and private Host adapter capture
   the same actual restricted configuration. Existing support's
   `consumer_configuration()` projects it; add no support issuer. In restricted
   mode accept only metadata `Some(false)` and effective media
   `Some("application/json")`. Otherwise return `UnsupportedOperation` with fixed
   operation/form/plan/binding context from bounds, start and paid step, preserving
   identity/capacity checks. All coordinate bounds precede every start/I/O/
   publication. One bad coordinate fails the transaction with funded rollback;
   never skip it or silently execute default GET. Renew fixed callback/work costs.
5. Keep the 64-byte target-copy artifact, compatibility and payload format.
   Restricted/unrestricted configurations have distinct identities. A restricted
   `mock://` target is valid compiler input: URI/protocol eligibility stays with
   the actual binding. No HTTP/security branch enters Core or Planning.

This adds five API rows in existing portable cells and changes the two existing
types' private layouts. No new WorkClass/resource row/schema, extension object
model or lowering callback. Retaining unrestricted support preserves its accepted
copy primitive; globally restricting it or attesting arbitrary compilers would
change more authority. Raw TD in HTTP and treating unknown as false are rejected.

## HTTP execution and resource obligations

Only default GET, NoSec, effective `application/json`, HTTP/1.1 and IP-literal
cleartext endpoints enter. TD owns effective URI/security meaning; Planning
already requires one NoSec definition. Before raw acceptance, validate selected
artifact/configuration and URI. Reject other schemes/HTTPS, DNS, userinfo,
fragments, templates, nonempty URI variables and unsupported endpoint syntax.
Parse the owned target into call state once, without TD rescan/Form selection.
The existing target capacity remains at most 64 UTF-8 bytes.

Use a fresh call-owned connection per read. No pool, detached driver, redirect,
decompression, HTTP/2, upgrade, retry or reconnect. Only HTTP 200 with
`application/json` body bytes yields success through existing Core validation
and seals. Other statuses/encodings reject explicitly; no new JSON/TD ingestion
or schema decoder. Error causes remain fixed/bounded and omit peer bodies.

Fixed policy maxima: 32 response headers / 8192 header bytes, 4096 decoded body
bytes, 16 trailers / 2048 trailer bytes, and 32768 cumulative response wire bytes
including interim/chunk framing. Read at most 4096 bytes per socket read and
per progress call. Header bytes include status line, field lines and terminating CRLF; trailer
bytes include field lines and terminating CRLF. Body bytes exclude chunk
framing. No phase/chunk/decoder reset refills totals. Reject unsupported
informational responses; no implicit draining after rejection. Qualify SDK parser,
continuation and disposal work separately. Test fixed-length, chunked and
close-delimited bodies, exact/one-over limits and fragmented/truncated framing.

These are observable policy bounds, not an exact SDK heap claim. Select enforced
parser/read-buffer ceilings compatible with them, bound retained ingress before
body delivery, and report bounded read-ahead/temporary overlap. A body limiter,
Content-Length, count-only queue or OS limit alone does not qualify ingress.
If the selected SDK cannot enforce this envelope, change SDK/configuration or
return to scoped review; do not write an HTTP parser.

| Resource owner | Required funding and release |
| --- | --- |
| TD/Planning/Core | Pay scalar work before Ready; renew actual cursor/plan/config/support/Host-slot Layouts, build peaks and rollback. Charge controlled allocations before construction; retain original local/parent provenance. Scalar adds no backing allocation. |
| Registration/call | Use `ConsumerOwnedHostV2`, matching immutable configuration, 4096-byte result bound and declared full capacities of both media Strings. Measure actual carrier/envelope and constructor overlap. Preserve fallible raw construction and unchanged rejected request; Core post-acceptance seal installation stays allocation-free. |
| Normal/late result | Fund full `Arc<[u8]>` allocation/control/alignment, String capacities and overlap. Retain call/lease/result and accounts until release. Backing violations use existing accepted-error seals. |
| Cancellation/cleanup | Preserve complete engine-call cancellation cap, transfer cap plus destination/status/context and prebacked manual fallback before effects. Ordinary pressure cannot consume cleanup funding; first cause/late result survive refusal, transfer, drain and root loss. |
| Binding | Own socket, buffers, parser, readiness and connection futures. Enforce ingress/concurrency/growth refusal, retain funded accepted owners on pressure, and perform no blocking I/O or detached work. |
| Deployment | Supply live runtime, clock driving, allocator and OS/FD/process policy. Report reactor/socket baseline separately from engine charges and SDK measurements. Process OOM/abort/runtime destruction has no in-process reporting guarantee. |

No network effect before raw acceptance and first poll. Actual local socket/
future retirement precedes complete local settlement; it cannot certify whether
the remote read ran. Finite nonblocking SDK-local disposal may allocate under
accepted Host policy after required residual commitment; original funding stays
live through destruction. Engine status/fallback/handoff stays allocation-free.
No network wait, blocking join, replacement work or unowned tail. Drop alone is
not a remote cleanup certificate.

## Prechecks, impact and performance

Run every indexed precheck before independent admission. The focused test added
to the existing aggregate fixture uses real typed TD/paid lending/current compiler
to reproduce lost metadata and a small scalar/restriction discriminator. Its
proposed scalar is a pre-code model, not production paid-fact evidence. There is
no HTTP SDK, alternate aggregate or runtime. Existing public compiler/aggregate/
Runtime tests establish the reusable baseline; authority validation executes
current resource/state/performance contracts. No precheck qualifies a future SDK.
Replace the small model assertions with actual public getter/configuration/compiler
checks at production completion, retaining their unique negative cases.

All existing completion files and gate states stay unchanged in admission.
When source changes, mark affected predecessor impact review-required and renew
once in the successor completion file: borrowed TD lending/work; closed compiler
configuration/Layout/callbacks; aggregate/output plan/preflight/rollback; complete
registrations and Runtime/ownership captured metadata/carrier costs. Preserve
semantic case identities, old results and strict/owned keys. Independent successor
completion may reaffirm these impacts together; ten new admission cycles are not
required. The exact impact map is:

| Existing tranche(s) | Successor disposition when source changes |
| --- | --- |
| WP-100-CONSUMER-VALIDATED-THING | Renew paid lending/cursor/layout/Ready/rewind/cancel evidence. |
| WP-200-CONSUMER-PROPERTY-READ-PLANNING | Renew owned logical-plan Layout/cost and identity/selection regressions. |
| WP-200-CONSUMER-COMPILER-SUPPORT | Renew restricted configuration, actual support/adapter/cursor/output Layouts, fixed callback work, refusal/recovery and substitution negatives. |
| WP-200-CONSUMER-PROPERTY-READ-AGGREGATE; WP-200-CONSUMER-OUTPUT-OWNERSHIP | Renew source-free scalar, all-bounds barrier, whole-build work/peaks and original-account rollback/reclaim. |
| WP-300-CONSUMER-ONLY-REGISTRATION | Renew complete-owner capture/equality, changed configuration/support layouts and rejection recovery. |
| WP-400-CONSUMER-PROPERTY-READ-RUNTIME; WP-400-CONSUMER-RESOURCE-OWNERSHIP | Renew metadata/cell/policy correspondence, dependent carrier measurements and normal/late handoff/cleanup funding; no new Runtime implementation. |
| WP-100-CONSUMER-CALL-VALUES-VALIDATOR; WP-300-CONSUMER-PROPERTY-READ-BINDING; WP-300-CONSUMER-HOST-CALL-CONSTRUCTION; WP-000 | Reuse unchanged contracts/evidence. Run shared regressions at completion; a demonstrated intersecting layout/behavior change extends impact review rather than silently inheriting measurements. |

State topology, selection/seal semantics, schema-2 row meanings and
all 196 numeric defaults remain binding.

Applicable performance identities are PERF-GW-009/010/028/029 and PERF-CS-020/021.
Reuse fixed fixtures/budgets/accounting/oracles for shared regressions. Their
harness excludes transport I/O and is executable via the authority precheck;
it supplies no HTTP latency claim. Changed-engine allocation/work and actual
network resource/lifecycle observations are completion evidence. No new throughput
budget, benchmark runner or generic workload family is introduced.

## Completion evidence

Write `docs/evidence/WP-600-http-consumer-property-read.toml` with exact source/
fixture revisions, commands/results, dependency versions/features/lock,
configuration/runtime/deployment and independent acceptance references.

| Stable key | Required executable evidence |
| --- | --- |
| `http-form-eligibility` | Production paid scalar/source-free plan in all three cells; default/explicit GET/POST/header/unknown extension/response/additional-response/coding/subprotocol/scopes, including present-empty. Zero/short credit, Ready replay, rewind/cancel and round trips. Native/public Host restricted positives; unknown/present/media negatives; later bad coordinate causes zero starts/I/O/publication and funded rollback/retry. Actual allocation refusals, unchanged strict keys and restricted/unrestricted client/compiler/artifact substitution negatives. |
| `http-consumer-property-read` | Public Host registration, consume/select/read of two targets, then at least 100 alternating reads after terminal release. Independently implemented loopback peer asserts GET/path and distinct results. Destroy TD/build inputs after consume. No raw-client/legacy bypass or retained source; saturation returns unchanged input. |
| `http-resource-lifecycle` | Pinned SDK/source and independent raw peer: exact/one-over headers/body/trailers/wire, fragmented/chunked/no-length, malformed/slow/truncated/disconnected peer, statuses/upgrades/encodings, no retry. Caller cancel/drop, deadline/result races, invalid normal/late success, accepted error, drain/root loss preserve seals/first cause and show FD/future retirement. Retained/transient/peak/largest allocations, supported fault injection, finite disposal work/allocations, allocation-free engine status/handoff and original-account release. Separate enforced bounds, measurements/estimates and assumptions. |
| `http-authoring-feature-conformance` | External real Consumer-only construction and runtime/permission recipe. Renew affected predecessor cases/layouts/failures; execute registered Consumer/Producer regressions, portable Core/Planning/Servient cells, authority/workload checks, locked workspace tests and dependency inspection. No neutral-library HTTP/runtime/TLS dependency or inferred embedded claim. |

Requirement mapping for those keys (IDs are the registered requirement owners,
not new HTTP requirements):

- `http-form-eligibility`: DOC-RUNTIME-001/002/003, ADMIT-MEM-001,
  ADMIT-TXN-001, CONSTRAINED-WORK-001, CONSTRAINED-PROGRESS-001,
  CONSTRAINED-OWN-001, PLAN-BOUND-001, PLAN-ARTIFACT-001, PLAN-COST-003.
- `http-consumer-property-read`: PLAN-REQUEST-001, PLAN-SET-001, BIND-REG-001,
  BIND-IO-001, BIND-OUT-001, API-OPTIONS-001, API-PAYLOAD-001.
- `http-resource-lifecycle`: API-RESOURCE-001, RES-LIMIT-001/002/003,
  BIND-MEM-001, BIND-DELIVERY-001, BIND-CALL-CANCEL-001,
  BIND-HOST-CANCEL-001, STATE-INFLIGHT-001, CLEANUP-RECORD-001,
  HANDLE-DROP-001, HOST-ASYNC-001, ERR-TAXONOMY-001, ERR-RETRY-001, TIME-001.
- `http-authoring-feature-conformance`: CRATE-DEPS-001, FEATURE-MATRIX-001,
  plus renewed shared/predecessor evidence for the preceding groups.

Independent completion acceptance must cover changed shared contracts and
predecessor impact. HTTP Producer/bidirectional loop, explicit HTTP vocabulary
(including explicit GET), DNS, HTTPS/trust/security, streaming and embedded
networking remain successors. HTTPS/security remains a release prerequisite.
This tranche alone cannot close WP-600 or qualify v1 release/protocol parity.
