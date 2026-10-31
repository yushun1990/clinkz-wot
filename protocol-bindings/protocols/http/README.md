# Host HTTP Properties and Actions

`clinkz-wot-binding-http` is an optional Cargo dependency used with the public
open `ConsumerRegistry`/`ConsumerRuntime`. `HttpConsumerConfig::for_cleartext_ip()` explicitly
permits cleartext IP access; `try_consumer_registration` constructs one complete,
Consumer-only registration without I/O. Its native compiler implements the
bounded NoSec / effective `application/json` Property Read/Write mapping from
TD-resolved semantic input. The [composed Write subset](#composed-property-readwrite)
uses separate input and completion roles. The retained `ConsumerPropertyReadRuntime`
uses `try_inline_consumer_registration` and remains a restricted Read adapter.

Reads support HTTP/1.1 status 200, IPv4/IPv6 literals and fixed-length,
chunked and close-delimited bodies. One registration admits one call, including
its unread terminal result. Each read owns a fresh connection and its driver.
The selected immutable artifact supplies the exact path/query and Host authority.
Nonempty URI variables and unsupported targets return the original request before
raw acceptance. No TD rescan, DNS, TLS, pool, redirect, retry, decompression,
upgrade, background task or detached driver is involved.

The Consumer tranche is complete within its independently accepted scope; its
[evidence](../../../docs/evidence/WP-600-http-consumer-property-read.toml)
owns that exact boundary and subsequent amendments retain their own provenance.
The [parser-policy amendment](../../../docs/work-packages/WP-600-http-property-read-target.md#http-parser-policy-amendment)
defines the stock SDK envelope. Historical v1 refusal and the unmerged PR #197
fork are not evidence for the new stock SDK paths.

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
`htv:statusCodeValue` may explicitly be 200; other response codes/media, additional
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
including no URI-template/coding/additional-response execution. Common fact access
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
response backing. Tests report actual allocation/FD disposal observations; these
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
JSON/200 and the type-only schemas described above. It supports anonymous TDs,
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
definitions from each effective TD Form operation. `ConsumerRuntime::write_property`
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
addresses, cleartext IP Consumer targets, endpoint NoSec and JSON/type-only
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
Expect, upgrades, pipelining and credential headers are refused. Native parsing
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
`ConsumerEffect::Unknown` after native progress does not imply rollback on error
or cancellation. Producer result, `FiniteEffect` and `FiniteDelivery` similarly
retain separate facts. No owner automatically retries a write. Unread settlements
pin original capacity/native occupancy; cancellation, replacement and root loss
preserve accepted work until bounded retirement and final acknowledgement.

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

All final TD Forms, effective operations, resolved security and type-only Action
input/output schemas are qualified through the existing whole-Thing selector,
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
captures the source-independent validator at publication and validates JSON/type
before behavior; each admitted continuation retains that original output service.
An explicitly raw Servient registration remains unvalidated, but a published
Action cannot bypass its captured validator through `RawActionRequest`.
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
`servient_finite::consumer::ConsumerRuntime`. `try_consumer_registration` returns
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
HTTP lowering preserves the qualified Producer mapping above: JSON/type-only,
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
v9. There is one active exchange per actual registration; the Engine independently
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
