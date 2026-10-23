# WP-600 Production Protocol Bindings

Machine-readable status and package dependencies are owned by
[`index.toml`](index.toml). [ADR-0022](../ADRs/0022-first-production-binding-and-runtime-environments.org)
owns the first-binding decision, alternatives and evidence impact.

[ADR-0023](../ADRs/0023-resource-ownership-and-policy-boundaries.org) revises
resource ownership and removes the architectural SDK choice. The completed
[Consumer ownership successor](../spec/consumer-resource-ownership.md) supplies
the shared schema-2 contract/readmission boundary consumed by HTTP entry. Reuse
its accepted completion at its recorded revision; this package selects no SDK
repair/qualification predecessor.

## Required production scope

Deliver an optional **Host HTTP/1.1 binding**, with HTTPS and explicit security
configuration before production release, through the accepted public Planning,
Binding and Servient path. Protocol-neutral crates acquire no HTTP, Tokio or TLS
dependency. The application links and registers the concrete crate explicitly.
The prospective crate is `protocol-bindings/protocols/http`; no crate, Cargo
feature or dependency is created by this strategy change.

WP-600 retains its WP-300 package-completion dependency. Exact capability
tranches may enter under ADR-0013 after their matching WP-200/WP-300/WP-400
predecessors and required architecture proof are complete. The Consumer gate is
passed. The [smallest HTTP admission target](WP-600-http-property-read-target.md)
records admission of the first vertical slice; the index owns its exact state
and bounded production implementation scope.

