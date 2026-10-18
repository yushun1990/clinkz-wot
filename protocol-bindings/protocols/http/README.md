# Host HTTP Consumer Property Read

`clinkz-wot-binding-http` is an optional Cargo dependency used with the public
`ConsumerPropertyReadRuntime`. `HttpConsumerConfig::for_cleartext_ip()` explicitly
permits cleartext IP access; `try_consumer_registration` constructs one complete,
restricted Consumer-only registration without I/O. The closed compiler accepts
metadata-free default GET / NoSec / effective `application/json` Forms.

The binding supports HTTP/1.1 status 200, IPv4/IPv6 literals and fixed-length,
chunked and close-delimited bodies. One registration admits one call, including
its unread terminal result. Each read owns a fresh connection and its driver.
The selected immutable artifact supplies the exact path/query and Host authority.
Nonempty URI variables and unsupported targets return the original request before
raw acceptance. No TD rescan, DNS, TLS, pool, redirect, retry, decompression,
upgrade, background task or detached driver is involved.

This implementation uses the existing admission restored after PR #199 and the
accepted [parser-policy amendment](../../../docs/work-packages/WP-600-http-property-read-target.md#http-parser-policy-amendment).
It is production progress submitted for independent review; the tranche remains
in progress. Historical v1 refusal and the unmerged PR #197 fork are not evidence
for the new stock SDK paths.

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

## Identity, buffering and ownership

The actual private encoding is `clinkz-http-ip-v5\0` followed by the exact ASCII
policy in `src/lib.rs::PRIVATE_POLICY`. It captures pinned SDK versions/features,
parser comparisons, extension and wire/read limits, declaration/discard/override
behavior, advertisement, concurrency/growth, retirement/disposal and endpoint
restrictions. V5 additionally captures connect I/O failure as
`DeploymentFailure`: socket/process/network availability is distinct from a
captured protocol limit. Parser policy and SDK settings retain accepted v4
behavior. The existing checked restricted configuration derives from its
SHA-256, preserving shared encodings, schema 2 and result limits. Support, client,
compiler, complete registration, selected artifact and diagnostics agree.
Historical v1/v3/v4 and unrestricted configurations cannot substitute in either
direction; the tests recover original owners/requests and verify no connection.
A captured registration or old artifact is never relabeled to enable v5.

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
a successful read on the same Runtime. Old `tests/sdk_limits.rs` probes retain
their historical incompatibility scope; they are not successor failures.

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
On x86_64, the allocation fixture also refuses the fallible 288-byte raw-call
box and verifies request recovery, occupancy release and retry. SDK infallible
OOM remains a Host process-failure assumption.

PR #200 independently accepted the v4 production slice as partial progress.
Further completion evidence includes representative native-stack and disposal
costs and remaining supported runtime/fault and retirement paths; universal SDK
heap/stack bounds are not a Host requirement. Independent review of this change
and tranche completion remain open. The evidence records exactly which
workspace, feature, portable, predecessor and authority checks ran. Passing tests
or the required GitHub diff-hygiene check do not complete this tranche or change a
gate. HTTPS/trust/security, Producer and embedded/hardware qualification retain
their separate later boundaries; HTTPS/security remains a release requirement.
