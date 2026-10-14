# Host Zenoh final-owner discriminators

Non-production evidence for the
[WP-600 owner decision](../../../docs/work-packages/WP-600-host-zenoh-owner-decision.md)
and workspace topics 0083/0076. These programs use stable public Zenoh and
Foundation APIs. They author no engine binding, registration, call, transfer,
request, validator or runtime. The separate owner's role lease is a local model.

Run from the repository root on Linux with Python 3 and the cached locked Cargo
dependencies:

```sh
python3 tools/architecture-fixtures/host-zenoh-owner/run.py
rustfmt --edition 2024 --check tools/architecture-fixtures/host-zenoh-owner/probe.rs
```

The runner builds the existing backend/Foundation with `--locked --offline`,
discovers SDK/Tokio/Foundation rlibs from Cargo artifact messages and uses rustc
to compile into a temporary directory. No new Cargo crate, dependency, lockfile,
production API or checker is introduced. Each child has a 15-second timeout.
Core dumps are disabled for the two intentionally aborting subprocesses.

Reproduced against master `0e3cafd52207e55327ef978cebf46e8d10092a34`, Zenoh 1.9.0
(VCS `81c6c933b6e41d72a05f04c4442ef57717ddc72b`), Rust 1.99.0,
x86_64-unknown-linux-gnu and the native System allocator. Local peer mode disables
multicast/gossip/listeners. `rx-limits` adds two real sessions and one loopback
TCP connection; no production Consumer is tested.

| Case | Executable observation |
| --- | --- |
| `last-drop` | Hold a timeout callback after it enters. Last Session Drop cannot return during the 100-ms observation window; it returns after release. This is not a worst-case latency measurement. |
| `abandoned-close` | Poll the original close to Pending while that callback is held, then destroy the future. `is_closed`, a successful retry and final Session Drop all precede callback completion. |
| `get-callback` | Hold a successful local reply callback inside synchronous get. First close returns successfully while the get worker is still inside the callback. Job/callback completion is a separate barrier. |
| `separate-owner` | A scoped worker retains the SDK owner and one committed original Foundation cleanup-item account. A model role lease drops without calling-thread allocation. That same parent stays exhausted after lease Drop and while close waits; release occurs after first close and final SDK destruction, then the worker is joined. |
| `closed-drop` | An already fully closed Session's final Drop still makes calling-thread allocation requests (two in the recorded run). The assertion is positive count, not an exact universal count. |
| `closed-drop-oom` | Return null for the next allocation only on the thread dropping a closed Session. The subprocess aborts with SIGABRT and the allocator-failure diagnostic; it cannot produce a recoverable rejection. |
| `original-close-timeout` | Hold a timeout callback, drive the actual SDK ten-second close timeout on a separate Tokio clock, and observe Err. Retry Ok and final Session Drop precede callback completion. The callback is then released. |
| `query-start` | Warm one SDK submission on the worker; construct the callback and next builder before metering. Resolving that public SDK get builder still allocates. Assert positive count, not a universal allocation count. |
| `query-start-oom` | Return null for the next allocation on that protocol worker during submission. SIGABRT terminates the process; no SDK error or recoverable worker result is produced. |
| `rx-limits` | Configure the client for one session/link, 65,535-byte batches/RX buffer, and 1,024-byte max message size. Receive sixteen 8,192-byte TCP replies with consolidation disabled and an explicit finite 16-item FIFO. Retain their actual SDK payloads simultaneously: 131,072 payload bytes. |

The source explains why the first two joins differ. SDK Session Drop calls
`close().wait()`; close takes primitives before awaiting the task controller.
Retry with no primitives skips that original wait. The close builder boxes its
future, and the asynchronous close implementation boxes additional state.
`Session::drop`, `api/builders/close.rs::IntoFuture` and
`api/session.rs::WeakSession::close_inner` at the locked revision are the relevant
source owners. The timeout case now executes the unmodified internal deadline;
the separate abandonment case does not wait for it.

`rx-limits` falsifies interpreting `transport/link/rx/max_message_size` as a
limit on all input messages: universal RX applies it to fragmented messages,
while an unfragmented reply fitting the negotiated batch passes. It also
falsifies interpreting `buffer_size` as a retained-byte maximum: RX acquires
buffers with `pool.try_take().unwrap_or_else(|| pool.alloc())`. See locked
`zenoh-transport::unicast::universal::{rx,link}` and
`zenoh-config::LinkRxConf`. The measured sum is payload length, not allocated
capacity, peak process memory or proof of an unbounded queue. This test
deliberately retains a finite set of Samples; a binding can choose an immediate
nonblocking callback and smaller batches, but must still account for SDK backing
before callback entry and provide recovery or actual prebacking.

The allocator meter is thread-local, so unrelated SDK startup allocations do not
make the lease-Drop check flaky. It counts allocation/reallocation requests, not
live/peak bytes, largest Layout, stack use or a production memory ceiling. The
fault injector is also thread-local and does not affect other SDK worker threads.
The held callback is deliberate fault injection; it is not proposed production
callback behavior or proof that every Zenoh close blocks this way.

The `separate-owner` test establishes a real original account and explicit
retirement. Its one cleanup item is **not** an admitted SDK byte allowance. It
proves no final worker/controller abandonment, complete Runtime capture, residual successor,
callback ingress bound, allocator recovery or production shutdown. Parent
charging protects only this declared lifetime slot. Non-aborting cases keep an
explicit live owner, release held callbacks and join their explicit workers.
The OOM cases intentionally terminate before cleanup. No new full-owner model
is constructed: the dependency failures are the decision's stopping evidence.

These are falsifiers and a limited construction experiment. They do not replace
the original 0083 timer/epoch experiments, the accepted Consumer/Producer gates,
ADR-0013 admission, the six production completion keys or real pico evidence.
