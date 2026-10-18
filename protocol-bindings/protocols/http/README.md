# Host HTTP Consumer Property Read — first production slice

`clinkz-wot-binding-http` is an optional Cargo dependency, used directly with
`ConsumerPropertyReadRuntime`. It does not change the umbrella facade or install
an executor. `HttpConsumerConfig::for_cleartext_ip()` explicitly permits cleartext
IP access. Keep this permission at the application boundary; there is no mutable
installed-client escape. Registration construction performs no I/O.

This candidate implements default GET / NoSec / effective `application/json`,
HTTP/1.1, status 200, IPv4/IPv6 literals, fixed-length and close-delimited bodies.
Only one call may occupy a registration, including its unread terminal result.
Each read uses a fresh connection. The selected owned artifact supplies the exact
path/query and Host authority. There is no source TD, selection, DNS, TLS, pool,
redirect, retry, decompression, upgrade, background task or detached driver in
this crate. Nonempty call URI variables and unsupported endpoint syntax reject
with the original request before raw acceptance. The restricted closed compiler
rejects additional Form metadata and non-JSON media before publication.

**Transfer-Encoding and Trailer reject, including chunked responses.** This is a
usable production implementation candidate within the existing admission, not
completion of `WP-600-HTTP-CONSUMER-PROPERTY-READ`. Successful chunked/trailer
qualification remains an obligation. Hyper's header-size setting also controls
trailers; simply using its default trailer handling would not establish the
admitted separate 16/2048 trailer envelope. No replacement HTTP parser is added.

## Application runtime

Run `cargo run --locked -p clinkz-wot-binding-http --example read_property --
TD.json PROPERTY` with an application-owned typed TD file. The example supplies
a checked owned Host policy, original parent accounts, monotonic clock, manual
progress, result-taking, shutdown and reclamation. It destroys the Thing after
consume. Loading that local file is application input, not bounded TD ingestion.

The application owns Tokio 1.52.3 and must enter an I/O-enabled runtime while
starting reads, then keep it alive and driving I/O throughout their lifetime.
A current-thread runtime must be driven by `Runtime::block_on`; merely entering
it does not drive its reactor. The example uses `enable_io().enable_time()`;
time is for application polling sleeps. The binding uses the supplied Servient
clock/deadline and needs no Tokio timer or spawned task. The application must
provide wake/poll progress and clock progress for slow peers and deadlines.

No entered runtime rejects before raw acceptance. Tokio lacks a public
nonpanicking enabled-I/O query, so acceptance probes reactor registration with
one unconnected, nonblocking socket, immediately dropped. It does not bind,
connect, read or write. A disabled-I/O panic is caught under unwind; the panic
hook can still run. Runtime shutdown after acceptance, `panic=abort`, process
OOM/abort and OS termination have no in-process reporting guarantee. No runtime
or thread is created by the library. Deployment owns shared reactor allocations,
thread-local caches, stacks, allocator fragmentation, OS socket buffers and FD/
process availability. The probe's extra temporary FD must be available.

## Enforced ingress and lifetime

The production ingress wrapper limits actual socket reads to 4096 bytes total
per paid BindingPolls callback, and each read to at most 4096. A cumulative wire
counter never resets and stops reads at 32768. It retains one fixed 8192-byte raw
head, inspecting it with pinned httparse before handing bytes to Hyper. The
32-header array rejects overflow, including duplicate field lines; exact header
bytes include status line and terminating CRLF. Unsupported 1xx, versions,
status, media, coding and framing reject before Hyper can normalize or hide them.
Media is exactly `application/json`, with no parameters or duplicate field.
There is no implicit drain after rejection.

Hyper owns HTTP framing. Its builder uses `max_headers(32)`,
`max_header_size(8192)`, `max_buf_size(8192)` and `writev(false)`; no permissive
parser options or HTTP/2 features. The body destination reserves 4096 bytes
fallibly and checks every frame before copying. Content-Length over 4096 rejects
in the raw head. Unknown-length overflow rejects after bounded read-ahead.
The fixed raw head (8192), per-read temporary (4096), body destination (4096),
Hyper read storage and header structures may overlap. Hyper's buffer setting is
a growth strategy, not a statement of exact heap capacity; its backing/slices,
channel state and allocator overhead must not be inferred from useful byte
length. The wire/read gate bounds received input regardless of SDK buffering.

The complete raw call owns its connection future and the Hyper driver. Both are
polled together in one future. Return, cancellation and drop destroy that future
and local socket before terminal settlement/release; no join, network wait,
reconnect, replacement work or unowned tail is needed. This says nothing about
whether the remote read executed. Root loss preserves Servient's precommitted
residual report even when the socket was locally retired.

Core receives only ordinary untrusted outputs, then applies the existing seals
and validation. Body is a complete `Arc<[u8]>`; media capacity is 16, with a second
16-byte coding capacity promised conservatively. Engine funding includes full
Arc control/alignment and result overlap through the public owned policy. The
raw carrier reports its concrete Layout; protocol future, socket and parser
backing remain binding-private. Registration declares a conservative client/Arc
allowance; complete Core compiler/envelope and engine costs remain owned by
existing public admission. The 16384-byte callback declaration is a conservative
configured allowance, not a measured maximum native stack or transitive SDK
heap certificate. Native stack peak and all SDK allocation-failure/disposal paths
remain unqualified completion evidence.

## Executable evidence and remaining work

Run `cargo test --locked --offline -p clinkz-wot-binding-http --all-targets --
--test-threads=1 --nocapture`. Python 3 is needed for the second independent peer.
The other peer writes raw TCP bytes and uses neither Hyper nor httparse. Tests
exercise public registration/consume/select/read/cancel/drop/shutdown, destroyed
TD input, 100 alternating targets, fresh connections, exact/one-over 32/8192
headers and 4096 bodies, fragmented fixed/close bodies, malformed/truncated and
silent peers, statuses/encodings/1xx, explicit chunk rejection, no reconnect,
deadlines, drain, root loss, unchanged rejected variables and parent reclamation.
The allocator observer records one 4096-byte public result path, including typed
input construction, and checks no tracked backing survives runtime destruction.
A silent-peer cancellation separately measures SDK retirement allocations and
checks that engine cancel/status and result-taking request no allocations. On
the measured build SDK retirement requested four allocations (113-byte peak),
including Tokio's deregistration queue and Hyper's dispatch error. Tokio retains
32 bytes of runtime-owned queue backing until a reactor turn/destruction. The
FD and connection future have already retired; the application must continue
driving the shared runtime. These are path observations, not universal disposal
guarantees or an exact SDK heap bound.

Completion still requires successful chunked/trailer envelope qualification;
real exact/one-over trailer and wire/framing cases; comprehensive SDK buffer/
continuation/transient/stack bounds and supported fault injection; invalid/late
result and deadline/result race renewal with the real transport; further FD and
ownership observations under pressure; and independent same-revision acceptance.
The accepted predecessor proofs remain authoritative at their recorded revisions.
This candidate changes none of their implementations or gate states. HTTPS,
trust/security, Producer and embedded/hardware qualification retain their own
later boundaries. See the work-package evidence for exact commands/revisions and
what was actually run.
