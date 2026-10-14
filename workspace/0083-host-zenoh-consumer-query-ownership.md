# Host Zenoh Consumer Property Read: SDK ownership admission blocker

Status: DISCUSSING

## Protocol-owner subquestion migrated; qualification remains open

The next decision was reconstructed independently from fetched master
`0dbf0bbc0e144b5dc2695ecf6e66ace77bbb67fd`, current Runtime/registration code,
the pinned SDK and fresh executable probes. Its selected owner topology,
alternative dispositions and precise remaining blocker now belong to
[the WP-600 decision](../docs/work-packages/WP-600-host-zenoh-owner-decision.md).
The necessary shared accounting/destructor interpretation is projected into
[Binding SPI](../docs/spec/binding-spi.md#shared-protocol-infrastructure-lifetime).
These are review candidates, not independent acceptance or production admission.

The [public-SDK final-owner fixture](../tools/architecture-fixtures/host-zenoh-owner/README.md)
distinguishes a last Session's blocking/allocating Drop, original-close
abandonment, get-job callback quiescence and a separately retained original
Foundation account. A scoped worker is constructible; its final-owner loss and
complete SDK resource/recoverability envelope are not proved. The broader
qualification investigation therefore remains DISCUSSING. The historical
timer/epoch observations below retain their original claims and limits.

Topic 0076 intersects this decision through independently composable capability
and physical runtime ownership. Its broader Contracts/image/package proposal
is not selected. Neither the prior epoch experiment nor rejection of a broad
redesign establishes final registration/worker retirement.

## Finding and admission consequence

The first production WP-600 boundary remains one real Host Consumer Property
Read through the accepted public Runtime, with repeated reads after terminal
release and a production Producer regression/round trip. The current concrete
binding cannot enter that boundary by wrapping its existing transport. In
addition to its legacy API, the locked SDK retains query work beyond reply
completion in a session owner whose lifetime and capacity are not represented
by the call limit.

The current backend's session/query owner and resource accounting require
qualification. Accepted shared-registration accounting permits session
infrastructure to outlive a successful read when its complete ownership and
immutable maximum remain charged. A fixed-budget reusable session epoch is a
credible candidate within that contract; neither one session per read nor an
individual SDK retirement hook per query is required.

ADR-0013 admission requires the chosen ownership/resource contract, closed
projections and passing checks that can run before production exists. Actual
target implementation measurements remain completion evidence. This DISCUSSING
investigation grants no WP-600 admission, implementation permission, acceptance,
gate reopening or production completion.

## Reconstructed authority and predecessors

The examined repository revision is `6185e2afeccbe9dca199f8aea7486f695183f8c3`.
The accepted
[Consumer gate](../docs/work-packages/consumer-property-read-architecture-gate.toml)
has passed; its ten prerequisites are complete/admitted/current with their
registered passed evidence. The exact gate and Runtime acceptance boundaries
are retained by their existing owners, including
[PR #171's independent acceptance](https://github.com/yushun1990/clinkz-wot/pull/171#pullrequestreview-5514507904)
and [the Runtime record](../docs/work-packages/WP-400-consumer-property-read-runtime-admission.md).
They qualify a finite mock author, not Zenoh SDK callbacks or buffers.

| Predecessor boundary | Authoritative evidence owners |
| --- | --- |
| Complete borrowed TD inspection and semantic lending; call values/response predicate | `WP-100-consumer-borrowed-td-admission.toml`, `WP-100-consumer-call-values-response-validation.toml` |
| Selected request, closed compiler support, all-readable aggregate and unique owned output/root | `WP-200-consumer-property-read-planning-selection.toml`, `WP-200-consumer-compiler-support.toml`, `WP-200-consumer-property-read-aggregate.toml`, `WP-200-consumer-output-ownership.toml` |
| Installed normal/late result seal, complete Consumer-only registration and recoverable Host call backing | `WP-300-consumer-property-read-binding-execution.toml`, `WP-300-consumer-only-registration.toml`, `WP-300-consumer-host-call-construction.toml` |
| Publication, one generation/call, exact local/parent capacity, manual cancellation and reclamation | `WP-400-consumer-property-read-runtime.toml` |

All evidence paths above are under `docs/evidence/`; the
[work-package index](../docs/work-packages/index.toml) owns their dependency
closure and status. Producer predecessors and the passed
[Producer gate](../docs/work-packages/property-read-architecture-gate.toml)
remain required regression authority.

The applicable
[ADR-0013](../docs/ADRs/0013-work-package-scoped-implementation-admission.org)
requires stable scoped lifecycle/resource ownership, closed affected projections,
passing executable prechecks, an exact-revision independent admission review
and completion evidence defined before implementation. Completed predecessors
satisfy its dependency condition, not its concrete-backend conditions.

[ADR-0006](../docs/ADRs/0006-host-binding-call-cancellation.org) and the active
[Binding SPI](../docs/spec/binding-spi.md) require an owned, side-effect-free
constructor, retained normal/late values, complete-object cleanup transfer or
durable residual, and immutable maxima including SDK/reactor/external-input
growth. `BIND-MEM-001`, `BIND-CALL-CANCEL-001` and
`BIND-HOST-CANCEL-001` are the immediate intersections. The
[Runtime admission](../docs/work-packages/WP-400-consumer-property-read-runtime-admission.md#resource-admission-and-trusted-callback-boundary)
binds trusted author limits to the actual moved registration. Compiler support,
a compatible identity, an item-bounded receive queue or a logical byte allowance
does not certify the concrete constructor, callbacks, result backing or Drop.

Topics 0076, 0081 and 0082 do not require a contracts extraction, reusable-plan
decision or subscription-sharing decision for this one-generation one-shot.
This finding intersects the real Host backend only; mock Consumer and Producer
acceptance is preserved.

## Concrete binding and locked SDK evidence

- `protocol-bindings/protocols/zenoh/src/protocol_binding.rs` returns legacy
  `Arc<dyn clinkz_wot_core::ClientBinding>` components, not the target
  `clinkz_wot_core::binding::ClientBinding`/complete Consumer registration.
  `zenoh.rs::invoke` validates TD/Form again and consults a pointer-keyed Form
  cache. `runtime/pool.rs` lazily opens and retains sessions by authority.
  These edges cannot be the target path.
- `runtime/zenoh.rs::get` starts a query with `builder.wait()`, waits with
  `recv_timeout`, copies the sample into infallibly allocated payload storage,
  and returns an output. It retains no generation-bearing query cleanup call.
  `selector.rs` also uses infallible variable String backing. Replacing those
  edges is necessary implementation work; API naming alone is not the blocker.
- `tests/target_property_read_feedback_probe.rs` defines its target compiler
  and Host/static server in the test file. Its compiler accepts ProducerRoute
  only. Its client uses raw SDK queries. The seven real-network cases reaffirm
  route/permit/carrier ownership, not production Consumer qualification.
- `Cargo.lock` pins Zenoh 1.9.0, checksum
  `85e22d7002ac149ef17fe400bb40a267ebbba40a83413bab03da7762256fa94e`.
  The published crate's VCS revision is
  `81c6c933b6e41d72a05f04c4442ef57717ddc72b`. The following findings use that
  source, not a later SDK.

In the pinned
[query builder](https://github.com/eclipse-zenoh/zenoh/blob/81c6c933b6e41d72a05f04c4442ef57717ddc72b/zenoh/src/api/builders/query.rs#L380),
`IntoFuture` constructs `Ready(self.wait())`; `await` does not defer query
submission into cooperatively polled steps. The same is true of the open
builder. Moving these expressions into a boxed async block defers invocation
but does not bound the synchronous work it eventually executes.

In the pinned
[session query implementation](https://github.com/eclipse-zenoh/zenoh/blob/81c6c933b6e41d72a05f04c4442ef57717ddc72b/zenoh/src/api/session.rs#L2660),
each query spawns a session-owned timeout task. Its token is a child of the
session task controller. A final reply removes QueryState; it does not signal
that task. With no further query state, the task still sleeps until its timeout
or session termination. Dropping a default FIFO receiver does not remove
QueryState. The query cancellation API is feature-gated `unstable`, currently
disabled; it removes query state but does not itself cancel/join the timeout
task. Enabling it alone cannot close this finding.

The default FIFO is count-bounded but its callback uses blocking `send`; an
item bound is not a byte bound or a nonblocking overflow policy. Default
consolidation can retain replies. These mechanisms require explicit policy and
physical accounting, including payload/media growth, before a new client is
eligible. The probe below disables consolidation and drains every reply, so
neither mechanism explains away its retained task tail.

The first fully driven `Session::close` clears query state and awaits session
task termination. It defaults to no callback wait; callback/get/reactor jobs
also need their own quiescence proof. The close implementation takes the
session primitives before its await. If that original future is abandoned or
times out, a later close can return without repeating its join. Retain the
original close owner and its capacity until actual retirement or truthful
residual; `Ok(())` from a retry, `Drop` or `is_closed()` cannot replace that
evidence. The close timeout and the admitted cancellation-drain policy must
also agree. A session clone inside a call does not grant authority to close
other users' Consumer or Producer work. These are qualification cases for
every session-based alternative, including an epoch owner.

## Minimal reproduction and limits

The complete disposable probe is below. It uses only public SDK APIs and never
implements an engine binding or fabricates an engine request. Every receiver
reaches disconnection and is destroyed before the next query starts. All 201
queries finish before the 30-second SDK timer; one warmup is outside the delta.
The local case has no matching queryable. The network case uses two sessions
connected through a loopback TCP socket and checks exactly one successful reply
per query.

Rust 1.99.0, x86_64-unknown-linux-gnu, System allocator, locked default Zenoh
features, with multicast/gossip disabled:

| Mode | Live bytes after warmup | After 100 more | After 200 more | After client close |
| --- | ---: | ---: | ---: | ---: |
| Local | 64,959 | 115,551 | 166,175 | 59,599 |
| TCP round trip | 458,011 | 508,659 | 559,283 | 155,850 |

The local increments are 50,592/101,216 bytes; TCP increments are
50,648/101,272. These are process-wide requested live heap bytes, not an exact
task Layout, an engine account measurement, a leak claim or a universal SDK
footprint. TCP close also retires transport storage while the server remains
live. Source inspection identifies the retained timeout tasks; the experiment
falsifies the claim that sequential terminal query receivers imply bounded
one-call physical retention. The tasks may eventually expire normally. That
does not supply a reusable capacity reservation in the current binding. A
separate, bounded epoch reservation can cover these tasks until whole-epoch
retirement without observing each task individually.

### Probe source

```rust
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicI64, Ordering},
    time::{Duration, Instant},
};
use zenoh::{Wait, query::ConsolidationMode};

struct Meter;
static LIVE: AtomicI64 = AtomicI64::new(0);
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            LIVE.fetch_add(l.size() as i64, Ordering::SeqCst);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size() as i64, Ordering::SeqCst);
        unsafe { System.dealloc(p, l) };
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            LIVE.fetch_add(n as i64 - l.size() as i64, Ordering::SeqCst);
        }
        q
    }
}
#[global_allocator]
static METER: Meter = Meter;

fn drain(session: &zenoh::Session, network: bool) {
    let replies = session
        .get("wp600/query-tail")
        .consolidation(ConsolidationMode::None)
        .timeout(Duration::from_secs(30))
        .wait()
        .unwrap();
    let mut count = 0;
    loop {
        match replies.recv_timeout(Duration::from_secs(1)) {
            Ok(Some(reply)) => {
                assert!(reply.result().is_ok());
                count += 1;
            }
            Ok(None) => panic!("query did not finalize within one second"),
            Err(_) => break,
        }
    }
    assert_eq!(count, usize::from(network));
}

fn main() {
    let network = std::env::args().any(|a| a == "--network");
    let started = Instant::now();
    let mut config = zenoh::Config::default();
    config.insert_json5("mode", "\"peer\"").unwrap();
    config
        .insert_json5("scouting/multicast/enabled", "false")
        .unwrap();
    config
        .insert_json5("scouting/gossip/enabled", "false")
        .unwrap();
    config.insert_json5("listen/endpoints", "[]").unwrap();
    let mut server = None;
    let mut queryable = None;
    if network {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("tcp/127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);
        let mut server_config = config.clone();
        server_config
            .insert_json5("listen/endpoints", &format!("[\"{endpoint}\"]"))
            .unwrap();
        let s = zenoh::open(server_config).wait().unwrap();
        queryable = Some(
            s.declare_queryable("wp600/query-tail")
                .callback(|query| {
                    query.reply("wp600/query-tail", "ok").wait().unwrap();
                })
                .wait()
                .unwrap(),
        );
        config.insert_json5("mode", "\"client\"").unwrap();
        config
            .insert_json5("connect/endpoints", &format!("[\"{endpoint}\"]"))
            .unwrap();
        server = Some(s);
    }
    let session = zenoh::open(config).wait().unwrap();
    std::thread::sleep(Duration::from_millis(500));
    // Every query receiver is fully drained and destroyed before the next starts.
    drain(&session, network);
    std::thread::sleep(Duration::from_millis(100));
    let base = LIVE.load(Ordering::SeqCst);
    for _ in 0..100 {
        drain(&session, network);
    }
    std::thread::sleep(Duration::from_millis(100));
    let first = LIVE.load(Ordering::SeqCst);
    for _ in 0..100 {
        drain(&session, network);
    }
    std::thread::sleep(Duration::from_millis(100));
    let second = LIVE.load(Ordering::SeqCst);
    assert!(started.elapsed() < Duration::from_secs(30));
    session.close().wait().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let closed = LIVE.load(Ordering::SeqCst);
    println!(
        "network={network} base={base} after100={first} after200={second} closed={closed} delta100={} delta200={} recovered={}",
        first - base,
        second - base,
        second - closed
    );
    if let Some(q) = queryable {
        q.undeclare().wait().unwrap();
    }
    if let Some(s) = server {
        s.close().wait().unwrap();
    }
}
```

### Reproduction command

Run at the repository root. The command extracts only the probe above into a
temporary directory, builds the existing backend with the repository lock and
discovers the exact SDK rlib from Cargo's artifact messages. It writes no
repository source or lockfile.

```sh
CARGO_INCREMENTAL=0 python3 - <<'PY'
import json, pathlib, subprocess, tempfile
topic = pathlib.Path("workspace/0083-host-zenoh-consumer-query-ownership.md")
source = topic.read_text().split("\x60\x60\x60rust\n", 1)[1].split("\x60\x60\x60", 1)[0]
build = subprocess.run([
    "cargo", "build", "--locked", "--offline", "-p",
    "clinkz-wot-protocol-bindings-zenoh", "--message-format=json",
], check=True, text=True, stdout=subprocess.PIPE)
artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith("{")]
sdk = next(pathlib.Path(p) for a in artifacts
           if a.get("reason") == "compiler-artifact" and a["target"]["name"] == "zenoh"
           for p in a["filenames"] if p.endswith(".rlib"))
with tempfile.TemporaryDirectory(prefix="wp600-query-tail-") as folder:
    folder = pathlib.Path(folder)
    rs, binary = folder / "probe.rs", folder / "probe"
    rs.write_text(source)
    subprocess.run(["rustc", "--edition=2024", str(rs), "-o", str(binary),
                    "-L", f"dependency={sdk.parent}", "--extern", f"zenoh={sdk}"], check=True)
    subprocess.run([str(binary)], check=True)
    subprocess.run([str(binary), "--network"], check=True)
PY
```

## Bounded reusable-session discriminator

[The independent review's disposable reproducer](https://github.com/yushun1990/clinkz-wot/pull/174#pullrequestreview-5515584302)
uses the probe's allocator, drain function, configuration and locked SDK. A
local reproduction at `bb723a6fd88d401d4f10a8b3655e0df5e5609a27` completed 1,024
reads in each local/TCP mode across 32 Consumer epochs, all before the first
30-second timeout. Each epoch starts with K=32 submission credits. A credit is
consumed before SDK get and never restored on a receiver result or deadline.
Ten attempts at saturation return the identical test input without another get.
The next epoch opens only after successful close and destruction of the previous
session. The separate Producer queryable remains usable after all TCP epochs.

| Mode | Closed live bytes at epoch 15 | At epoch 31 |
| --- | ---: | ---: |
| Local | 48,512 | 48,512 |
| TCP | 145,070 | 145,070 |

At most K queries are submitted per epoch, so the inspected path can create at
most K timeout tasks even if none expire. Whole-epoch retirement can therefore
acknowledge those tasks together; per-query SDK joins or an SDK correction are
optional alternatives. The measurements are process-wide requested live bytes,
not a production byte ceiling. This discriminator supplies no callback/job
quiescence, hostile-input bound, recoverable backing, registration shutdown or
final-root residual proof. Those obligations remain open for production.

## Necessary decision and narrower alternatives

Decide the concrete **query plus session-work owner**, its supported deployment
configuration and its actual release barrier before writing a production
adapter. Preserve the accepted request/seal/first-cause/lease/cleanup contracts.

| Alternative | Assessment |
| --- | --- |
| Wrap the legacy transport in HostBindingCall | Rejected: retains TD/Form re-entry, blocking execution, infallible variable backing and no full query-work owner. |
| Use SDK get/open `await` and drop the future/receiver on cancellation | Rejected: eager synchronous resolution and retained session tasks remain. |
| Limit active calls to one, cap FIFO items, shorten timeout, or add a grace sleep | Rejected as the ownership proof: repeated terminal calls retain overlapping timer work; elapsed time is not acknowledged destruction. Shorter timeout alone supplies no declared maximum outstanding work or admission enforcement. |
| Retain one reusable Consumer session epoch with K submission credits; debit before each SDK get, backpressure at K and replenish only after complete epoch retirement | Credible narrower candidate, supported by the discriminator above. SDK tail credits remain consumed across successful reads and deadlines. Retain actual registration/epoch ownership and original accounts through session-task, callback and reactor-job quiescence. Physical byte/input bounds, opening/rotation, failed or abandoned close and finite registration shutdown/Drop still require qualification. No per-query retirement hook or SDK change is inherently needed. |
| Own an exclusive session with each call; use a declared bounded protocol reactor for synchronous SDK work and await complete retirement | Alternative equivalent to an epoch with K=1. It also needs callback/job quiescence, truthful close failure/residual and physical bounds; measure per-read connection cost. The evidence does not establish that every read needs its own session. |
| Retain a reusable session with bounded permits returned on individually acknowledged query/task retirement, possibly via a pinned SDK correction | Optional finer-grained alternative if it supplies actual retirement and enforceable input/byte bounds. A fabricated side table or polling a deadline is insufficient. Compare its SDK/dependency cost against whole-epoch retirement; it is not a requirement for all reusable owners. |
| Relax boundedness or treat SDK memory/tasks as outside binding accounting | Rejected under current authority. That would require an explicit architecture decision and affected evidence review. |

A bounded reactor may advance protocol I/O and wake the engine; it receives no
TD, selector, handler dispatch or publication authority. Naming a worker does
not establish complete ownership, recoverable construction, a join barrier or
reserved capacity. Compare these candidates before choosing one. The concrete
decision must identify which state is call-owned, registration-owned, shared
SDK infrastructure or residual, who charges each physical allocation, and what
acknowledges its final release. A shared-session close must never serve as a
per-call cancellation mechanism affecting a sibling role. Epoch submission
credits and their SDK backing are distinct from the Runtime's reusable semantic
call opportunity: terminal call acknowledgement may free that opportunity while
the original registration/epoch charge stays held. A candidate must bind both
lifetimes to actual owners and prevent new epoch admission until full retirement.
The current Runtime retires its registration through bounded destruction; an
epoch design must demonstrate registration shutdown and root-abandonment within
that boundary or return an explicit decision before changing the shared SPI.

## Smallest production envelope after resolution

One Host-only complete Consumer registration, one eager consumed generation,
one active call/result opportunity, fixed finite endpoint/key/URI/payload/media
limits and repeated reads after verified terminal release. Read two properties
using non-first original Form coordinates after destroying the source Thing.
Compiler input is the already selected/resolved candidate; execution receives
only its owned artifact and OutboundRequest. No raw application invocation,
legacy selection, execution-time Thing/Form scan or second result validator is
acceptable.

Promote only the necessary Host Producer Property Read route/permit/response
implementation into production for a Consumer-to-Producer TCP round trip; keep
the historical Host/static seven-case probe as regression evidence. A
test-local target server or raw SDK responder can isolate SDK behavior but
cannot satisfy the final production loop. Target paths may replace obsolete
Property Read edges directly; preserve disjoint legacy operations until their
own admitted migration. Broad constructor/client/server or subscription removal
is not required by this investigation.

The eventual ADR-0013 candidate must close requirements, authority/API/state/
resource projections, concrete source/removal paths, host feature applicability,
configuration identity and exact predecessor closure in `index.toml`.
Production changes should remain in the Zenoh package and a downstream
normal-library application/test fixture. Shared engine/TD/Planning changes,
new public support issuers, subscriptions, zenoh-pico runtime parity, fleet
claims, broad WP-600 completion and new architecture gates are outside this
envelope. A discovery requiring them returns to decision/admission review.

Reserve the following prospective completion keys at
`docs/evidence/WP-600-host-zenoh-consumer-property-read.toml`; no results or
passing state exist for them:

| Key | Falsifiable completion evidence |
| --- | --- |
| `zenoh-host-consumer-public-path` | Real complete production client, production compiler/artifacts and public Runtime; source destruction, two exact selected targets/original coordinates, Core-sealed body/media/status, repeated reads and production Producer TCP loop. Negative identities reject before effects; inspect zero target-to-legacy edges. |
| `zenoh-host-query-owner-retirement` | Account for every query, callback, timeout task, receiver, reactor job, session and result owner. Retirement may be acknowledged per query or for a complete bounded epoch. Run at least 1,000 sequential reads within one SDK timeout window, including prompt finals and absent/late peers. For an epoch, consume K credits before SDK submission, never refund on reply/deadline, return unchanged input at K with zero additional starts, and replenish only after observed full epoch retirement including callback/job quiescence. Bound physical growth and retain original registration/epoch accounts independently of terminal call acknowledgement. Explicit shutdown and root abandonment preserve truthful residual/account ownership and child-before-parent release. |
| `zenoh-host-call-cancel-cleanup` | Before-first-poll, pending, completion race, caller/call/handle/root drop, immediate/polled late completion, drain expiry, shutdown failure and residual. First cause is immutable; normal/late success uses the installed Core seal; settlement and destruction occur once. Any transfer moves the whole decorated call and preserves the unused seal; refusal returns the identical owner. |
| `zenoh-host-sdk-resource-qualification` | Concrete constructor/failed-construction/result/cancel/destructor audit, recoverable backing, finite supported input capacities and SDK configuration. Record actual simultaneous live/peak/largest allocations, queue/task/callback bytes and scratch separately from maxima. Disabled/minus-one/exact/plus-one limits, oversized reply and saturation reject or reach truthful cleanup without hidden memory or blocking engine polls. Test shared-parent exhaustion with two otherwise separate roots. |
| `zenoh-host-producer-regression` | Production Host route carrier survives prepare/active/committed stages, exact permit gates acceptance, correlation survives response, and drain/readiness failure/prepublication cancellation reach truthful complete or residual cleanup. Reproduce the existing seven-case probe unchanged; report its static scope only as historical regression. |
| `zenoh-host-feature-and-author-boundary` | External normal-library application links public packages, no private/unsafe engine access or mock transport; audit optional dependency direction and supported Host registration. No-default and async-no-std remain compile regressions, not real pico qualification. Record mandatory declarations/workarounds and concrete callback limits; an unsupported profile fails before protocol work. |

Existing workload identities `PERF-GW-009`, `PERF-GW-028`,
`PERF-GW-029` and `PERF-GW-031` cover the affected Host metadata, invoke
cancellation, generation retention and registration dimensions. Preserve their
fixture/budget identities and run the existing manifest verifier before
admission. The SDK-tail discriminator is additional ownership evidence, not
completion of those broad workload matrices. Measure session reuse/rotation and
reactor costs separately; no latency or allocation budget is invented or relaxed
here.

## Checks and independent review boundary

Applicable existing prechecks are the nineteen unchanged Consumer-gate commands,
the seven-case real Producer feedback test, Zenoh no-default/async-no-std compile
cells, the original probe, the bounded-epoch discriminator and Git diff hygiene.
Authority checking already covers API/ADR/state/resource/legacy projections
and the 66 existing performance
manifest cases. No gated legacy smoke test reported as skipped is real-protocol
acceptance.

The PR must report exact revisions, commands and actual outcomes. These checks
can reaffirm predecessors and reproduce the blocker; they cannot pass the
unimplemented production keys. Independent review should challenge the task-tail
interpretation, the bounded-epoch, per-read and per-query alternatives, and every
missing resource/release mechanism. A concrete ownership contract and its
pre-code checks must be defined and migrated into applicable owners before a
docs-only production candidate receives exact-revision admission review and a
separate admission transition. Actual production measurements and the six keys
above remain post-implementation completion evidence under ADR-0013. This
investigation neither self-approves that decision nor changes any admitted
predecessor.
