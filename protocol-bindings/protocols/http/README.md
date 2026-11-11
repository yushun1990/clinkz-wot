The qualified finite HTTPS Property Read/Write path supports these security
conjunctions and distinguishable alternatives between them:

| Complete branch | Native evidence | Required actual services |
| --- | --- | --- |
| NoSec | No credential header | TLS trust and application permission |
| Basic | Authorization: Basic | Basic credentials and Basic verifier |
| API key | X-Api-Key | API key and independent API-key verifier |
| Basic AND API key | Both headers | Both credential/verifier pairs |

Use `try_https_consumer_registration`, `HttpConsumerConfig::for_https_ip`,
`HttpClientTls` and instance-bound `HttpClientAccess`. `try_composed` configures
independent Basic/API-key slots; `try_new` is the Basic-only convenience, including
an initially missing/revoked credential. Producer `HttpFiniteProducer::try_https`
pairs `HttpServerSecurity::try_composed` with independent optional verifiers.
API-key-only and HTTPS NoSec require no dummy Basic service. Attach the same
server registration to `HttpFiniteConfig::with_security` before installation.
TLS 1.3 chain/expiry/IP-SAN verification precedes credential projection; TLS trust
is independent of authentication and application permission. NoSec still requires
application permission. API keys are opaque printable ASCII of 1–256 bytes.
Basic retains its bounded ASCII RFC 7617 subset.

TD lists and Combo allOf require every component. Consumer selection chooses the
first completely supported branch in declaration order, qualifying actual services
and native placement together. Producer preparation/final checking supports every
advertised alternative. Nested composition is allowed within shared processing
limits. Duplicate native placements in a conjunction and alternatives with the
same evidence set refuse Producer qualification; different named keys competing
for X-Api-Key are not independently selectable verifier domains in this subset.
Static Authorization/X-Api-Key headers, including case variants, cannot supply or
override credentials. Other placement names/locations, proxies and requested TD
scopes remain unsupported; they never disappear or default to NoSec.

Evidence presence chooses exactly one retained native branch. Missing, extraneous,
malformed or failed credentials never fall back to another branch. For a
conjunction, both verifiers must independently return the same Principal, including
identical claims in the same order. Different identities/claims refuse; no identity
inference or privilege union occurs. The application handler authorizes the verified
Principal separately. All verifier revisions are captured across the complete
sequence, so callback-time changes invalidate dispatch/delivery even when the last
verifier returns unchanged identity. Mutable services must publish non-reused
passive revisions; immutable services may use zero. Native refusal responses can
be delivered without authorizing application data.

Credentials remain in instance-bound access services and original replies. Grants
bind the exact conjunction, audience URL, operation, registration and incarnation.
`replace_credentials` / `replace_api_key` invalidate existing bindings and unread
results; rebind the unchanged selected definition to adopt new state. Original
result/reply and resource leases remain until final disposal, including cancelled,
failed and late interactions. Raw credential/capability formatting is redacted and
owned credential buffers are cleared locally; this does not promise erasure of
all Hyper/TLS/kernel copies. `RawProducerServient::revocation(key)` retains its
original capacity and can revoke only that incarnation.

This is Host/manual, IP-literal, loopback Producer, structured JSON Read/Write
support. Other authenticated operation families, OAuth2/Digest/Bearer/PSK,
challenge/refresh flows, other placements, proxies, TLS 1.2/mTLS, DNS, redirects,
certificate CRL/OCSP and constrained TLS remain unimplemented or explicitly
unsupported. Historical paths below retain their own scope. See the
[composition assessment and executable evidence](../../../docs/evidence/security-composition.md)
and [initial Basic/TLS evidence](../../../docs/evidence/td-security-https.md).

# Host HTTP Properties, Actions and Finite Collections

