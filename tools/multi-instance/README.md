# Multi-instance Consumer workload

Run from the repository root, with the existing locked dependencies:

```sh
cargo build --release --locked --offline -p clinkz-wot-binding-http --example multi_instance
python3 tools/multi-instance/run.py --suite full --repeat 3 --output /tmp/clinkz-multi-instance
```

The runner requires committed sources, runs cases **serially**, and preserves
the exact revision, binary and lock hashes, environment, commands and raw JSONL.
It reverses case order on alternate replicates. `--suite smoke --repeat 1`
checks all preparation modes and both drivers quickly. `--suite baseline` is a
smaller before/after comparison. Output directories must be fresh. No dependencies
beyond Rust, Python 3 and the locked crates are installed. Linux/glibc provides
the complete OS/allocator observations.

For one case:

```sh
target/release/examples/multi_instance --population 10000 --active 0.01 --lanes 8 --slots 1 --members 16 --rounds 3 --mode targeted --driver engine --accounts independent --delay-us 0
```

`--help` lists the controls. Shared mode prepares one exact definition per native
lane and binds independent logical instances without a TD. Targeted mode prepares
one checked path/header mapping per instance while retaining one common semantic
content owner. Cold mode constructs the common content again for every mapping.
All targets are independently addressable resources on **one** listening peer;
neither resource paths nor registration counts count devices. The workload uses
Thing-root ReadAllProperties over 1–32 integer members. It checks every returned
body against its selected target.

The full suite also records expected preparation refusals at 30 and 32 members
for this exact TD shape: the semantic resolver's 32 scopes include Thing, Form
and security scopes as well as members. The 29-member case executes normally.
Refusal records require zero native compilations/requests and complete original
account rollback; the workload does not raise the resolver's capacities.

Each instance has independent authority and a destination/instance-bound mutable
permission service. Independent accounts include a real bank per instance.
Shared accounts compare a common quota domain; they preserve independent
permission but do **not** provide per-instance quotas. Physical data/access
services are adopted once and imported into actual native registrations. The
unmodified Gateway profile and per-entry work budget of 1,000,000 credits per
class remain explicit. No production limits are raised.

The current-thread runtime drives at most one call per selected native lane.
The workload's explicit queue exists only in this executable, outside the Engine;
it adds no production scheduler or retries of rejected effects. Each round
rotates a cohort of `ceil(population * active)` instances. Service latency starts
at admission; cohort latency includes waiting behind occupied lanes. Each result
is consumed and dropped before admitting its lane's next call. A warm exchange
per lane precedes measurement. Separate result-retention phases capture one
result per lane, retire roots, and check final physical/account release.

The Binding driver executes the **same** qualified HTTP artifact and Core finite
seal through `start_finite`, omitting Consumer Runtime policy, bookkeeping and
data validation. This is a measurement control, never an authorized application
entry point. It retains the same idle instances and uses the same peer, target
bodies, connection policy, queue and warm-up. Compare paired synchronous API/allocation and
wall-time measurements; subtraction and loopback timing alone do not establish
an exact instruction-level Engine cost. Synchronous API spans use elapsed time
around admission/poll, exclude waits between polls, include SDK work and possible
OS preemption, and are not exclusive thread CPU time. Idle construction/source-free binding
contains no network I/O. Native job-step measurements include the public
compilation wrapper and exclude earlier snapshot/bounds construction; enclosing
qualification includes those costs and source admission. Parse, qualification
and native-step work/traffic are recorded separately. Native-step time is nested
inside qualification time and must not be added again.

The global allocator counts successful allocation/reallocation requests, full
requested sizes and live/peak Layout bytes across the measured process. It does
not count allocator headers, fragmentation, stack or kernel buffers. glibc
`mallinfo2` reports in-use and mapped arena/mapping bytes; `/proc/self/status`
reports RSS and the process lifetime RSS high-water mark. These differ from each
other and from Engine reservations. Peaks reset per phase for requested heap
only. Sources, fixtures, bank provisioning and harness vectors are explicit
phases; telemetry/OS sampling occurs after each snapshot. Small lazy Tokio/stdio
allocations may survive final Engine release. The peer runs in a separate
process with its own allocations/stacks, and reports actual request and
concurrent-response counts. Its response delay is a workload input, not a latency
guarantee. RSS is sampled, not a continuous phase peak.

Source destruction, expected-body fixture destruction and load-driver vectors
have separate phases. Final result-release timing excludes the subsequent scan
that verifies all 196 account kinds for every bank. Nested cost `net_live_bytes`
is the sum of interval net changes (for example, owned parsed Things before
later destruction), not a concurrently retained peak. Use enclosing phase live
and peak values for physical storage. Work credits are conservative allowances,
not CPU instruction counts.
The admitted phase includes the caller's preallocated `ConsumerSettlement`
carriers as well as call handles. The carrier size is recorded in configuration;
it is actual heap in this workload, independent of the runtime's vacant slots.

Generate a median CSV, including the measured ranges for throughput:

```sh
python3 tools/multi-instance/summarize.py /tmp/clinkz-multi-instance > /tmp/clinkz-multi-instance/summary.csv
```

The summarizer also reads `measurements.jsonl.gz` when only compressed raw data
is present. Raw per-replicate work, memory and latency observations remain
available alongside the medians.

This is a bounded Host/manual HTTP characterization, not a release benchmark,
cross-protocol fairness proof, hardware fit claim, pooling design or all-family
scalability claim. See the evidence for measured conclusions.
