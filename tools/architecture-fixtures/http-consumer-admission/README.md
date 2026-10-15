# HTTP Consumer dependency admission discriminators

Non-publishable, standalone pre-implementation evidence for the
[local-disposal candidate](../../../docs/work-packages/WP-600-http-local-disposal-admission.md).
There is no engine binding, compiler, installed registration or production
dependency in this fixture. Its own `[workspace]` and lock leave the root Cargo
graph unchanged.

Run from the repository root:

```sh
cargo fetch --locked --manifest-path tools/architecture-fixtures/http-consumer-admission/Cargo.toml
python3 tools/architecture-fixtures/http-consumer-admission/run.py
cargo fmt --manifest-path tools/architecture-fixtures/http-consumer-admission/Cargo.toml -- --check
```

Only the first command needs network access on a cold cache. The runner uses
`--locked --offline`, disables incremental compilation, and places build output
under root `target/http-consumer-admission`. It runs the same eleven tests twice:
first against the unmodified registry source, then against an extracted,
disposable source copy with `closed-interest.patch`. It verifies the archive
checksum, every original registry file, unchanged non-Hyper lock identities,
and that Cargo actually selects exactly one patched Hyper. It never modifies
the Cargo registry, production source, or root manifest/lock.

`--variant upstream` and `--variant closed-interest` isolate those runs. The
completion-only command below additionally checks the installed source file set
and bytes against that exact patched archive, and builds against the installed
path. That path intentionally does not exist in this admission candidate:

```sh
python3 tools/architecture-fixtures/http-consumer-admission/run.py --variant installed
```

## Dependency and configuration identity

The fixture lock fixes the complete graph. Direct dependencies are Hyper
1.12.0 (`client,http1`), hyper-util 0.1.20 (`tokio`), Tokio 1.52.3
(`rt,net,time,io-util`), http 1.4.1, http-body-util 0.1.3 and bytes 1.11.1;
all disable default features. In particular, there is no pool, connector,
HTTP/2, TLS, resolver, client executor or client background task. The raw peer
uses the standard library in a separate fixture thread.

Hyper's registry archive SHA-256 is
`2c3e324da4c95177d6291d4c8730197c0d1822f8a9766814a4a44fa5ab797c9c`.
Its embedded VCS revision is
[`7f67699ce32d717081deb7018b4da7d679277ac0`](https://github.com/hyperium/hyper/tree/7f67699ce32d717081deb7018b4da7d679277ac0).
The public client builder sets `max_headers(16)`, `max_header_size(4096)`,
`max_buf_size(8192)` and `writev(false)`. These settings identify the experiment;
they are **not a production byte or work declaration**. No `read_buf_exact_size`
call overrides the maximum-buffer setting.

## Observations and repair boundary

Native x86-64 Linux, Rust 1.99.0, System allocator. Each measured window contains
only destruction of already constructed owners. A thread-local allocator
observer counts `alloc`, `alloc_zeroed` and `realloc` requests and requested
bytes; it performs no formatting or allocation itself. Peer-thread and runtime
construction allocations are outside the window. Counts are requested Layout
bytes, not retained heap, allocator overhead or process RSS.

| Suspended state; response/body interest destroyed before connection | Upstream allocation requests / bytes | Disposable repair |
| --- | --- | --- |
| Idle, no request | 0 / 0 | 0 / 0 |
| Queued request, ordinary or retry callback | 3 / 65 | 0 / 0 |
| Request dispatched, waiting for response head, either callback | 3 / 81 | 0 / 0 |
| Incomplete body, including a delivered data frame | 1 / 24 | 0 / 0 |
| Actual TCP peer holds an incomplete body open | 1 / 24 | 0 / 0 |
| Completed empty response | 0 / 0 | 0 / 0 |

The scripted I/O owner is destroyed exactly once. The TCP peer observes EOF or
reset after complete local disposal. Neither observation claims remote rollback.
Destroying the connection **before** live response/body interest still allocates
in the repaired source and delivers the original cancellation/dispatch or
incomplete-message error. Already accepted body data precedes the incomplete
error. These controls prevent a patch that merely suppresses error reporting.

The source-backed causes are
[`Envelope::drop` and `Callback::drop`](https://github.com/hyperium/hyper/blob/7f67699ce32d717081deb7018b4da7d679277ac0/src/client/dispatch.rs),
[`SenderDropGuard::drop`](https://github.com/hyperium/hyper/blob/7f67699ce32d717081deb7018b4da7d679277ac0/src/proto/h1/dispatch.rs),
and [`Error::new`](https://github.com/hyperium/hyper/blob/7f67699ce32d717081deb7018b4da7d679277ac0/src/error.rs).
The first three synthesize errors even after the corresponding receiver is
gone; `Error::new` boxes its representation. The patch checks the already
closed receiver before constructing that unobservable error. It makes existing
private closed-receiver queries available to those destructors, with no public
API, allocator replacement, shutdown task or new error representation.

Correct destruction order is essential: discard pending response futures,
response/body receivers and held frames, then the sender and connection. A
receiver cannot reopen. With all interests gone there is no concurrent body
receiver holding the private channel mutex. The remaining guards/fields perform
finite local release; the patch introduces no thread join, I/O polling or new
work. It does not promise allocation-free connection destruction while an
observer remains live. Ordinary paid I/O/error progress retains the Host SDK
failure scope of ADR-0022.

The two-target TCP case drives sender, response body and connection concurrently
on one application-owned current-thread Tokio runtime. The client never calls
`spawn`; fresh connections reach `/one` and `/two` on an independent peer.
This establishes a practical direct-ownership route. It is not a public
consume/read proof, Core result-seal test, resource qualification, benchmark,
SDK OOM recovery claim, TLS or embedded evidence.

## What remains outside these prechecks

The installed dependency must reproduce these results before this predecessor
is complete. The later HTTP tranche must independently establish its concrete
call's ordered Drop at every state, Runtime last-root residual/report/account
order, allocation-free raw/Core construction recovery, source-backed physical
input capacities and overlap, total/per-poll work, compiler support and paid
metadata eligibility. An idle Drop or successful TCP read alone proves none of
those obligations. Tests of the adapter that does not yet exist cannot run here.
