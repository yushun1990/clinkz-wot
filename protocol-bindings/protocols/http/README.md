# Host HTTP Consumer Property Read

`clinkz-wot-binding-http` is an optional Cargo dependency used directly with
`ConsumerPropertyReadRuntime`. `HttpConsumerConfig::for_cleartext_ip()` explicitly
permits cleartext IP access. The complete restricted registration captures that
immutable configuration; constructing it performs no network I/O.

This implementation supports default GET / NoSec / effective `application/json`,
HTTP/1.1, status 200, IPv4/IPv6 literals, and fixed-length, chunked and
close-delimited bodies. Each call owns a fresh connection. One call, including
its unread terminal result, may occupy a registration. The selected owned
artifact supplies the exact path/query and Host authority. Nonempty call URI
variables and unsupported endpoints reject with the original request. The paid
Form restriction rejects additional metadata and non-JSON media before
publication. The binding retains no TD and performs no selection, DNS, TLS,
pooling, redirect, retry, decompression, upgrade or detached work.

The private [Hyper 1.12.0-clinkz.1 correction](../../vendor/hyper/CLINKZ.md)
adds independent inclusive trailer limits and configurable cumulative extension
accounting. Three source files differ from the verified upstream archive; Hyper
still owns all framing. Original Hyper 1.12.0 counterexamples remain executable
against a separate dev dependency in `tests/sdk_limits.rs`. Public shared APIs,
resource contracts and runtime ownership are unchanged. Tranche completion and
independent acceptance remain open.

Only one `Transfer-Encoding: chunked` is supported, with no Content-Length.
Duplicate Content-Length, other transfer codings and malformed/conflicting
Trailer declarations reject. Requests advertise `TE: trailers` with its
Connection option. Declared or undeclared trailers are accepted within the
limits below, then discarded. Neither declarations nor actual fields may
replace Content-Type, Content-Length, Transfer-Encoding, Content-Encoding,
Trailer, Upgrade, Connection, Host or TE. Trailer metadata cannot change the
primary result's status, media or body.

## Application runtime

Run `cargo run --locked -p clinkz-wot-binding-http --example read_property --
TD.json PROPERTY` with an application-owned typed TD. The example supplies a
checked ConsumerOwnedHostV2 policy, original parent accounts, monotonic clock,
manual progress, result-taking, shutdown and reclamation. It destroys the Thing
after consume. Loading that local file is application input, not bounded remote
TD ingestion.

The application supplies Tokio 1.52.3, enters an I/O-enabled runtime when
starting reads, and keeps it alive and driving I/O throughout their lifetime.
A current-thread runtime needs `Runtime::block_on`; entering it alone does not
drive the reactor. The example uses `enable_io().enable_time()`; timers are for
application polling sleeps. The binding uses the Servient clock/deadline, with
no Tokio timer or spawned task. Applications must supply wake/poll and clock
progress for slow peers and deadlines.

Missing runtime/I/O services reject before raw acceptance. Tokio has no public
nonpanicking I/O-driver query, so acceptance probes registration using one
unconnected nonblocking socket, immediately dropped. It does not bind, connect,
read or write. A disabled-I/O panic is caught under unwind; the panic hook may
run. Runtime shutdown after acceptance, `panic=abort`, process OOM/abort and OS
termination have no in-process reporting guarantee. Deployment owns shared
reactor allocations, thread-local caches, stacks, allocator fragmentation, OS
socket buffers and FD/process availability, including the probe's temporary FD.

Applications can depend on this repository's HTTP crate by path; the SDK path
dependency travels with the checkout. No root Cargo patch is needed. The library
implements Hyper's I/O traits on its existing ingress wrapper; hyper-util is a
dev dependency only. Registry distribution of the private SDK version remains
an obligation before publishing a release.

## Enforced ingress and lifetime

The fixed policy limits are:

| Input | Maximum |
| --- | --- |
| Main response head | 32 field lines / 8192 raw bytes |
| Decoded body | 4096 bytes |
| Trailers | 16 field lines / 2048 raw bytes |
| Complete response wire | 32768 cumulative bytes |
| Socket read / paid BindingPolls callback | 4096 / 4096 input bytes |

Head bytes include status line and terminating CRLF. Trailer bytes include field
lines and terminating CRLF. Duplicate field lines count separately. Framing,
including extensions, consumes the same lifetime wire total without resets.
The extension ceiling is 32768, so it cannot mask a permitted complete response.
Unsupported informational responses, versions, statuses, media and codings
reject without draining. Media is exactly `application/json`, without parameters
or duplicate fields.

