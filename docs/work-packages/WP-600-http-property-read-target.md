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

The [HTTP parser-policy amendment](#http-parser-policy-amendment) is an accepted
revision of this same tranche under ADR-0013. [Independent scoped review on PR
#198](https://github.com/yushun1990/clinkz-wot/pull/198#pullrequestreview-5562750022)
accepted exact head `a7acfa6c4779ce803c145babf653bd53e7ceebcb`; the index records
the revised contract as admitted. The tranche remains in progress: this review
does not qualify its subsequent production implementation. The original
admission and accepted shared/partial implementation evidence retain their exact
historical scope.

## Tranche and predecessor boundary

Deliver one optional Host HTTP/1.1 Consumer through public
`ConsumerPropertyReadRuntime`: one complete Consumer-only registration, one
eager consumed generation, one active call/result opportunity, two distinct
property targets in the same TD, and repeated reads after terminal release.
HTTP execution is std-only; portable cells verify the shared amendment and
regressions, not an embedded HTTP backend.
The TD/Core/Planning eligibility amendment below and the HTTP adapter belong to
this one tranche. The original admission ordered the shared amendment before the
binding; those accepted implementations retain their recorded scope. The
parser-policy amendment contains no production edits.

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

The shared-source rows record the original admission scope, implemented and
accepted in PR #193. This parser-policy successor changes only the existing HTTP
crate's private configuration/transport and focused evidence; an intersecting
shared change requires an explicit impact extension rather than reusing this
table as new permission.

| Owner | Exact change |
| --- | --- |
| TD / WP-100 | `td/src/validated.rs`: one paid scalar on existing semantic lending. Necessary exports only in `td/src/lib.rs`. Preserve ordinary Form construction, round trips and Basic validation. |
| Core compiler/plan / WP-200 | `core/src/plan.rs`, `core/src/binding_compiler.rs`, necessary `core/src/lib.rs` exports: scalar carrier and opt-in closed restriction below. No open compiler attestation or new artifact/callback family. |
| Planning / WP-200 | `planning/src/consumer_property_read.rs`: carry the scalar through existing preflight/materialization. Preserve source-free output and rollback. ADR-0021's inline amendment replaces the former all-bounds barrier with sequential pure lowering and coordinate-first failure precedence; renewed evidence must retain zero client invocation/publication. No protocol branch or Planning-owned vocabulary interpretation. |
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
that qualifies an SDK. The [parser-policy amendment](#http-parser-policy-amendment)
adds the successor identity and substitution obligations. No mutable/raw installed-client
escape. Registration construction performs no network I/O. The application
supplies and keeps alive the chosen runtime and any required entered context;
missing services reject before request acceptance. The SDK-specific deployment
recipe is implementation completion evidence, not a new shared runtime API.

## Paid metadata and protocol-neutral compiler amendment

At original admission, lending lost Form extensions/response overrides and the
closed compiler only copied targets. The accepted PR #193 shared amendment
implements the following API/semantic contract; this parser-policy amendment
preserves it without new shared source changes:

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
   identity/capacity checks. Complete materialization precedes compiler starts;
   sequential pure bounds/start/progress follows ADR-0021's ordering amendment.
   Every coordinate must finish before client invocation/I/O/
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

## HTTP parser-policy amendment

Amend only the Binding-private parser policy of
`WP-600-HTTP-CONSUMER-PROPERTY-READ`, on baseline
`098a829a2a5f23cc9fd3446ad2cbb7b942ea583c`. This is an ADR-0013 scoped admission
amendment, not a change to ADR-0013's general process or ADR-0023's resource
ownership contract. [Review 5562587383](https://github.com/yushun1990/clinkz-wot/pull/197#pullrequestreview-5562587383)
independently demonstrated a bounded stock-Hyper alternative to the private
fork. [PR #197](https://github.com/yushun1990/clinkz-wot/pull/197), at
`4b60ab0daf87648b3dc9ffa915dda8a949f2a0a4`, remains unmerged technical evidence;
its vendored SDK, production changes and policy identity are not this amendment's
implementation baseline.

The original 16-field/inclusive-2048-byte trailer envelope and expectation that
chunk extensions could use the whole wire allowance were conservative local
choices. No demonstrated engine, product or deployment constraint requires
those particular choices. Stock Hyper 1.12.0 shares main-head/trailer settings
and has a fixed cumulative extension restriction. A private fork can implement
the old envelope, but that does not establish that its update, regression and
registry-distribution burden is necessary. A checkout path build does not prove
the registry route ([Cargo dependency locations](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations)).

The required safety properties have different owners:

| Owner | Invariant preserved |
| --- | --- |
| Engine / SPI | Fund concrete carriers, requests, complete normal/late result backing and overlap before retention/effects. Preserve selected artifact/configuration/generation checks, seals, bounded diagnostics and unchanged rejected-input recovery. |
| Engine / Runtime | Preserve first cause, deadlines/cancellation, leases, original local/parent provenance, complete-call cancellation/transfer caps, prebacked cleanup/status/fallback and allocation-free engine handoff. No settlement before required local retirement. |
| Binding | Bound external input, concurrency, parser buffers/queues and growth at the actual protocol boundary. Own the original socket, driver, futures and disposal through release; refusal cannot lose an accepted owner. Specific parser numbers and comparison semantics are private policy, captured by identity. |
| Deployment | Own process/runtime/allocator/OS/FD/stack policy and lifetime. Neither SDK input ceilings nor measured peaks certify total memory, stack, finite disposal or recovery from process failure. |

The amended fixed policy is a conjunction of limits; being below the total-wire
ceiling alone does not promise acceptance of every framing shape:

| Dimension | Amended requirement | Disposition |
| --- | --- | --- |
| Main response head | At most 32 field lines and 8192 raw bytes inclusive, including status line and terminating CRLF | Unchanged. |
| Decoded result body | At most 4096 bytes | Unchanged; complete engine backing remains separately funded. |
| Trailer section | At most 32 field lines, including duplicates; raw length **less than 8192 bytes** (8191 maximum), including terminating CRLF | Replaces independent inclusive 16/2048. Use stock shared 32/8192 settings; reject at byte 8192, including unfinished input, and on a completed 33rd field without waiting for final section termination/EOF. A partial 33rd field remains subject to the byte/wire limits. |
| Chunk extensions | At most 16383 bytes under Hyper 1.12.0's cumulative extension counter; reject at 16384 | Replaces the requirement to accept extensions throughout the wire envelope. Counter includes extensions on every chunk, including the final zero chunk; its delimiter semantics are SDK-specific and must be evidenced. |
| Cumulative response wire / input | 32768 bytes including all framing; at most 4096 per socket read **and per paid progress callback** | Unchanged. No phase/chunk/decoder reset refills totals. |
| Connection / SDK buffer | One fresh call-owned connection; stock `max_headers(32)`, `max_header_size(8192)`, `max_buf_size(8192)` | Preserve concurrency/ownership; qualify actual retained capacities, rejecting-byte buffering, read-ahead and temporary overlap separately. |

Recommend the existing registry Hyper `=1.12.0` client/http1 path with those
settings and existing pinned application-runtime dependencies. The
[executable counterexamples](../evidence/WP-600-stock-hyper-policy-probe.md)
test simultaneous maximum head/body/trailers, complete and unfinished overflow,
duplicate fields, fragmentation and cumulative extensions. These SDK-level
results justify reviewing the policy alternative; they are not acceptance of a
production binding. Keep the original SDK incompatibility probes as historical
counterexamples to the old envelope, not successor failures demanding a fork.
Do not write an HTTP framing parser or add a generic runtime/retirement API.

Implement bounded chunked decoding and discard permitted trailer metadata
without merging it into the head or Core result. Trailer values cannot change
status, media, representation, routing or framing, and confer no integrity or
authentication guarantee. Reject case-insensitive declaration/actual trailer
overrides named `content-type`, `content-length`, `transfer-encoding`,
`content-encoding`, `trailer`, `upgrade`, `connection`, `host` or `te`.
Allow unannounced permitted trailers under the same bounds; announcement is not
an ingress barrier. Do not advertise `TE: trailers` for this discard policy.
Retain single-chunked-only transfer coding, TE/Content-Length ambiguity refusal,
and the other status/media/encoding/upgrade restrictions. This supplies framing
interoperability, not trailer interpretation ([RFC 9112 §§7.1–7.1.2](https://www.rfc-editor.org/rfc/rfc9112.html#section-7.1)).

The successor must fix review 5562587383's separate declaration-list defect:
after SP/HTAB handling, ignore permitted empty `Trailer` list elements and
validate each nonempty field name. Require positives for empty `Trailer:`,
trailing/consecutive empty members and a nonempty control within the existing
raw-head limits; retain invalid-name/forbidden-name negatives. Do not copy
`Trailer: X,` as a negative ([RFC 9110 §§5.6.1.2 and 6.6.2](https://www.rfc-editor.org/rfc/rfc9110.html#section-5.6.1.2)).

### Configuration identity and review boundary

The implementation must introduce a fresh versioned private encoding, distinct
from master's `clinkz-http-ip-v1`, the withdrawn v2 and PR #197's fork-specific
v3 (use `clinkz-http-ip-v4` for the successor). Encode actual SDK versions and
features, shared parser settings and inclusive/exclusive semantics, extension
counter, wire/read/callback limits, trailer discard/declaration/override policy,
advertisement, growth/concurrency, retirement/disposal and all retained URI,
status, media and permission restrictions. Record exact bytes/hash with the
implemented source; this document does not assign an unimplemented digest.

Derive the existing checked restricted configuration from that actual private
policy; client, compiler/support, complete registration, selected artifact and
diagnostics must agree. Changing any captured behavior changes the key. Test
old/new policy substitution in both directions before I/O with original-input
recovery, plus the existing restricted/unrestricted and generation negatives.
Never relabel an old artifact or mutate a captured configuration to enable the
new behavior. Shared key derivation, strict vectors, complete engine-policy
identity, schema 2, result limits and all 196 resource defaults are unchanged.

Independent scoped review judged this amendment's rationale, exact owner and
identity changes, counterexamples and impact map at the exact head recorded
above. The index projects that acceptance as `admitted`; original PR #191
acceptance remains the historical basis for the original scope. No source,
dependency, gate or predecessor acceptance transition is part of this
docs-only projection.

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

Use the fixed limits and comparison semantics in the parser-policy amendment
above. Body bytes exclude chunk framing. Reject unsupported informational
responses; no implicit draining after rejection. Qualify SDK parser,
continuation and disposal work separately. Test fixed-length, chunked and
close-delimited bodies, simultaneous maxima, exact/one-over limits and
fragmented/truncated framing, including unfinished overflow held open by a peer.

These are observable policy bounds, not an exact SDK heap claim. Select enforced
parser/read-buffer ceilings compatible with them, bound retained ingress before
body delivery, and report bounded read-ahead/temporary overlap. A body limiter,
Content-Length, count-only queue or OS limit alone does not qualify ingress.
If the selected SDK cannot enforce the amended envelope, change SDK/configuration
or return to scoped review. Qualify the actual private buffer/queue growth policy;
neither the raw byte limits nor normalized trailer-map lengths establish its
physical capacity or a universal heap/stack/disposal ceiling.

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

Run every indexed precheck before independent admission. Original admission used
a small pre-code scalar/restriction discriminator in the existing aggregate
fixture. Accepted shared completion replaced it with actual public paid-getter,
configuration and compiler checks while retaining unique negatives. Current
compiler/aggregate/Runtime tests establish that reusable baseline; authority
validation executes current resource/state/performance contracts. SDK-level
counterexamples supply no alternate aggregate, binding or Runtime qualification.

For this parser-policy amendment, rerun the existing indexed baseline checks,
the committed stock-SDK incompatibility tests and the standalone alternative
probes linked above. Production remains master's early Transfer-Encoding
refusal. No test-only enablement or vendored SDK is needed to review the
counterexamples. The independent reviewer must distinguish a passing SDK probe
from the still-required public Runtime qualification.

Original admission left completion files and gate states unchanged. Its shared
source changes required the following predecessor impact review and renewals
once in the successor completion file, preserving semantic case identities, old
results and strict/owned keys. PR #193 independently accepted those renewals;
they are historical obligations already satisfied, not new parser-policy work:

| Existing tranche(s) | Original shared successor disposition |
| --- | --- |
| WP-100-CONSUMER-VALIDATED-THING | Renew paid lending/cursor/layout/Ready/rewind/cancel evidence. |
| WP-200-CONSUMER-PROPERTY-READ-PLANNING | Renew owned logical-plan Layout/cost and identity/selection regressions. |
| WP-200-CONSUMER-COMPILER-SUPPORT | Renew restricted configuration, actual support/adapter/cursor/output Layouts, fixed callback work, refusal/recovery and substitution negatives. |
| WP-200-CONSUMER-PROPERTY-READ-AGGREGATE; WP-200-CONSUMER-OUTPUT-OWNERSHIP | Renew source-free scalar, sequential pure lowering with actual later-bounds rollback/coordinate-first diagnostics, whole-build work/peaks and original-account rollback/reclaim. |
| WP-300-CONSUMER-ONLY-REGISTRATION | Renew complete-owner capture/equality, changed configuration/support layouts and rejection recovery. |
| WP-400-CONSUMER-PROPERTY-READ-RUNTIME; WP-400-CONSUMER-RESOURCE-OWNERSHIP | Renew metadata/cell/policy correspondence, dependent carrier measurements and normal/late handoff/cleanup funding; no new Runtime implementation. |
| WP-100-CONSUMER-CALL-VALUES-VALIDATOR; WP-300-CONSUMER-PROPERTY-READ-BINDING; WP-300-CONSUMER-HOST-CALL-CONSTRUCTION; WP-000 | Reuse unchanged contracts/evidence. Run shared regressions at completion; a demonstrated intersecting layout/behavior change extends impact review rather than silently inheriting measurements. |

State topology, selection/seal semantics, schema-2 row meanings and
all 196 numeric defaults remain binding.

The preceding impact map records the original shared amendment. Its accepted
renewals on PR #193, and PR #195/#196's accepted partial scopes, remain valid at
their recorded revisions. This accepted parser-policy amendment changes no
shared source or predecessor contract. Its narrower subsequent implementation
impact is:

| Affected owner/evidence | Required disposition |
| --- | --- |
| WP-600 admission/index and `http-resource-lifecycle` | Replace only the old independent trailer envelope and extension acceptance obligation with this policy after scoped review. Original 16/2048 incompatibility evidence remains true for its historical contract. PR #196's SDK-correction-only recommendation is superseded by the reviewed policy alternative, not silently reclassified as SDK compliance. |
| Existing HTTP production evidence | Keep fixed/close/refusal results and v1 identity at their original source. Successful fork/v2 paths are historical observations, not upstream production qualification. The current refusal stays until the replacement's bounded ingress and retirement are exercised. |
| HTTP call/client/compiler configuration and private transport | Renew actual layouts, retained/transient/peak/largest allocations, overlap, buffer/queue growth, continuation/native-stack and supported faults/disposal for changed paths. No fork measurement transfers to stock SDK production. Preserve full result backing and existing funding even where quotas/inline sizes are unchanged. |
| `http-consumer-property-read` and `http-authoring-feature-conformance` | Renew public two-target/source-free/100-read success across fixed/chunked/trailer/close paths, identity substitution, deployment recipe and registry-resolvable dependency graph; execute the registered regressions/completion checks. |
| All completed shared predecessors, WP-000 and passed gates | Retain acceptance/states and source-specific evidence. Rerun applicable regressions; extend ADR-0013 impact review only if a concrete shared behavior/layout change is demonstrated. No blanket reopening, repeated shared admission or gate closure. |

The replacement must test simultaneous main/body/trailer maxima; raw padding
and duplicate fields; 8191/8192 trailer bytes and 32/33 complete/unfinished field
cases; 16383/16384 cumulative extension bytes across the final zero chunk; and
exact/one-over 32768 wire totals whose body/extension limits do not mask the
wire boundary. Use fragmented, malformed, truncated and peer-held-open cases.
Rework the old 17-field/2049-byte unfinished probes to exercise the new rejection
boundaries; retain their old source as counterexamples. Add positives within the
new envelope so reject-all chunked/trailers cannot pass. Cancellation, deadline,
late-result, drain/root-loss, pressure/reuse and original-account/FD retirement
evidence remains required on the changed real transport paths. Report enforced
bounds, measurements/estimates and deployment assumptions separately; no finite
disposal or native-stack conclusion follows from parser acceptance/rejection.

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
| `http-form-eligibility` | Production paid scalar/source-free plan in all three cells; default/explicit GET/POST/header/unknown extension/response/additional-response/coding/subprotocol/scopes, including present-empty. Zero/short credit, Ready replay, rewind/cancel and round trips. Native/public Host restricted positives; unknown/present/media negatives; later bad coordinate causes zero client invocation/I/O/publication and funded rollback/retry of earlier compiled artifacts. Materialization/qualification failures still precede compiler starts. Actual allocation refusals, unchanged strict keys and restricted/unrestricted client/compiler/artifact substitution negatives. |
| `http-consumer-property-read` | Public Host registration, consume/select/read of two targets, then at least 100 alternating reads after terminal release. Independently implemented loopback peer asserts GET/path and distinct results. Destroy TD/build inputs after consume. No raw-client/legacy bypass or retained source; saturation returns unchanged input. |
| `http-resource-lifecycle` | Stock pinned SDK/source and independent raw peer: amended parser-policy simultaneous/exact/one-over/unfinished header/body/trailer/extension/wire cases, declaration-list positives and forbidden overrides; fragmented/chunked/no-length, malformed/slow/truncated/disconnected peer, statuses/upgrades/encodings, no retry. Caller cancel/drop, deadline/result races, invalid normal/late success, accepted error, drain/root loss preserve seals/first cause and show FD/future retirement. Retained/transient/peak/largest allocations, actual growth/read-ahead/overlap, continuation/native stack, supported fault injection, finite disposal work/allocations, allocation-free engine status/handoff and original-account release. Separate enforced bounds, measurements/estimates and assumptions. |
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