`clinkz-wot-binding-http` is an optional Cargo dependency used with the public
open `ConsumerRegistry`, `ConsumerAssociation` and `ConsumerCalls`.
`HttpConsumerConfig::for_cleartext_ip()` explicitly
permits cleartext IP access; `try_consumer_registration` constructs one complete,
Consumer-only registration without I/O. Its native compiler implements the
bounded NoSec / effective `application/json` Property Read/Write mapping from
TD-resolved semantic input. The [composed Write subset](#composed-property-readwrite)
uses separate input and completion roles. The retained `ConsumerPropertyReadRuntime`
uses `try_inline_consumer_registration` and remains a restricted Read adapter.

Prepare bindings with `ConsumerRegistry`, then register their original authority
and association with a finite `ConsumerOwner`. It controls registered lifecycle;
application instance handles and `ConsumerAssociation` views are non-revoking.
Construct execution capacity separately and supply it to each operation:

```rust
use clinkz_wot_servient_finite::consumer::{
    ConsumerOwner, StandaloneConsumerAssociation, ConsumerCalls,
};

let (bindings, funding) = prepared.into_execution();
let standalone = StandaloneConsumerAssociation::try_new(
    consumer.reference(), bindings, funding, instance_accounts.clone(),
).unwrap_or_else(|e| panic!("{:?}", e.error()));
let owner = ConsumerOwner::try_new(registry_accounts.clone(), 16, 32).unwrap();
let instance = owner.register_instance(consumer, &mut budget)
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
let association = owner.register_association(&instance, standalone, &mut budget)
    .unwrap_or_else(|e| panic!("{:?}", e.error()));
let calls = ConsumerCalls::try_new(work_accounts.clone(), 1).unwrap();
let mut call = association.read_property(
    &calls, "level", deadline, now, &mut budget,
).unwrap();
// Explicitly fence this original instance; this does not report native drain.
owner.remove_instance(&instance, &mut budget).unwrap();
// Schedule funded call.poll / calls.step turns to observe the fence and clean up.
```

`instance_accounts` must match the bindings' bank. Work and registry metadata
accounts may be separately provisioned. One owner can hold many registered
instances/facets, while many associations share one `ConsumerCalls` or use several
independent work owners. Native concurrency and original quotas still apply.
`remove_association` closes only one facet; parent removal/owner shutdown fences
all its registered facets. `replace_instance` requires the same numeric slot and
checked next generation. Old identity/facet handles cannot resolve a successor.
Dropping any returned handle leaves registration intact; `begin_close` or actual
owner Drop fences all its registrations. Standalone association close/Drop remains
a low-level passive ownership option; adoption consumes that revoking owner.

The application owns scheduling, fresh logical-clock observations and work credit.
Removal does not notify a parked driver. Native I/O wakes alone cannot cover a
silent peer, passive removal, abandoned handle or newly supplied credit. Arrange
lifecycle-command, cyclic or actual timer turns and retain runnable responsibility
after zero/partial credit. A logical `Deadline` does not install a timer or imply
automatic timeout. If aggregate progress polls using another task's context, also
arrange result-consumer turns; there is no automatic authority/result notification
registry. The [Read example](examples/read_property.rs) uses explicit periodic
progress, and [managed real-HTTP cases](tests/support/managed_capacity.rs) exercise
removal without another peer response.

Work-owner explicit close preserves `calls.step` cleanup access. Actual facade
Drop vacates and locally disposes abandoned calls, even while unrelated handles
retain shared state. Retained handles can finish their own cleanup; settlements
retain original results/accounts. This requires complete finite nonblocking local
abort from the selected Binding/services and claims no remote cancellation ACK,
rollback or completed network drain. General asynchronous cleanup transfer is not
implemented by this finite owner. No automatic eventfd/observer backend is linked.
[Runtime Safety](../../../docs/spec/runtime-safety.md) defines the full contracts.

The open finite path uses the bounded [prepared DataSchema subset](../../../docs/evidence/structured-dataschema.md).
Nested object properties/required names, exact integer inclusive/exclusive bounds
and Unicode string lengths are interpreted once by shared TD/Planning and Core;
JSON supplies bounded value access. Actual scalar or collection validators
qualify their complete data role. Property Read/Write and ReadAllProperties have
real structured Consumer/Producer and reuse evidence. Other finite collection
facets use the same kernel, with their existing role/mapping requirements;
their older type-only tests do not establish broader structured interoperability.
Continuing Action input/output use the same bounded structured subset; root
Action status preparation retains its narrower type-only profile.

Reads support HTTP/1.1 status 200, IPv4/IPv6 literals and fixed-length,
chunked and close-delimited bodies. One registration admits one unretired native
exchange. Completion, error or cancellation drops its connection and complete
driver before releasing occupancy. Retained terminal results keep their original
services, definitions and funding independently; destroying an old result cannot
release a successor's occupancy. Each read owns a fresh connection and its driver.
The selected immutable artifact supplies the exact path/query and Host authority.
The restricted Read path declines URI variables; the modern finite Property
facet supports the bounded dynamic profile below. The restricted registration
marks templates unsupported during preparation, so default Form selection can
reach a supported static alternative. Unsupported targets return the
original request before raw acceptance. No TD rescan, DNS, native connection pool, redirect, retry, decompression,
upgrade, background task or detached driver is involved.

The Consumer tranche is complete within its independently accepted scope; its
[evidence](../../../docs/evidence/WP-600-http-consumer-property-read.toml)
owns that exact boundary and subsequent amendments retain their own provenance.
The [parser-policy amendment](../../../docs/work-packages/WP-600-http-property-read-target.md#http-parser-policy-amendment)
defines the stock SDK envelope. Historical v1 refusal and the unmerged PR #197
fork are not evidence for the new stock SDK paths.

## TD-described finite response alternatives

The open finite Consumer and completely qualified finite Producer execute
Property Read/Write primary and additional responses. Shared Planning retains
TD order, response media and parameters, success (default false), and the named
schemaDefinitions override or ordinary output role. JSON decoding and schema
validation use the existing shared evaluator. Request input stays independent:
Write uses JSON Property data, while an additional response may have another
schema and independently selected media. Bodyless Read requires no request
encoder; an explicit response can select JSON even if Form contentType differs.

HTTP identifies alternatives by unique explicit `htv:statusCodeValue` values.
Primary remains 200 Read / 204 Write; these same codes may be explicit.
Other primary native mappings remain unsupported.
Each additional entry requires a code, and collisions, missing/malformed codes,
unhandled response instructions or unsupported representations refuse the whole
selected interaction. Native codes 200–599 are bounded by this profile; 202 is
unsupported because continuing acceptance is outside this finite mapping. Data
responses cannot use 204, 205 or 304. Empty responses use the strict existing
204-through-peer-EOF policy. In this status-only profile, additional Write
responses require a named value schema: another empty 204 cannot be uniquely
distinguished. Class annotations do not select a branch. Core
never interprets HTTP status classes: an explicitly declared 503 success and
201 application failure retain their TD meanings. Unexpected undeclared native
responses are Binding failures; the Binding never guesses a branch from data.

The supported leaf response representations are exactly `application/json` and
`application/json; charset=utf-8`, with UTF-8 JSON bodies <=4096 bytes, no content
coding, and a 32-byte media backing ceiling. These share the actual JSON value
validator; there is no new codec hierarchy or arbitrary charset/suffix promise.
JSON remains UTF-8 in both cases; the parameter changes no JSON encoding
semantics ([RFC 8259 §11](https://www.rfc-editor.org/rfc/rfc8259.html#section-11)).
The raw header guard checks the selected media exactly before Hyper normalization.
Write keeps response headers private until classification; empty 204 headers
remain private through EOF. Header/body/trailer/wire limits and call-owned
connection retirement remain in force. Input still uses exact application/json;
collection/status facets retain their existing exact JSON representation.

A handler uses its borrowed `HandlerContext::response_contract` to choose a
branch and declared terminal status. One behavior can serve Forms with different
response schemas or branch order. HTTP retains the exact matched native mapping
in the original exchange/reply; input/output validation and context lending use
that mapping, never the first target/operation match. No protocol Form token or
native route manager is added to Core.
ApplicationError is a validated application-declared failure with its payload,
not a CoreError or a claim that no effect happened. Producer finite Write success
still requires the handler to have completed the work. Consumer additional Write
responses remain effect-uncertain even when declared successful: this increment
does not supply a separate native completion proof for them. Ordinary primary
Write completion retains Completed. Local first cause, invalid response data,
application failure, remote-effect certainty and native delivery remain distinct.
Rejected Consumer data keeps the original response branch/body under settlement
funding. No response failure triggers automatic replay.

Consumer preparation covers every response of the selected interaction. Producer
preparation/final checking cover every advertised alternative before Serving,
retain exact response/schema owners and refuse final-TD drift. Security
composition, original authority, authorization revisions, accepted work,
cancellation and final physical release retain their existing contracts.

This capability is scoped to finite leaf Properties. Additional responses for
collections, continuing Actions, notifications, non-JSON media, general native
header/content-based discrimination and other ownership representations remain
unsupported. The caller-owned Read representation stays primary-only. The
restricted adapters below keep their separate admission/evidence scope.
[Executable response evidence](../../../docs/evidence/td-responses.md) describes
the actual tests and limits.

## Framing policy

Hyper `=1.12.0` is an **unmodified crates.io dependency**, with `client,http1` and
no default features. The workspace manifest/lock and dependency locations are
unchanged. No Cargo patch or vendored SDK is needed in an external application.

| Input | Enforced policy |
| --- | --- |
| Main response head | At most 32 field lines and 8192 raw bytes inclusive, including status line and final CRLF |
| Decoded body | At most 4096 bytes, checked before each copy |
| Trailer section | At most 32 field lines, duplicates included; raw bytes **less than 8192** including final CRLF |
| Chunk extensions | Hyper's cumulative counter **less than 16384**, across all data chunks and the final zero chunk |
| Complete response wire | At most 32768 bytes, including every framing phase |
| Socket ingress | At most 4096 bytes per read and per paid BindingPolls callback |

The raw-head wrapper uses httparse `=1.10.1` before handing bytes to Hyper. It
rejects unsupported statuses/versions, informational responses, content encoding,
upgrades, nonexact/duplicate JSON media, duplicate Content-Length, any coding
other than a single `chunked`, and TE/Content-Length ambiguity. A `Trailer` header
requires chunked framing. SP/HTAB around list members is removed and empty members
are ignored, including `Trailer:` and `Trailer: X,`. Nonempty members must be
valid field names. This implements the accepted [list recipient rule](https://www.rfc-editor.org/rfc/rfc9110.html#section-5.6.1.2).

Permitted announced or unannounced trailers are discarded. Neither declarations
nor actual trailers may contain `content-type`, `content-length`,
`transfer-encoding`, `content-encoding`, `trailer`, `upgrade`, `connection`, `host`
or `te`, matched without case. Trailers never modify status, media, routing,
framing or Core results and provide no integrity/authentication claim. Requests
send `Connection: close` and do **not** advertise `TE: trailers`.

Hyper owns all chunk framing; no replacement parser is added. Its builder uses
`max_headers(32)`, `max_header_size(8192)`, `max_buf_size(8192)` and `writev(false)`.
The shared settings reject trailer byte 8192 and a completed 33rd field without
waiting for section termination or peer EOF. A partial 33rd field remains under
the byte/wire bounds. Hyper counts extension octets after the introducing `;`,
excluding the terminating CRLF; the counter never resets between chunks.
Being below the wire limit alone does not guarantee that another limit permits
a particular framing shape. There is no drain or reconnect after rejection.

## Supported native compilation

The HTTP Binding supplies the default GET for `readproperty`; explicit
`htv:methodName` overrides it. GET, POST, PUT, PATCH and DELETE are supported as
bodyless read requests. Unsupported methods, wrong value types, duplicate expanded
instructions, explicit nulls and unsupported standardized HTTP terms yield an
unavailable Form, never a silent default GET. A strict Form choice cannot fall
through; another supported Form remains usable under default selection.

`htv:headers` supports at most eight entries and 1024 total name/value bytes,
with string `htv:fieldName` and `htv:fieldValue`. Header syntax is validated before
publication. Host, authentication/cookie, connection, upgrade and framing/encoding
headers cannot override this slice's target, NoSec or transport constraints.
Explicit Accept must equal `application/json` and may occur once. The default
Accept is emitted once. Primary `response.contentType` may explicitly be JSON and
`htv:statusCodeValue` follows the [finite response profile](#td-described-finite-response-alternatives); unsupported response media and additional
responses, coding, scopes and subprotocols remain unsupported.
Only endpoint NoSec is executable. An effective security definition with a `proxy`
server is explicitly declined during native compilation; it cannot silently turn
into a direct endpoint request. Another supported Form remains selectable.

Root/Property/Form/nested-schema/response/security extensions and semantic class
annotations arrive in TD's bounded resolved tree. Aliases and supported scoped
contexts are resolved by TD, not reparsed by HTTP. Inherited aliases retain their
definition-time meaning across later context entries, prefix shadowing and
vocabulary changes; unsupported expansion dependencies are refused before I/O.
`@vocab` expands standard and previously defined local prefixes in its preceding
context; definitions in the same new map cannot change that expansion. Labels,
SAREF types and unrelated vendor annotations do not change the request. Unknown semantics and advertised
Profiles are preserved without Profile/extension conformance claims; this slice
implements no activated Profile. The resolver is explicitly a bounded local-context
subset, not general JSON-LD: see
[its exact input/capacity/credit limits](../../../docs/work-packages/WP-100-consumer-validated-thing-admission.md#host-resolved-extension-lending).
Each eligible Form requires an atomic reservation of 65,536 `DocumentNodes`
credits; unused fuel is returned and actual work remains charged to the original
TD lifetime. One bounded semantic closure is live at a time. Sibling Forms cannot
exhaust its arena or impose unsupported local contexts. Shared root/property
restrictions can still make all affected Forms unavailable; this is an adapter
interoperability limit, not TD invalidity. The public example supplies that credit.
The production tests include 24 properties/48 Forms and actual repeated HTTP I/O.
The same public input exposes bounded borrowed standard Property/DataSchema facts,
nested/composed schemas and named response-schema dependencies alongside resolved
extensions. A non-HTTP compiler fixture consumes these facts and executes distinct
owned native instructions after the TD is dropped; it is not a real alternate
transport or general conformance proof. HTTP keeps its existing execution limits,
including no URI-template/coding/additional-response execution in the restricted
Read representation. The modern finite Property capabilities are scoped above
and below. Common fact access
does not imply support for those capabilities.
This concrete HTTP compiler uses inline artifacts and one atomic compile callback;
the open Consumer SPI also supports owned resumable compilers, exercised by the
independently linked datagram fixture.

Native method/header interpretation is private to this package. The Engine filters
Forms by declared scheme and effective WoT operation/security and owns publication,
plans, call resources and cleanup. No same-protocol scoring or Engine vocabulary
switch is involved. The separately registered Producer role and public loop are
described below; Consumer and Producer support policies remain role-specific.

## Identity, buffering and ownership

The private encoding is `clinkz-http-ip-v8\0` followed by the exact ASCII policy
in `src/lib.rs::PRIVATE_POLICY`. V8 retains operation-scoped Read/Write programs
and bounded JSON input, and gates empty-204 Write completion on peer EOF.
The separate native configuration digest includes its
artifact compatibility, target capacity and result limits. Client, compiler,
complete registration, selected artifact and diagnostics agree. Historical private
policies cannot substitute in either direction; identity tests recover original
owners/requests and verify no connection. The parser, response ingress, connect
failure classification and disposal policies retain their prior qualified scope.
No old artifact is relabeled as the native HTTP program.

The wire counter and per-callback credit never refill by parser phase. The fixed
8192-byte raw head, 4096-byte read scratch and fallibly reserved 4096-byte body
may overlap Hyper read storage, raw trailer storage, parsed maps and body-channel
backing. `max_buf_size` controls a reserve strategy; it is not an allocator
capacity ceiling. Stock source inspection establishes adaptive read reservation,
a growable trailer BytesMut starting at 64 bytes, raw-byte comparison **after**
storing the rejecting byte, a temporary httparse array and normalized HeaderMap,
and a single queued body frame. Bytes slices can retain their original backing.

`tests/allocation_trace.rs` measures the actual public production path, including
simultaneous maximum input, unfinished trailer rejection and cancellation. It
records allocation sizes/counts, retained/peak/largest requests, callback
observations and complete release after the application runtime is destroyed.
See the versioned [evidence](../../../docs/evidence/WP-600-http-consumer-property-read.toml)
for measured values. These observations distinguish received bytes, physical
backing, temporary overlap and engine-account funding. They do not certify
universal SDK memory/native-stack/disposal bounds or recovery from process OOM.

The complete call owns the connection future/driver until local retirement.
Return, cancellation and drop destroy them before terminal settlement/release,
without a network wait, join or unowned tail. Remote execution is unknown.
SDK-local disposal may allocate; engine cancel/status and result-taking allocate
zero on the measured paths. Tokio can retain shared deregistration-queue backing
until reactor progress or destruction, after the original socket has closed.

Core still validates ordinary untrusted outputs and installs existing seals.
Engine funding covers complete Arc body/control/alignment, both declared 16-byte
media/coding capacities and normal/late overlap. Original parent accounts and
prebacked cancellation/cleanup remain live until release. The concrete call
reports its measured Layout; protocol backing remains Binding-private. The
16384-byte transient declaration is a configured allowance, **not** a measured
maximum native stack or transitive SDK heap certificate.

## Application deployment

Run the application example against a typed local TD file:

```sh
cargo run --locked -p clinkz-wot-binding-http --example read_property -- TD.json PROPERTY
```

It creates an application-owned Tokio `=1.52.3` current-thread runtime with
`enable_io().enable_time()`, a monotonic Servient clock, checked owned policy and
original parent accounts. It consumes the Thing, destroys the source, manually
polls/takes the result, shuts down and checks original-account reclamation. The
command exits after one read; Ctrl-C stops the process. Loading the file is
application input, not bounded remote TD ingestion.

Enter a live I/O-enabled runtime when starting reads and keep it driving through
their lifetime. Merely entering a current-thread runtime does not run its reactor;
use `Runtime::block_on`. Time is used for application polling sleeps; the binding
uses the supplied Servient clock/deadline, without a Tokio timer or spawned task.
Applications must drive wakes and clock progress for silent peers/deadlines.

No entered runtime rejects before raw acceptance. Tokio has no public nonpanicking
I/O-driver query, so acceptance probes registration of an unconnected nonblocking
socket and immediately drops it. No bind/connect/read/write occurs. A disabled
I/O panic is caught under unwind, although the panic hook can run. Deployment
owns runtime lifetime, stacks, allocator/OS buffers, shared reactor allocations,
FD/process availability and the probe's extra temporary FD. Runtime destruction
after acceptance, panic=abort, process OOM/abort and OS termination have no
in-process reporting guarantee.

## Validation and remaining obligations

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --all-targets -- --test-threads=1 --nocapture
cargo clippy --locked --offline --no-deps -p clinkz-wot-binding-http --all-targets -- -D warnings
```

Python 3 and loopback networking are needed. Independent raw TCP and Python
standard-library peers exercise public consume/select/read, two distinct targets,
100 alternating reads across fixed/chunked/trailer/close framing, exact/overflow
and unfinished inputs, declaration interoperability, cancellation/drop/deadline,
completion orders, drain/root loss, saturation/reuse and original-account release.
On Linux, the 100-read test also checks that descriptors return to baseline after
each peer observes retirement. Supported body-reservation refusal preserves
607/DeploymentFailure; decoded overflow preserves 608/BindingPolicy, followed by
a successful read on the same Runtime. Parser-boundary regressions exercise the
actual configured production path in `tests/real_peer.rs`.

`tests/lifecycle_races.rs` controls the clock and final peer byte/EOF across
fixed, chunked, close-delimited and invalid-trailer responses. All 16 orders
exercise zero/short progress, exhausted parent capacity, retained unread results,
first-cause cancellation/expiry, retirement and reuse. This transport destroys
its future on cancellation and returns no late SDK value; buffered valid or
invalid input cannot enter a second callback after retirement. Shared Core
late-result seal tests retain their separate scope.

On Linux, `tests/fd_pressure.rs` runs a disposable child with `RLIMIT_NOFILE=128`.
It exercises preacceptance input recovery, postacceptance socket creation failure
(604/DeploymentFailure), silent-peer cancellation while descriptors are exhausted,
connection refusal and successful reuse. It changes no parent process limit.
The allocation fixture also measures and refuses the fallible raw-call box
and verifies request recovery, occupancy release and retry. SDK infallible
OOM remains a Host process-failure assumption.

PR #200 independently accepted the v4 production slice and PR #201 independently
accepted the v5 diagnostic/pressure evidence, both as partial progress.
`tests/support/runtime_cases.rs` adds real pending-connect reset and stopped-driver
registration refusal/recovery, plus cancellation and full retirement of one
public Runtime while another pending HTTP owner and an application socket keep
using the same reactor. These cases reuse the existing allocation observer.

The evidence now records representative native stack from GDB on actual debug
and optimized production paths, and synchronous disposal time/allocation/free
observations. Actual native frames are substantially larger than the heap
continuation; neither its size nor the transient declaration is a thread-stack
ceiling. Samples qualify those executed paths and leave stack provisioning and
stronger availability/fit to the deployment. Universal SDK ceilings are not a
Host requirement.

The std-only Servient integration target uses its matching feature gate.
Historical validation and the independently accepted Consumer completion retain
their exact revisions in the evidence record. Current tests do not grant gate,
package or release acceptance. Producer completion, HTTPS/trust/security and
embedded/hardware qualification have separate boundaries; HTTPS/security remains
a release requirement.

## Producer semantic preparation

`HttpProducerConfig::for_loopback` checks an application-supplied nonzero
loopback address without I/O. With `prepare`, `compile_property_read` lowers
Planning's completely covered `ResolvedProducerInput` into an owned
`HttpProducerProgram`: GET, endpoint NoSec, JSON, primary status 200 and the
exact configured authority/path/query. Default and local aliases, relative
href, optional source id and empty Property keys are supported. Unknown scoped
predicates, Profiles and unsupported security/response instructions refuse.
The target limit of 64 bytes belongs to this HTTP adapter.

Preparation supports only a type assertion (null, boolean, number, integer,
string, object or array), or no type assertion. It refuses nested/composed
schemas, const/enum, required, numeric bounds/multiples, lengths, patterns,
formats, content assertions and unknown schema predicates. A schema accepted
by TD tooling is not necessarily executable by this service. Metadata, default,
unit and read/write annotations do not restrict returned values. This replaces
the old extension-only check, which could accept assertions without validating
them at runtime.

The native program retains no source, arena, logical plan, handler or resource.
It also carries no installation or publication authority. `producer_execution`
supplies a runtime-only executor; optional `try_producer_registration` pairs the
actual executor with pure native preparation. The completed registered handler,
program and local instance enter `ProducerInstallationRuntime`, which funds the
retained owner before a real nonblocking listener is installed. No Tokio runtime
or dummy Consumer is needed. Occupied-port/FD failure, cancellation and root
loss retain/dispose the same owner and preserve original accounts.

The public source-free `ServientBuilder::new(limits, shared_accounts).produce`
consumes that completed preparation and unique authority. `begin_expose(5000)`
requests publication after native readiness; caller-driven `step` then accepts
under a borrowed permit, invokes the registered `ReadPropertyHandler`, Core-seals
its result and lends the sealed response back to the original native owner.
`InstalledClosed` alone still issues no permit. The old `produce_td` and
`ExposedThingHandle` staged Host facade has been removed.

The joined v6 application is `examples/serve_property.rs` (see the joined path below):

```sh
cargo run --locked --offline -p clinkz-wot-binding-http --example serve_property -- 8080 30
# From another terminal while it is Serving:
curl --http1.1 -H 'Connection: close' http://127.0.0.1:8080/value
```

It returns `21.5` from a real handler and retires after 30 seconds. This adapter
uses std nonblocking TCP and Hyper 1.12.0 without a Producer Tokio reactor or
detached task. The example paces its manual polling; wakeups are not OS readiness
notifications. One root serves one logical route and one connection at a time.
Multiple roots can use the same `Arc<HostResourceAccounts>` as public Consumer
roots. Generic installations may lease shared native infrastructure; HTTP's
exclusive listener is a concrete current limit, not a generic cardinality rule.
Address reuse permits restart after closed connections enter TIME_WAIT; port
sharing is disabled and an occupied listener still refuses installation.

Requests require HTTP/1.1 GET, exact path/query and Host, bodyless framing, at
most 16 fields and 8192 raw head bytes inclusive. A native poll reads/writes at
most 4096 bytes. Keepalive, pipelining, Expect and upgrade are refused. Valid
application results use JSON <=4096 bytes; invalid type, shape, media, size or syntax
returns JSON `null` with status 500. General TD data-schema validation remains
unimplemented and unsupported assertions refuse preparation. The actual captured
executor checks the prepared type and JSON encoding before success sealing.
Integer classification uses exact decimal/exponent meaning, including `1.0`
and `100e-2`, without floating-point rounding. Strings and keys require scalar
Unicode, including paired surrogate escapes. The caller-clock timeout spans
unfinished heads, dispatch and pending replies. A timeout or reply failure closes the instance;
an application can prepare a fresh generation after retirement.

Native socket/parser/future/encoded-reply allocation is Binding-private under
the fresh Producer policy identity, separately from reserved Engine owner and
response backing. Shared JSON key/string decoding invoked by the Engine is part
of the installation declaration. Constrained strings and objects reserve 13,312
temporary bytes: up to three 4096-byte bodies for decoder buffer growth/owned
strings, plus the existing 1024-byte wrapper/error allowance. A single decoder
allocation may request up to 8192 bytes, so contiguous-allocation admission also
covers that bound. Other scalar schemas retain the 1024-byte allowance.
These bounds cover the locked slice decoder; they are not a native stack ceiling.
Tests report actual allocation/FD disposal observations; these
are neither universal SDK maxima nor a strict stack/allocator guarantee. Host
process OOM remains a deployment assumption. Successful local writes do not
prove that a peer observed the response. HTTPS, authentication, automatic
scheduling, asynchronous handlers and shared HTTP listener matching remain
future work. The usable bounded service is an implementation candidate requiring
independent architecture and WP-600 completion acceptance.

The [Producer completion candidate](../../../docs/evidence/WP-600-http-producer-property-read.toml)
maps all six registered obligations to their actual tests and observations.
The public loop covers two successive targets/results on the same listening
address (default GET and aliased explicit GET/status, relative hrefs), 100 reads
each after source destruction, and cancellation/deadline/call/client/Producer
root loss before and after dispatch followed by a fresh successful exposure.
Raw-peer tests separately observe disposal after the first native write of a
4096-byte response, including its already observable prefix. Repeated raw reads
check native retained allocations as well as Engine accounts; exact/one-over
heads also have allocator/descriptor observations. The typed single-registration
root and instance/correlation-only request are provisional narrow APIs, not a
general multi-route/multi-operation Producer SPI or proof of shared matching.

This adapter remains separate from `servient/finite`'s raw shared logical
Producer. Neither its older single-provider Serving marker nor raw activation
qualifies a final TD under v6. Complete actual participation/behavior/support,
required native readiness and the final Servient publication commit remain
necessary; no v6 exposure API is introduced by the schema prerequisite.

## HTTP through the shared finite owner

`HttpFiniteConfig::try_new` consumes an owned `HttpProducerProgram`, the matching
listen configuration, candidate instance/key and timeout. `HttpFiniteBinding`
then implements Core's native finite facet directly. It retains no handler,
logical authority, compiler or TD. The old `HttpProducerInstallation` is not
under this path. Both paths reuse the HTTP parser/socket/reply mechanics.

Use `RawFiniteRuntime::try_new` with `FiniteValidation::Required` to require
the captured mapping's actual input/output JSON/type validator. The associated `RawProducerServient`
owns the sole logical behavior and can drive independently typed native owners.
Pass caller-clock time to progress. Native request deadlines do not reset with
polls; dispatch prepays the declared complete result traversal before the codec.
The original result/behavior remain owned through cancellation, partial writes
and settlement acknowledgement. Closing one HTTP owner leaves siblings usable.

Raw mode still enforces HTTP's native encoding envelope. Unencodable successful
output refuses before a response write and retains its original value/effect
through retirement as NotDelivered. The fixed 500/null mapping delivers an
original error, including a Required validation failure; it does not replace an
unencodable success and report that value delivered. Clock violations after
native admission use InternalInvariant with the active Binding, Handler or Cleanup
phase, while initial timing refusal remains Validation/Admission.

`activate_raw` opens only local acceptance, including with Required checking.
It does **not** publish a qualified TD. This integration has caller-provisioned
bounded finite slots, not the older root's aggregate Host account admission.
Complete final-TD/actual-behavior/provider/service qualification, aggregate
admission, final endpoint/dependency checks and the local publication commit
belong to the joined facade below. The legacy funded installer remains a
separate restricted implementation with its own historical regression scope.

Run the executable shared integration (raw independent peers and public Consumer):

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --test finite_producer
```

Run the preparation regressions with:

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --test producer_serving -- --test-threads=1 --nocapture
cargo test --locked --offline -p clinkz-wot-binding-http --test producer_preparation
cargo test --locked --offline -p clinkz-wot-td --features validated-thing --test producer_lending
cargo test --locked --offline -p clinkz-wot-binding-http --test producer_installation -- --test-threads=1
cargo test --locked --offline -p clinkz-wot-servient --test producer_installation -- --test-threads=1
```


## Joined v6 Producer

`HttpFiniteProducer` startup-registers the actual direct finite preparation
provider with `producer::ProducerServient`. Its `prepare` accepts the complete
retained TD, exact behavior key and unique authority. It rejects the entire TD
if any retained Form/operation, schema, security or lifecycle promise is outside
the supported single-read scope. It funds the staged logical/native/result/
cleanup owners before opening a socket. This uses no legacy installer API.

The first funded step installs native participation and returns InstalledClosed.
Until publication the Binding accepts/parses no application stream; prepublication
traffic is buffered only in the bounded OS listen backlog (requested at 128),
with OS wait/refusal at saturation. A following funded step reinterprets the
entire final TD, checks the exact mapping/provider/services and actual local
endpoint, then commits description, behavior and current logical authority with
no allocation or fallible native call in that local commit. `description(key)`
returns the exact final JSON only while that incarnation is current.

This first scope uses explicit nonzero loopback endpoints, one Property/Form,
effective `readproperty`, synchronous registered behavior, endpoint NoSec,
JSON/200 and the bounded prepared schemas described above. It supports anonymous TDs,
empty Property keys, default/aliased GET and relative hrefs. It does not rewrite
port-0 endpoints or silently discard alternatives. The current public Consumer
still needs an id/nonempty key; its joined test supplies those in the original
published TD rather than modifying the description after publication.

After publication only Servient dispatches. The native Binding owns request
matching/correlation/reply/physical release; the shared logical owner checks
current identity and policy, validates output with the actual captured service,
and retains original result/effect/delivery until settlement acknowledgement.
Cancellation/lost exposure revokes the same permission cell as the registry.
Close/failed cleanup retains owners; `retire` requires Closed and an acknowledged
settlement. The caller drives progress; Drop guarantees finite local disposal,
not remote rollback or completed drain. Replacement preserves old accepted work.

The joined facade is Host/std/manual and contains optional preparation. The
separate finite execution crate still links only Core/Foundation. Its generic
source-free publication primitive relies on the facade's qualification; raw
activation remains raw. Multiple independently prepared/installed participants
can share one logical Thing. This restricted facade does not exercise shared HTTP listeners; TLS/authentication, Events,
embedded devices and resource/performance release qualification remain unproved. Original-bank
reservations and a conservative bounded-input decoding allowance are distinct
from measured OS/SDK or total allocator maxima.

## Composed Property Read/Write

The open `ConsumerRegistry` prepares separate `ReadProperty` and `WriteProperty`
definitions from each effective TD Form operation.
`association.write_property(&calls, name, payload, deadline, now, &mut budget)`
takes the Property value as a `Payload`, validates it with the captured actual
data service and authorizes the original Consumer/operation before native I/O.
Its recoverable local rejection returns the original payload. Execution retains
no source TD or preparation registry. Read and Write can coexist on one Property;
neither consumes or replaces the other's definition.

The Producer uses the existing whole-Thing `prepare_thing` and
`ComposedProducerExposure` publication path. `HttpFiniteProducer` returns one
owned `HttpProducerProgram` per operation/Form, including the Property value
schema. `HttpFiniteConfig::try_from_mappings` shares one listener across up to
32 native Property mappings and rejects duplicate method/path pairs before
setup; `try_new` also permits a single mapping. Each selected definition must
have exactly one actual participant, required validation service and behavior.
No advertised operation or Form is silently discarded. Required input validation
precedes fresh application authorization and handler entry; malformed or denied
input cannot call the write behavior.

The supported mapping is Host/std/manual HTTP/1.1 to nonzero loopback Producer
addresses, cleartext IP Consumer targets, endpoint NoSec and the bounded prepared JSON schema subset
values up to 4096 bytes. The existing unsupported schema/security assertions
refuse preparation. Default methods follow [TD 1.1 section 8.3.1](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#http-binding-assertions):
GET for Read and PUT for Write. Explicit single-operation methods must match on
the Producer; Write currently supports PUT on both roles. Explicit multiple-op
Forms cannot supply one `htv:methodName`. Omitted/default and explicit Read/Write
operation sets are exercised on the same endpoint. Property `readOnly` and
`writeOnly` declarations constrain eligibility even with contradictory explicit
Forms. Unsupported alternatives may remain visible in a Consumer support report;
the complete retained Producer TD must qualify.

One native exchange occupies the listener. Requests have an exact path and Host,
at most 16 headers/8192 header bytes and 4096 read/write bytes per native poll.
PUT requires exactly one nonzero Content-Length (<=4096) and exactly one
`Content-Type: application/json`; chunked input, content coding, trailers,
Expect, upgrades and pipelining are refused. Cleartext NoSec rejects credential
headers; the HTTPS composed profile accepts only its selected bounded credential headers.
Native parsing
collects the entire bounded input before offering a candidate. The shared Engine
validator then checks its prepared Property schema before effects.

Write handlers report empty `InteractionStatus::Ok` completion. HTTP maps this to
an empty 204; handler/validation errors retain the existing 500/null mapping.
The Consumer accepts only an empty primary 204 without Content-Length or
Transfer-Encoding, with no bytes after the header through peer EOF on the
existing `Connection: close` exchange. Hyper regards a 204 as complete at its
header terminator; this stricter Binding guard keeps the existing bounded raw
head private until EOF, then supplies it to Hyper under the same 4096-byte
callback allowance. Neither coalesced bytes beyond the head buffer nor bytes
arriving on a later read can bypass the check. A peer that leaves the response
direction open keeps the original call/socket/funding pending; cancellation,
deadline, revocation and owner loss can still retire it without waiting for EOF.
This is the chosen native completion boundary alongside HTTP's
[connection-close behavior](https://www.rfc-editor.org/rfc/rfc9112.html#section-9.6).
Read-shaped 200, asynchronous 202, truncated/malformed
responses and unexpected data do not establish Write success. This strict
response subset is a concrete Binding contract, not a universal WoT requirement
or an HTTP status switch in Core. It neither performs hidden Read nor confirms
the physical state independently of the application's reported completion.

Constructor refusal, native exchange, validated application completion and
remote effect remain separate. `ConsumerSettlement::native_outcome` retains
late results; `result` also requires original authority and fresh policy.
Validated Property Write completion reports `ConsumerEffect::Completed` and
`FiniteEffect::Completed`. Consumer `Unknown` after attempted progress without
that evidence does not imply rollback on error or cancellation, even for HTTP
401/403/500. Producer pre-effect refusal is independently `NotInvoked`; an error
after behavior entry remains `Unknown`. Producer result and `FiniteDelivery`
retain separate facts. No owner automatically retries a write. Retained Consumer
settlements pin original result/service funding independently of the native
exchange lease released at connection/driver retirement. Engine call slots remain
occupied until settlement handoff or funded abandoned-call disposal. An unread
Producer settlement keeps that runtime unavailable for the next receive until
acknowledgement. Cancellation, replacement and root loss preserve original accepted
work and funding through their required retirement and disposal boundaries.

`property_write` proves actual Consumer-to-Producer state changes and subsequent
Read, independent definitions, readOnly/writeOnly and explicit operation
applicability, full-description refusal, untrusted input, ambiguous native replies,
handler error after effects, cancellation/timeout, replacement, lost services,
unread capacity and final socket/account release. The empty-Write regressions
additionally exercise 4096/8192-byte header boundaries,
coalesced/fragmented trailing data, valid peer half-close and funded cancellation
while waiting for EOF. The independently Cargo-linked
Consumer fixture executes Write over real datagrams using its own `WRITTEN`
completion marker and opaque Core status 0, including retained late success.
It is test-only and adds no production protocol.

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --test property_write
cargo test --locked --offline --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml
```

The restricted legacy single-read Consumer/Producer adapters retain their scope.
This increment makes no general DataSchema/security, HTTPS, collection,
constrained profile, automatic-progress or production qualification claim.

## Composed HTTP Producer Actions

`HttpActionProducer` selects independent source-free definitions for
`invokeaction` (POST), `queryaction` (GET) and `cancelaction` (DELETE). This
concrete participant requires all three Forms and registered behaviors: it has
no cross-Binding correlation service and refuses an incomplete native set
before publication. This is an HTTP participant limit, not a Planning or
Servient operation-family rule. The
HTTP mapping uses a nonzero loopback address, JSON, endpoint NoSec, a fixed
invoke path and control paths ending in `{?id}` with an Action
`uriVariables.id` string schema. POST returns `202` and a JSON
`{"id":"<epoch>-<invocation>","status":"accepted"}` with the selected status
URL in `Location`. GET returns `running`,
`completed` (with validated `output`) or `failed`; DELETE requests cooperative
cancellation and returns `cancelling` or `terminal`. These wire envelopes and
correlation conventions are this Binding's documented HTTP mapping, **not** a
new Engine semantic or a claim that a W3C Note mandates this representation.
A displaced lease's status URL cannot address a successor's invocation. Output
is advertised only when a query Form can expose it.
Invalid requests/input receive 400, policy refusal 403, missing or foreign
correlation 404, stale logical authority 410 and exhausted admission/work
capacity 503; unexpected failures receive 500. None of these statuses proves
that an already accepted Action was undone.

All final TD Forms, effective operations, resolved security and complete supported
Action input/output schemas are qualified through the existing whole-Thing selector,
closed installation, managed Host resource handoff and local Servient publication.
Unsupported DataSchema constraints, executable extension assertions, synchronous
completion promises, alternate media/codings/security, method/response mismatches,
other URI templates, missing control mappings and multi-operation Forms refuse before publication. Pure
definitions hold no socket or logical permission. `HttpActionRuntime` is one
concrete native participant per Action; other Bindings may share a listener or
use distinct control routes/Bindings. This does not constrain general Planning
or Servient cardinality. An HTTP Property Read participant can join the *same*
Thing with separate listeners and one logical behavior authority.

Native HTTP/1.1 I/O is caller-driven, one connection at a time per participant,
with 16 headers, 8192 head bytes, 4096 JSON input/output bytes and a 10-second
per-exchange deadline. Native ingress has no application handler. Servient
captures the source-independent validator at publication and validates structured
JSON constraints before behavior; each admitted continuation retains that original output service.
An explicitly raw Servient registration remains unvalidated, but a published
Action cannot bypass its captured validator through `RawActionRequest`.

Structured Action validators declare an extra 4096 × 80 `HandlerSteps` per
input/output callback, matching the finite structured JSON envelope. The logical
owner captures this allowance at publication. Invoke prepays it twice for input
and possible immediate output; normal continuation progress prepays it once for
output before polling application work. Short credits prevent permission/dispatch
or keep continuing work unpolled. Changed validator declarations fence validation
under the original allowance; native controls and local close retain their
independent ownership and work budgets.
`progress_actions` fairly polls retained work after the
initiating reply; completion stays queryable until explicit `acknowledge` or
application-selected terminal retention expiry (`reap_completed`, five seconds
in the example). Pending work is never expired as a completed result.
`begin_close` revokes ingress and releases native sockets but cannot release
unread work. `HttpActionDelivery` reports local write progress/failure in
`cause` and separately retains any original Servient refusal in
`request_error` for diagnostics, even if the write fails. Local completion does
not prove remote receipt and partial failure cannot erase possible application
effects. The caller provisions the Action-store baseline
independently of native participation backing. NoSec/loopback is not TLS or a
remotely secure bearer-token service.

Run a live example, then use curl from a second terminal:

```sh
cargo run --locked --offline -p clinkz-wot-binding-http --example serve_action -- 8081 30
curl -i -X POST -H 'Content-Type: application/json' --data 12 http://127.0.0.1:8081/run
# GET the returned Location, or DELETE /stop?id=<id from POST>.
```

`tests/action_producer.rs` uses real TCP peers for mixed-capability publication,
invalid input/output, independent continuation, status/cancel, policy refusal,
low-budget and record capacity, replacement, stale correlation, unread
settlement, partial input, occupied ports and release.
UDP Action tests remain test-only evidence for shared native infrastructure.
HTTPS, broader DataSchema/security services, other HTTP Action response styles,
automatic progress and remote effect certainty remain unsupported.


The `read_property` example now uses open startup registration through
`planning::open_consumer::ConsumerRegistry` and the source-free
`clinkz_wot_servient_finite::consumer::{ConsumerAssociation, ConsumerCalls}`.
The association captures the original Consumer and bindings; separately
constructed capacity is supplied to `association.read_property(&calls, name,
deadline, now, &mut budget)`. `try_consumer_registration` returns
the actual compiler/executor/configuration and JSON/type plus cleartext-IP access
services. Complete type-only schema and NoSec support is checked in preparation;
codec/type validation runs on normal and late output. Unsupported schema/security
meaning is reported unsupported instead of trusting result-shape sealing alone.
The lower-level `try_inline_consumer_registration` exists for the retained
restricted aggregate regression/measurement path. HTTP SSE is not implemented.


## TD-driven continuing Consumer Actions

`ConsumerActionRegistry::prepare_action` selects invoke and every described
query/cancel operation independently. Register `HttpActionConsumer` with
`HttpConsumerConfig::consumer_services()` (or actual application services) and an
explicit 4096-unit `PlanCompileWorkUnitsPerStepMax` allowance. This is the same
bounded pure native callback policy as the existing typed MQTT facet. The shared
HTTP lowering preserves the qualified Producer mapping above: bounded structured JSON,
endpoint NoSec, cleartext IP literals, POST invoke and independent GET/DELETE
control Forms ending in `{?id}`, with a string `uriVariables.id`. Absent or false
`synchronous` uses this continuing facet; the existing explicitly synchronous
finite Consumer remains separate. A missing described control provider refuses
preparation; absent operations remain unavailable. Different actual registrations
may select controls on this native listener; another listener/continuation format
is unsupported by this concrete mapping.

The source-free `consumer_action::ConsumerActionRuntime` owns each invocation
independently of finite native exchanges. `invoke_action` returns a local handle;
`progress` exposes Invoking, Pending, Querying, Cancelling, local Closing and terminal states.
`query` and `request_cancel` are explicit described operations. POST 202 with the
bounded native id means accepted work, never a terminal result. GET `running`
remains pending; `completed` supplies output for the original captured validator;
`failed` is a remote execution failure. DELETE `cancelling` acknowledges only a
request; `terminal` acknowledges no result. A later query must obtain completion.
The default access service checks the actual expanded control target before I/O.
No Location header is followed, no URL is invented by the Engine and no failure
or cancellation replays the Action.

The native Consumer exchanges use one fresh closing TCP connection, with 16
headers, an inclusive 8192-byte head, one Content-Length, an 8192-byte response
envelope and 4096-byte application data. At most 16384 response bytes are retained;
one native poll shares 4096 read/write bytes. Complete framing is checked through
peer EOF, including fragmented trailing bytes. Chunked/close-delimited bodies,
content codings, trailers, upgrades, informational responses, malformed/duplicate
response fields and unexpected status/role combinations refuse this mapping.
Borrowed JSON RawValue preserves output bytes without constructing a JSON tree;
the actual captured data service validates them. The HTTP policy fingerprint is
v13 (the continuing Action mapping was introduced in v9). There is one active exchange per actual registration; the Engine independently
bounds continuing invocations and unread terminal slots.

`cancel_local`, handle abandonment, deadline, policy/service loss, root close and
instance replacement retire local work with its first cause. They do not send a
hidden DELETE or prove remote rollback. A lost DELETE reply retains Uncertain
remote cancellation; Requested still permits eventual application completion.
Settlement retains accepted status, original native result, validation and cleanup
errors, remote cancellation and effect uncertainty separately. Pre-effect refusal
returns original input. Pending/error cleanup and unread settlement keep original
funding; Drop disposes native owners finitely before refunding it. The API admits
no detached job, native pooling, reconnect or cleanup transfer.

`Completed` means a validated native result is ready; unread success remains
revocable. `take_settlement(now, budget)` is the application handoff and performs
fresh paid time/authority/service/access checks even after recent progress.
Zero or short handoff credit keeps the original result and invocation slot.
Cancellation, root/instance loss, expiry or policy loss before handoff yields a
fenced settlement with the original native output still available as evidence.
Once successful handoff has occurred, later revocation does not retract that value.
If a native poll fails but retirement returns a late completion, `native_error()`
keeps the poll error while `native_result()` keeps the actual late output.
The captured validator still checks it; neither a valid late result nor a later
cancellation can erase the original failure or silently retry the Action.

Run `serve_action`, save its printed qualified TD to `TD.json`, then run:

```sh
cargo run --locked --offline -p clinkz-wot-binding-http --example invoke_action -- TD.json run 12
cargo run --locked --offline -p clinkz-wot-binding-http --example invoke_action -- TD.json run 12 --cancel
cargo test --locked --offline -p clinkz-wot-binding-http --test action_consumer --test action_producer
cargo test --locked --offline --manifest-path tools/architecture-fixtures/open-consumer/Cargo.toml --test action
```

The HTTP integration joins real Engine roles and covers pending/completed and
cancellation paths, unsupported mappings, validation, capacity, replacements,
malformed native replies, abandonment and final socket/account release. The
independent downstream fixture executes real datagrams using different invoke and
control Binding implementations, including late invalid completion during blocked
and failed cleanup. These tests do not establish general Action interoperability,
HTTPS, broader DataSchema/security, constrained execution, automatic progress or
remote effect certainty.

## Thing-root ReadAllProperties

The open Consumer and completely qualified composed Producer support TD 1.1
`readallproperties` on an **actual Thing-root Form**. The Consumer API is
`association.read_all_properties(&calls, deadline, now, &mut budget)`.
The root definition
retains the selected Form, actual registration/configuration, data/access services
and an owned schema for every described readable Property. A root mapping can be
selected independently of individual Property Form support. Producer publication
still requires every final retained Form/effective operation, a distinct registered
`AffordanceTarget::Thing` behavior and complete native participant coverage.

The mapping uses one bodyless HTTP GET and a primary 200 `application/json`
response, with the existing bounded read framing policy. GET can be defaulted or
explicit; other explicit methods, response statuses/media, subprotocols, coding,
required URI variables, additional response mappings and non-endpoint-NoSec
security refuse preparation. Cleartext IP Consumer targets and the existing
nonzero loopback Producer endpoints retain their respective limits. Concrete
Bindings that lack native collection support remain unsupported.

The response is one object containing every readable (non-writeOnly) Property
exactly once, keyed by its TD name. ReadOnly Properties are included. Every value
is checked against that Property's complete supported prepared schema, including
exact mathematical integer validation. Missing, duplicate (including escaped-name
duplicates), unknown and writeOnly members, invalid values and malformed payloads
refuse successful settlement. Empty readable sets require `{}`. Unsupported
assertions on any included member refuse preparation, rather than accepting only
an object type. The captured service must explicitly support collection validation;
its scalar validation support alone is insufficient. The shared JSON codec
validates original member bytes with borrowed RawValue and no JSON value tree.

The semantic descriptor bounds are 32 readable members, 256 UTF-8 bytes per name
and 4096 total name bytes. These are ceilings, not a promise that every such TD
fits: the existing resolved-loan limits (128 nodes, 32 scopes and bounded context
work) and admission/profile limits also apply, including schemas of described
writeOnly Properties in the root loan. The HTTP response remains at most 4096
bytes. At most 32 explicit native root/Property mappings share a listener; native
method/path collisions refuse, even for different logical GET operations.
Consumer data policy funds up to `4096 * 40 + 16 * 2` validation work units;
Producer validation captures and prepays an additional `4096 * 40` units beyond
the finite owner's base body/media traversal. Member/name backing, bounded
destruction, captured services and callback scratch retain their original accounts.
Consumer native policy fingerprint is v13.

The Engine enters the root application behavior once. It does not call or
implicitly aggregate individual Property handlers. Application/Binding code owns
native aggregation and any stronger consistency guarantee. This mapping promises
no simultaneous sampling or atomic snapshot. A root application aggregation
failure returns an error for the group; no HTTP 206/207 or per-member partial
failure envelope is supported as successful collection completion. Cancellation,
replacement, lost services and close keep original root attribution, first cause,
late native/application evidence and physical release owners. No retry or
individual-read fallback is implied by failure.

WriteMultipleProperties, collection
observation, query-all Actions and subscribe/unsubscribe-all Events are
unimplemented. They require their own selected-member/data roles, native support
and appropriate lifecycle; this finite read does not qualify them. HTTP JSON,
type-only schema, NoSec and manual Host evidence establishes no general schema,
security, HTTPS, constrained-profile or production collection qualification.

[Real execution tests](tests/read_all_properties.rs) join an Engine Consumer to
an Engine Producer for multiple Properties in one request, preserve individual
operations on the same listener, exercise independent adversarial peers, complete
qualification, service loss, cancellation, replacement, saturation, unpaid
validation and final socket/account release. Existing Property/Action/Event
regressions retain their own claims.

Run the real examples in separate terminals:

```sh
cargo run -p clinkz-wot-binding-http --example serve_properties -- 8082 30 > /tmp/collection-td.json
cargo run -p clinkz-wot-binding-http --example read_all_properties -- /tmp/collection-td.json
cargo run -p clinkz-wot-binding-http --example read_multiple_properties -- /tmp/collection-td.json level
cargo run -p clinkz-wot-binding-http --example read_multiple_properties -- /tmp/collection-td.json on
```

The Consumer prints the validated combined value. The Producer publishes the
complete final TD before writing it and retires its listener/accounts on exit.

## Thing-root ReadMultipleProperties

`association.read_multiple_properties(&calls, &["level"], deadline, now, &mut budget)`
selects members for one actual Thing-root `readmultipleproperties` interaction.
Preparation retains all eligible readable names/schemas, actual services and the
selected root Form. It performs no request encoding. Each call checks exact names
against that retained set and owns an inline bitmap into its original schema.
Caller storage and the source TD can be destroyed before execution. The bitmap
remains with the original call through cancellation, unread/late results and
settlement. The schema is not modified by any call.
Preparation also qualifies the captured generic Array input validator that
invocation calls on the encoded name array; selection-specific codec support
alone does not qualify this dependency. Required support and policy are checked
again after authorization, including final admission and immediate/unread result
handoff. A change fences success while retaining the original result and funding.

The supported Form explicitly specifies `"htv:methodName":"POST"`, with
`application/json` request and primary 200 JSON response. As defined by
[TD 1.1 §5.3.1.1](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#thing),
the default request is a JSON array of exact Property names, such as `["level"]`.
The response is a JSON object containing exactly those members, such as
`{"level":7}`. A subsequent call can send `["on"]` against the same prepared TD
and must receive only `{"on":true}`. `[]` requires `{}`. Name order has no semantic
meaning; the encoder uses retained schema order. Names are never normalized or
silently deduplicated. Undescribed/writeOnly names refuse before native effects.

TD HTTP defaults ReadMultipleProperties to GET when no method is specified. This
slice declines that mapping: a GET request body has no generally defined
semantics. It also declines URI templates/uriVariables, including TD's illustrated
GET query mapping. No fixed query parameter or implicit member-list-to-variable
conversion is introduced. Supporting another request mapping requires actual
semantic/Binding support. Single-operation explicit POST is qualified as part of
the actual Form; a multi-operation Form with one explicit method is refused.

The native Producer matches POST/path/authority, accepts one bounded fixed-length
JSON body, validates its member set before authorization/dispatch, and calls one
registered root behavior. The actual `FiniteValidator` receives the original
input again for output validation. Every requested member is required exactly
once with its own retained schema; an extra readable member is as invalid as an
unknown/writeOnly member. Escaped duplicates, missing/invalid values and malformed
responses refuse whole-result success. HTTP 206/207, truncated bodies, application
aggregation failures and per-member failure envelopes are errors, with original
group attribution and uncertain post-handler effect. No read fan-out, retry,
atomic snapshot or simultaneous sampling is implied.

The ReadAllProperties descriptor/profile ceilings also apply. Both native input
and output are at most 4096 bytes; escaping expansion can make an otherwise valid
selection unencodable and is refused within the bounded writer. Consumer data
policy separately prepays each encode/input-validation/output-validation callback
at `4096 * 40 + 16 * 2` work units. Complete encoded-input lifetime, callback
scratch and cleanup are reserved before encoding; the captured data policy
explicitly declares complete returned selection-payload backing independently of
encoder scratch and result body limits; refusal drops it before funding
releases. The additional Producer work bound is `4096 * 120` when the participant
contains a selected-read mapping, covering both original-selection decodes and
result member traversal. ReadAll-only participants retain `4096 * 40`. The exact
captured bound is still rechecked immediately before output validation.

[Selected-read execution tests](tests/read_multiple_properties.rs) prove different
subsets from one TD, a separate all-read on the same listener, static/unsupported
selection, independent malformed/partial peers, exact Producer validation,
authorization, lost services during encoding/result validation, prepublication,
cancellation, replacement before/after dispatch, unpaid work, failed sibling
admission, retained results and final account/socket release. Shared fixtures also
rerun the existing schema cleanup regressions for selected reads. This remains
the Host/manual, JSON/type-only, endpoint-NoSec slice; general URI mapping,
security, HTTPS, constrained profiles and production qualification remain open.

## Thing-root WriteAllProperties

`association.write_all_properties(&calls, payload, deadline, now, &mut budget)` selects
one actual Thing-root `writeallproperties` Form and returns the original payload
on local refusal. Preparation retains every writable (non-readOnly) member's
name and supported prepared schema, including writeOnly members. The actual
captured map service validates the complete input; missing, unknown, readOnly,
duplicate (including escaped duplicate), malformed and invalid members refuse
before native effects. Readable output schemas and individual Property Forms
never substitute for that input. Unsupported included assertions decline
preparation. An empty writable set accepts only `{}`.

This Host/manual, endpoint-NoSec, application/json subset uses exactly one HTTP
PUT, default or explicit, with one complete JSON object body. Native authority,
method/path matching, framing, encoding and status interpretation stay in HTTP.
The same listener may carry separate root reads and individual Property mappings;
a method/path collision refuses qualification. URI variables/templates, additional
responses, content coding and unsupported schema/security assertions are declined.
Complete Producer qualification includes every final retained Form, original
writable schema, actual native coverage and registered distinct root behavior.

The Producer validates the whole map and authorizes the group before entering
that behavior once. The behavior must return empty `InteractionStatus::Ok` only
when the **whole group has completed**; HTTP maps it to 204. Accepted, payload
results, partial-member envelopes and handler errors are not whole-group success.
The Consumer's 204 response must have no body, Content-Length or Transfer-Encoding
and must satisfy the existing Write header checks **through peer EOF**, including
trailing bytes in later reads. A valid head without EOF is still pending. 200,
202, 206/207, truncated/extra data, loss or malformed framing leave remote effects
uncertain; the Consumer cannot infer pre-effect refusal from a generic 500.

`FiniteEffect::NotInvoked` proves no application entry. After entry, failure can
follow actual partial or complete application; `Unknown` promises neither rollback
nor per-member results. `FiniteEffect::Completed` records valid whole-group handler
completion independently of `FiniteDelivery`. `ConsumerEffect::Completed` requires
validated native completion; otherwise progressed calls remain `Unknown`.
Original native evidence survives cancellation, replacement, policy loss and
unread settlement even when local success is refused. No implicit replay,
N-WriteProperty fallback, transactional atomicity or physical-state confirmation
is supplied. These are the [Core effect contracts](../../../docs/spec/interaction-core.md).

The existing member descriptor limits apply: 32 members, 256 UTF-8 bytes per
name, 4096 total name bytes; the semantic loan/profile may refuse earlier. Input
is at most 4096 bytes. Consumer policy v13 prepays `4096 * 40 + 16 * 2` validation
work and captured scratch; Producer named-map validation adds `4096 * 40` beyond
base input/output traversal (or the selected-read allowance when sharing that
participant). Original input/schema, captured services, native exchange and
retained cleanup remain funded until their actual disposal/acknowledgement.

The `write_all_properties` integration target exercises both Engine roles and
independent peers, real partial application, readonly/missing/invalid members,
service/work refusal, complete publication, cancellation/replacement, unread
completion and final resource release. The selected-write mapping below reuses
the writable eligibility without weakening all-write completeness. Neither slice
claims production, HTTPS, broader schema/security or constrained qualification.

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --test write_all_properties
# With serve_properties running and its TD in /tmp/collection-td.json:
printf '%s' '{"level":8,"on":false}' > /tmp/collection-values.json
cargo run -p clinkz-wot-binding-http --example write_all_properties -- /tmp/collection-td.json /tmp/collection-values.json
cargo run -p clinkz-wot-binding-http --example read_all_properties -- /tmp/collection-td.json
```


## Thing-root WriteMultipleProperties

`association.write_multiple_properties(&calls, payload, deadline, now, &mut budget)`
selects an actual root `writemultipleproperties` Form. The payload is one JSON
object of selected writable names and their new values, each validated against
its own original Property schema. Omitted writable members are untouched by this
intent; readOnly names, unknown names, decoded duplicates, malformed data and
invalid values refuse the entire input before effects. writeOnly members remain
eligible. The captured service derives the exact selection from the validated
map and retains it independently of the native request. No second list or
selection-array encoding is required. This subset permits an empty map and still
makes one request. Unsupported assertions on any eligible member refuse
preparation, including members a later caller might omit.

The mapping is one HTTP PUT, default or explicit, and strictly empty 204 through
EOF, with the all-write framing, size, work, endpoint-NoSec and schema limits
above. TD 1.1 [Table 32](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#protocol-bindings)
provides the HTTP method default. The JSON object/204 convention follows
[WoT Profiles, 4 November 2025 Working Draft, section 6.2.1.4](https://www.w3.org/TR/2025/WD-wot-profile-20251104/#writemultipleproperties).
That draft supplies mapping guidance, not a Recommendation or a claim of profile
conformance. 200/202/206/207 are unsupported responses for this mapping, not
proof of absence of remote effects. There is no automatic retry, readback,
N-WriteProperty lowering, atomicity, ordering or rollback guarantee.

Complete Producer qualification retains the whole final TD, the original
writable eligibility and one distinct root behavior. That behavior receives the
complete validated input and must authorize the entire selected group before any
member effect. Consumer member policies can inspect the captured schema and
validated set through `authorize_property_write`, with fresh permission at native
admission, subsequent progress and unread handoff.

The protocol-independent finite result can retain bounded per-member reports and
explicit atomicity evidence under the [Core contract](../../../docs/spec/interaction-core.md).
This HTTP mapping provides neither. It refuses an application member report,
even a complete one, rather than silently discarding it or substituting 204.
A normal 204 establishes reported whole-group completion only. Errors after
handler entry can follow partial application; generic HTTP failure cannot convey
which members changed. An independently Cargo-linked test datagram mapping
exercises richer reports through both finite roles, with TD-driven Consumer
preparation and raw Producer registration; it is not production protocol or
TD-qualified Producer interoperability evidence.

[Selected-write tests](tests/write_multiple_properties.rs) cover changing subsets
from one TD, independent schemas, omitted members, member permission, original
payload refusal, unsupported advertisement, real partial application, ambiguous
peers, prepublication/service loss, cancellation/replacement, late/unread
completion, short work, schema cleanup and final account/socket release.

```sh
cargo test --locked --offline -p clinkz-wot-binding-http --test write_multiple_properties
```

## Thing-root Action status query

A root `queryallactions` Form with explicit `htv:methodName: "GET"`, JSON,
endpoint NoSec and a supported primary 200 response selects one native status
query. TD 1.1 has no HTTP method default for this operation. The Consumer uses
`association.query_all_actions(&calls, deadline, now, &mut budget)`;
it needs no prior invocation handles and
retains its own original query authority/services through normal or late result
settlement. Root status responses do not grant local Action control authority.

The payload mapping follows the Action-name/ActionStatus-array representation in
[HTTP Profile Working Draft 2025-11-04 §6.2.2.4](https://www.w3.org/TR/2025/WD-wot-profile-20251104/#queryallactions).
This is draft guidance and a supported subset, not a TD-defined universal status
schema or a claim of HTTP Profile conformance. Existing invocation uses 202 and
`accepted`, with separately described `{?id}` query/cancel Forms. It does not
implement that draft's 201 acceptance or deletion-on-cancel contract.

The result is an object keyed by exact declared Action names, with arrays of
records containing `status` and a unique absolute HTTP(S) `href` of at most 256
bytes. Supported states are `pending`, `running`, `completed` and `failed`.
Completed output must match that Action's retained output role; it is absent
exactly when the role is absent. Other states carry no output. Unknown Actions,
duplicate decoded names/fields/identities, missing identity, unknown status,
foreign fields and invalid output refuse the response while retaining its
original bytes. Missing names mean no visible retained records; empty arrays
are valid. Optional timestamps/problem details and alternative envelopes are
outside this subset and are refused, not discarded. A successful query may
contain failed invocations and does not establish an effect or cancellation.

`HttpActionRuntime` supports several Actions and one root mapping on one
listener. Its entire described Action set and every invoke/query/cancel Form must
qualify against the same actual correlation owner, including schema and service
rechecks before publication. Each Action currently needs one Form for each of
those three operations. The root requires a separately registered
`ActionQueryPolicy`; Action handlers receive fresh `QueryAllActions` permission
checks for their own original invocations. Root denial refuses; member denial
filters a record; original principal mismatch excludes it. NoSec leaves these
application checks in force. No Engine leaf-query loop is used.

The native owner emits every declared name (possibly an empty array), orders each
array by reverse local admission order and preserves the original epoch/id and
Action control path. It does not progress invocation work while reading status.
Current application continuations are `running`; no queued `pending` state is
invented. Completed/failed records remain until explicit acknowledgement or the
configured terminal reaper; running work cannot be pruned. A cancel request can
still be running or complete later. Old tokens cannot target replacement leases
or another Action. A root reply has its own retained recipient/target/route and
local delivery evidence, independent of individual invocation results.

This bounded Host/manual mapping permits at most 32 prepared native definitions,
128 retained records, 32 Action schemas / 4096 total name bytes, paths of 64 bytes
and a 4096-byte aggregate response. Existing TD semantic-arena, registry and
managed-account limits may refuse earlier. An oversized aggregate fails as a
whole with 503; it is never truncated or silently paginated. Native aggregation
prepays 65536 work units; managed participation reserves a 128 KiB controlled
I/O/formatting envelope in addition to definitions, records and services. OS
socket state remains Binding-owned. These are conservative controlled-backing
bounds, not process RAM or throughput measurements. Acknowledged Action
settlements retain their original managed funding until result disposal. In
addition to the native envelope, each managed Action acquires a separate original-bank
result/cleanup lease before behavior entry. Live invocations and acknowledged but
retained settlements share the configured Servient result-slot bound. Acknowledging
an HTTP record permits no uncharged result growth: a held settlement keeps its
slot, and reservation saturation returns 503 before application invocation.

Both completed-query and settlement output lend slices/fixed metadata through
Core `InteractionOutputView`/`PayloadView`; no payload/body Arc can be extracted. To share original
bytes, retain the settlement (for example in Rc) through their final use. An
explicit byte copy is separate caller-owned backing. Result disposal, including
the final shared settlement reference, returns its slot and byte/cleanup leases.
Finite Consumer `result()`, `rejected_output()` and `native_outcome()` use these
same borrowed views, including late native success fenced by cancellation or
revocation. `ConsumerNativeOutcome` preserves native retry/cleanup evidence
separately from application failure. Sharing a Consumer settlement keeps the
original response, native owner and original account reservations alive even
after call or root disposal; final owner disposal releases them. Explicit copies
own distinct backing and do not retain the original response allocation.
Consumer QueryAllActions preparation qualifies every resolved response assertion,
including foreign-vocabulary predicates; unsupported envelope instructions refuse
that candidate while independent supported operations may remain available.

[Real HTTP tests](tests/query_all_actions.rs) exercise both Engine roles,
independent native peers, concurrency, validation, complete qualification,
fresh permission, cancellation/replacement, response loss, capacity, original
pending replies, blocked cleanup and final account/socket release. Distributed
multi-owner aggregation, history completeness, atomic snapshots, general
DataSchema/security, TLS, constrained/automatic progress, replay and production
qualification remain unsupported. Query cancellation disposes its socket and
never sends an Action cancel or reinvokes application behavior.

## URI-variable Consumer Property interactions

Modern finite Consumer Property Read/Write supports the shared bounded
[URI-variable profile](../../../docs/evidence/uri-variables.md). Thing variables
and affordance overrides have one shared schema interpretation. Calls use
`association.read_property_with_variables(&calls, name, variables, deadline,
now, &mut budget)` or `association.write_property_with_variables(&calls, name,
payload, variables, deadline, now, &mut budget)` with unescaped scalar lexical
values; e.g. a prepared
`http://127.0.0.1:{port}/devices/{name}{?detail}` mapping can contact different
ports and paths without constructing new definitions or retaining the TD.

HTTP qualifies a fixed scheme and IP host, simple path/port and form-style
query/continuation expressions. Query templates may immediately follow the
authority; the Binding supplies the required root slash in HTTP origin-form
serialization under [RFC 9112 §3.2.1](https://www.rfc-editor.org/rfc/rfc9112.html#section-3.2.1).
Native resolution reparses the actual expanded
target before instance authorization or credentials, and native start verifies
that exact destination. There is no redirect, retry, DNS, host/scheme expansion
or implicit origin-wide audience. Exact `HttpClientAccess` audiences remain live
services independent of the definition; an out-of-audience target refuses before
credentials or I/O. The default destination bound remains 64 bytes;
`HttpConsumerConfig::with_target_capacity` permits an explicit 1–256-byte bound
for modern finite execution. Artifact/configuration identities version the change.

The concrete types/expressions, refusals, real HTTP/HTTPS evidence and remaining
limits are owned by the linked profile. In particular relative templates against
`base`, dynamic Property Producer matching and general caller-supplied variables
in other operation families remain unsupported. Existing native Action
continuation-ID mappings retain their separately qualified profile.
Finite Property Producer preparation/final qualification rejects dynamic declarations/templates;
Consumer support never licenses advertising an unqualified mapping.