The ingress wrapper retains a fixed 8192-byte head, inspecting it with pinned
httparse 1.10.1 before handing each read to Hyper. The SDK builder selects
`max_headers(32)`, `max_header_size(8192)`, `chunked_limits(16, 2048, 32768)`,
`max_buf_size(8192)` and `writev(false)`. The decoder rejects trailer byte 2049
before appending it and field 17 at its first byte, including unfinished input.
A socket read may already contain that offending byte and read-ahead: at most
4096 bytes per read/callback and 32768 for the call. The decoder's raw-span
limit is not a maximum for all simultaneously retained SDK backing.

The body destination reserves 4096 bytes fallibly and checks each frame before
copying. Content-Length over 4096 rejects at the raw head. Unknown-length and
chunked overflow reject after bounded read-ahead. The raw head, 4096-byte read
temporary, body destination, SDK read storage, trailer buffer and header maps
may overlap. Hyper's buffer setting controls its growth strategy; it does not
prove exact retained capacity, allocation overhead or a transitive heap bound.

One raw call owns the entire connection future and driver, polled together.
Return, cancellation and drop destroy the future/socket before local terminal
settlement. There is no network wait, join, reconnect, replacement task or
unowned tail. Local retirement cannot certify whether the remote read ran.
Root loss preserves Servient's precommitted residual report.

Core receives ordinary untrusted outputs and applies existing validation and
seals. The body is a complete Arc allocation; the media String capacity is 16,
with a second 16-byte coding capacity promised conservatively. Original engine
accounts fund the complete carrier, envelope, result backing/control/alignment,
overlap and cleanup/handoff until release. Protocol buffers, futures and socket
backing remain binding-private. The 16384-byte callback allowance is configured,
not a measured native stack maximum or SDK heap certificate.

## Executable evidence and limits

Run `cargo test --locked --offline -p clinkz-wot-binding-http --all-targets --
--test-threads=1 --nocapture`. Python 3 supplies the second independent HTTP peer;
the other writes raw TCP bytes without Hyper or httparse. Tests use the complete
public registration/consume/read path and destroy source input before execution.
One runtime executes 100 alternating distinct targets using fixed, chunked with
trailers, and close-delimited responses, checking fresh connections and release.

Public tests cover exact/one-over header, body, trailer and wire limits,
including simultaneous 32/8192 head, 4096 body and 16/2048 trailers. Raw padded
trailers demonstrate that normalized maps cannot hide overflow. Real one-octet
writes split head/chunk/extension/trailer delimiters through socket wakes.
32767/32768-byte responses succeed; 32769 rejects. Both many small chunks and a
single large extension reach the wire boundary with bodies below 4096. Totals
end at the response terminator; they do not certify trailing unrelated input.

Unfinished 17-field and 2049/4096-byte trailers automatically reject within a
250 ms test guard, without caller cancellation/deadline/peer EOF, retire, allow
a later read and reclaim original accounts. A direct real-socket ingress test
checks zero credit, shared callback credit and no receive of wire byte 32769.
Malformed/truncated/disconnected/slow peers, unsupported responses, cancellation,
dropped tickets, deadlines, drain/root loss, rejected-input recovery and reuse
remain covered. Partial chunk/body/trailer phases exercise cleanup. Controlled
fixed-body and final-trailer orders exercise cancel-before-progress and
result-before-cancel; comprehensive concurrent race/pressure evidence is open.

Allocation observations cover fixed and maximal-trailer chunked 4096-byte
results. They include typed input and same-thread first-use reactor costs,
exclude runtime creation and peer-thread allocations, and observe zero tracked
backing after root/runtime destruction. Silent-peer SDK cancellation requests
four allocations (113-byte observed peak); tested partial body/trailer disposal
requests two (56-byte peak). Tokio retains 32 runtime-owned deregistration bytes
until reactor progress/destruction. Engine cancel/status and result-taking
allocate nothing in those cases. These are path/build observations, not ceilings.
Supported fallible body-reservation refusal yields 607/DeploymentFailure;
decoded-body overflow yields 608/BindingPolicy. Both fixed and chunked paths
verify identity/context, retirement, successful retry and parent reclamation.

Remaining completion obligations include comprehensive SDK retained/transient/
continuation/stack and supported fault/disposal qualification, real transport
invalid/late-result and deadline/result races, pressure/FD observations, required
whole-workspace completion validation and independent same-revision acceptance.
Unchanged shared/portable/Producer evidence retains its original provenance.
HTTPS/trust/security, Producer and embedded/hardware retain later boundaries.
The existing WP-600 evidence owner records exact revisions, checks and limits.
