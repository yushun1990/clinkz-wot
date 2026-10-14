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
Core dumps are disabled for the intentionally aborting subprocess.

Reproduced against master `0dbf0bbc0e144b5dc2695ecf6e66ace77bbb67fd`, Zenoh 1.9.0
(VCS `81c6c933b6e41d72a05f04c4442ef57717ddc72b`), Rust 1.99.0,
x86_64-unknown-linux-gnu and the native System allocator. Local peer mode disables
multicast/gossip/listeners. No real remote peer or production Consumer is tested.

| Case | Executable observation |
| --- | --- |
| `last-drop` | Hold a timeout callback after it enters. Last Session Drop cannot return during the 100-ms observation window; it returns after release. This is not a worst-case latency measurement. |
| `abandoned-close` | Poll the original close to Pending while that callback is held, then destroy the future. `is_closed`, a successful retry and final Session Drop all precede callback completion. |
| `get-callback` | Hold a successful local reply callback inside synchronous get. First close returns successfully while the get worker is still inside the callback. Job/callback completion is a separate barrier. |
| `separate-owner` | A scoped worker retains the SDK owner and one committed original Foundation cleanup-item account. A model role lease drops without calling-thread allocation. That same parent stays exhausted after lease Drop and while close waits; release occurs after first close and final SDK destruction, then the worker is joined. |
| `closed-drop` | An already fully closed Session's final Drop still makes calling-thread allocation requests (two in the recorded run). The assertion is positive count, not an exact universal count. |
| `closed-drop-oom` | Return null for the next allocation only on the thread dropping a closed Session. The subprocess aborts with SIGABRT and the allocator-failure diagnostic; it cannot produce a recoverable rejection. |

The source explains why the first two joins differ. SDK Session Drop calls
`close().wait()`; close takes primitives before awaiting the task controller.
Retry with no primitives skips that original wait. The close builder boxes its
future, and the asynchronous close implementation boxes additional state.
`Session::drop`, `api/builders/close.rs::IntoFuture` and
`api/session.rs::WeakSession::close_inner` at the locked revision are the relevant
source owners. A ten-second internal close timeout is source evidence; this
fixture tests abandonment instead of waiting for that timeout.

The allocator meter is thread-local, so unrelated SDK startup allocations do not
make the lease-Drop check flaky. It counts allocation/reallocation requests, not
live/peak bytes, largest Layout, stack use or a production memory ceiling. The
fault injector is also thread-local and does not affect other SDK worker threads.
The held callback is deliberate fault injection; it is not proposed production
callback behavior or proof that every Zenoh close blocks this way.

The worker test establishes a real original account and explicit retirement. Its
one cleanup item is **not** an admitted SDK byte allowance. It proves no final
worker/controller abandonment, complete Runtime capture, residual successor,
callback ingress bound, allocator recovery or production shutdown. Parent
charging protects only this declared lifetime slot. Every test keeps an explicit
live owner and eventually releases its callback and joins its worker.

These are falsifiers and a limited construction experiment. They do not replace
the original 0083 timer/epoch experiments, the accepted Consumer/Producer gates,
ADR-0013 admission, the six production completion keys or real pico evidence.