Its [parser-policy amendment](WP-600-http-property-read-target.md#http-parser-policy-amendment)
revises only the existing Consumer tranche's private trailer/extension policy
and configuration identity for the demonstrated stock-SDK alternative. The
amended admission requires independent scoped review; accepted predecessor and
partial production results retain their recorded scopes. SDK input limits do
not establish total memory, stack or disposal bounds. The amendment imports no
private fork or production enablement from PR #197.

Consumer execution uses the admitted source-independent consumed plan, selected
`OutboundRequest`, recoverable call construction, Core result seal, generation
lease, cancellation and cleanup. Producer execution uses compiled routes,
committed-closed guards, Servient publication, borrowed accept permits and owned
response opportunities. Neither side re-enters legacy selection or dispatch.
Bindings own protocol syntax/I/O/correlation and cannot call handlers directly.

Select and qualify concrete HTTP/TLS SDKs and the application-provided runtime
inside the HTTP Binding implementation tranche. Hyper/Tokio/rustls remain
candidates, not architectural requirements. Exact versions, features, supported
limits, progress and disposal evidence belong to that implementation's declared
envelope. Do not infer bounded receive allocations from a body-size adapter,
Content-Length, queue item count or process memory limit.

The first Consumer slice uses independent HTTP peers. The next tranche adds the
production Producer and a real public Consumer-to-Producer loop. TLS, trust and
security qualification follow before any production release claim. Broaden
one-shot operations only with corresponding shared authority. Streaming remains
subject to its existing domain entry and minimal ObserveProperty proof; this
decision admits no SSE, subscription or emission implementation.

This changes the first protocol and environment qualification, not the roadmap's
one-shot and long-lived operation-family objectives. Matching real HTTP evidence
must follow the shared capabilities admitted for v1, including the required
long-lived proof before broad streaming expansion. Package closure cannot waive
those predecessors by calling their implementation unsupported; protocol-specific
unsupported cells remain an explicit separate capability disposition.

The original entry envelope rejected form extensions through a paid presence
projection. The [native compilation amendment](WP-600-http-property-read-target.md#native-host-compilation-amendment)
replaces HTTP's closed lowering with bounded resolved input and Binding-owned
method/header/primary-response mapping. Supported explicit instructions and
native defaults are Binding obligations; unknown annotations remain preserved
without claims about their behavior. This scoped Host change does not expand the
accepted production Producer, security, Embedded or arbitrary-protocol envelope.

The [admitted Producer tranche](WP-600-http-producer-property-read.md) implements
the single-route source-free production service and public HTTP loop, including
whole-description coverage and the Host endpoint-driver replacement. Its six
completion obligations still require exact-head independent acceptance. The
earlier committed-closed path and mock gate retain their historical evidence
scope; neither supplies production completion acceptance.

## Resource and environment contract

The binding declares enforceable item/byte/connection/header/body/work limits
and a declared lifetime/transient transport envelope. Fund engine-controlled
carrier/handoff capacity in its original local/parent accounts before effects
and retain it until actual release. Binding-private growth uses the concrete
binding policy rather than mandatory transitive SDK engine quotas. Shared
infrastructure and references have separate physical owners, counted once each.
[Foundation](../spec/foundation.md#host-allocation-and-environment-failure-scope)
owns Host allocation-failure scope; [Binding SPI](../spec/binding-spi.md#shared-protocol-infrastructure-lifetime)
owns complete retirement and final-owner obligations.

Ordinary saturation/network failure/cancellation remain structured and owned.
Process OOM/abort/kill is a deployment failure, without in-process cleanup or
remote-effect recovery guarantees. Every surviving accepted owner still requires
bounded progress and truthful terminal disposition. Explicit shutdown and last
root loss must be tested against the actual HTTP dependency/destructor graph.
No worker, global pool or new generic retirement SPI is required in advance.

## Completion evidence

Required evidence is accumulated by exact capability tranches, without a new
global architecture gate:

- `http-consumer-property-read`: actual public consume/read/cancel/drain/reclaim
  against independent peers, selected targets, normal/invalid/late results and
  repeated calls without retained growth or legacy execution.
- `http-producer-property-read`: real route readiness/activation/publication,
  permit-gated handler dispatch, response ownership and shutdown; then the
  bidirectional production loop with the Consumer above.
- `http-resource-lifecycle`: pinned-source and executable exact-limit/one-over,
  malformed/slow/truncated input, constructor rejection, saturation, disconnect,
  caller/root loss, physical release, original accounting and finite Drop.
- `http-https-security`: authenticated TLS peers with explicit trust/configuration,
  certificate/hostname failure, bounded handshake/record storage and deadline,
  credentials absent from payload/logs, and cleanup after failed handshakes.
- `http-authoring-feature-conformance`: complete-role construction without dummy
  roles or engine edits, no concrete dependencies in portable cells, applicable
  workload/feature evidence, explicit unsupported capabilities and release limits.

The WP-300/WP-400 evidence and existing performance identities remain binding for
their applicable scopes. A raw SDK peer proves peer behavior, not the missing
production role. Throughput, RAM or constrained parity claims require measurements
at their actual implementation revision and deployment.

Package completion requires WP-300 completion and the production capability set
selected for v1, including the public HTTP Producer/Consumer path and HTTPS.
Applicable later one-shot/streaming tranches arrive with their own evidence;
unsupported protocol operations reject explicitly and cannot be silently lowered
to another operation. No real embedded protocol/backend is implied by completion.

## Deferred bindings and removal boundary

Zenoh and zenoh-pico remain future optional bindings. Their
[former detailed scope at the decision baseline](https://github.com/yushun1990/clinkz-wot/blob/6b3e478359e59ce21af233e38cc8337679d2d9a1/docs/work-packages/WP-600-protocol-bindings.md),
[qualification history](WP-600-host-zenoh-owner-decision.md), SDK probes and
Producer architecture feedback are preserved. They are not prerequisites of
HTTP admission, WP-600 v1 completion, WP-700 or release. Any resumed Zenoh work
must re-evaluate eligibility under current specifications; this change does not
qualify the existing SDK/backend or require its repair.

On resumption, assess the historical scope against current requirements and a
concrete deployment. Host Consumer/Producer execution, real pico C/backend
storage and progress, shared capability traces, input/correlation/generation,
subscription/emission and cleanup evidence remain relevant where claimed.
Historical package-exit language is not an active v1 dependency or implementation
admission. Git retains that full migration input rather than a second scope copy.

The v1 release surface must exclude or retire legacy Zenoh execution until its
target replacement is admitted. Preserve optional experimental source without
making its old selector/dispatch path a v1 API or routing any target request into
it. WP-300/WP-400 retain shared legacy-removal work; WP-700 proves final absence
in releasable cells. Adding HTTP does not implement the obsolete Zenoh migration
adapters. General protocol-shape neutrality and embedded production remain
separate evidence claims, not release prerequisites invented by this decision.
