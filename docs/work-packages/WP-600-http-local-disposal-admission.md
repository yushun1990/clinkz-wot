# HTTP dependency local disposal: ADR-0013 admission candidate

`WP-600-HTTP-LOCAL-DISPOSAL` is a **candidate**, not an admitted or completed
tranche. [The registry](index.toml) owns that state. Independent exact-revision
review and a separate admission decision are required before production changes.
The disposable patch is an executable precheck, not an installed SDK correction.

## Decision and smallest scope

The unchanged Hyper 1.12.0 client cannot satisfy the existing allocation-free
last-root disposal contract. Its queued-request, pending-response and incomplete
body destructors allocate errors even after all local observers are gone.
Reversing the local owner order alone does not fix this. The
[pinned-source discriminator](../../tools/architecture-fixtures/http-consumer-admission/README.md)
owns the exact dependency/configuration identity, source references, measured
counterexamples and disposable repair results.

Select one narrow **dependency source correction**, retaining ADR-0022's HTTP
direction and direct call ownership. Install the exact reviewed Hyper archive
with only the three-file `closed-interest.patch`: suppress construction of an
error only when its receiver is already closed. Preserve error delivery to live
observers and the existing public API. Future call disposal must destroy response
interests before the connection. This does not introduce a generic retirement
SPI, another architecture gate or a universal SDK qualification package.

The ownership and evidence boundary differs from the HTTP adapter: the correction
can be verified without an engine registration, TD/compiler amendment or target
adapter. The unchanged dependency already falsifies a stable requirement; this
is not a demand to measure nonexistent production code. Separating this one
repair permits independent review of third-party error/disposal semantics before
admitting the larger cross-crate Consumer change. It does not require successive
precursors for every HTTP implementation detail.

| Alternative | Disposition |
| --- | --- |
| Unmodified Hyper plus careful field order | Reproduced allocation in both orders; insufficient. |
| Permit this allocation as SDK I/O under ADR-0022 | Contradicts the expressly preserved final-root contract. No failure-scope relaxation is proposed. |
| Poll to completion during final Drop | Requires progress after root loss and can allocate or wait for the peer; not finite prepaid local discard. |
| Leak, detach or move the connection to a worker | Does not establish physical retirement or release of original charges; adds owners rather than correcting this destructor. |
| Replace Hyper, add a high-level client or change Runtime ownership | No evidence that this small private-source defect warrants those larger changes. The direct unspawned TCP path works. |
| Fold the unqualified dependency into an adapter-only admission | Hides a demonstrated prerequisite outside that stated source scope. A later combined scope would have to explicitly own and independently review the correction. |

## Reconstructed authority and predecessor impact

The reconstruction basis is fetched master
`47f209575b830b3a86046f712a3deddfc80edf0e`, including merged PR #177 and its
independent reviews. [ADR-0013](../ADRs/0013-work-package-scoped-implementation-admission.org)
controls admission; [ADR-0022](../ADRs/0022-first-production-binding-and-runtime-environments.org)
selects HTTP and explicitly leaves exact dependency eligibility to admission.

[Foundation's Host failure scope](../spec/foundation.md#host-allocation-and-environment-failure-scope)
allows bounded SDK allocation in paid I/O progress, while preserving controlled
construction and cleanup. [Binding SPI](../spec/binding-spi.md#shared-protocol-infrastructure-lifetime)
requires finite nonallocating final local disposal and complete retirement.
[Runtime lifecycle](../architecture/50-servient-runtime-lifecycle.md#consumed-thing-lifecycle)
owns the final residual/account sequence. Actual
[`Root::drop`, `Driver::drop` and `CallOwner::drop`](../../servient/src/consumer_property_read/host.rs)
destroy the raw call and registration before releasing original capacity; there
is no asynchronous registration-retirement hook to assume instead.

The exact completed predecessor is
`WP-400-CONSUMER-PROPERTY-READ-RUNTIME`, including its transitive ten-tranche
Consumer closure and registered evidence. The passed
[Consumer gate](consumer-property-read-architecture-gate.toml) proves those
mock/controlled-owner semantics, not this SDK. The passed Producer gate remains
regression evidence. This candidate changes none of their code, statuses,
requirements, declarations or claims; their evidence remains current within its
original scope. WP-000 is the complete package prerequisite; WP-600's broader
WP-300 package dependency and package completion remain unchanged.

Topics 0081/0082 concern multi-Thing reuse and subscriptions; 0076's broader
runtime/image proposals and 0083's deferred Zenoh qualification do not intersect
this private HTTP dependency correction. The bounded investigation
[0084](../../workspace/0084-http-consumer-local-disposal.md) is migrated here.
No unresolved shared-architecture choice is resolved by this candidate.

The production compiler path was also reconstructed, rather than inferred from
the gate. `td/src/validated.rs::ValidatedPropertyReadForm` lends paid resolved
URI, content, coding, subprotocol, scope and security facts, but not Form
extensions or response overrides. `planning/src/consumer_property_read.rs`
materializes owned coordinates and runs every actual compiler bound before any
start. Its `BindingCompilerInput` currently has only plan, candidate and role.
`core/src/binding_compiler.rs` and `core/src/binding.rs` admit the private,
configured `ResolvedTargetCompiler`/Host adapter, not an arbitrary HTTP compiler;
its fixed artifact owns at most 64 target bytes and supplies no HTTP semantics.
The legacy Zenoh client and raw SDK peer are not substitutes for that path.

Consequently, dependency repair alone cannot justify full HTTP admission. The
target still needs the minimum paid protocol-neutral metadata/support projection,
including rejection of explicit method/header/response instructions and unknown
metadata. HTTP-specific interpretation cannot move into shared Planning or be
recovered by a runtime TD rescan. It must also audit the pinned client's separate
head/trailer count/byte, read-buffer capacity and body limits, simultaneous
buffer ownership, and inner-loop/per-poll work. The builder settings in the
fixture establish only its configuration: `max_buf_size` has an 8192-byte
minimum, `read_buf_exact_size` replaces that setting, and a delivered-body limit
does not cap pre-body storage. These are later **pre-admission** declarations,
not work that can be deferred to a production-completion claim. Actual adapter
Layout/peak measurements and public consume/read/seal/account execution follow
implementation under its admitted declarations. The target owns those details;
this predecessor invents neither their API nor provisional byte budgets.

## Exact implementation and projections

Only after admission:

- Extract the checksum-identified Hyper 1.12.0 archive into
  `protocol-bindings/dependencies/hyper-1.12.0`, preserving every original file,
  license and provenance, and apply the exact three-file fixture patch. Do not
  silently upgrade versions or change parser features. Put any local provenance
  note in `protocol-bindings/dependencies/README.md`, outside the verified source.
- Exclude that vendored dependency from root workspace membership. Extend the
  existing standalone fixture's installed-source check if needed, retaining the
  upstream counterexample and repaired positive/control cases. Its locked graph
  must select exactly one installed Hyper, with all other locked identities
  unchanged. Do not add an unused root dependency or a placeholder HTTP crate.
- Register the actual source owner in WP-600 inventory when its manifest exists.
  The later HTTP admission must consume this exact source/feature disposition;
  adding it to the application's Cargo graph is not permission to execute an
  unadmitted adapter.

The only upstream behavioral edits are closed-interest guards in
`src/client/dispatch.rs` and `src/proto/h1/dispatch.rs`, plus the private visibility
of the existing closed-receiver query in `src/body/chan.rs`. No engine source,
Core issuer, portable compiler, TD lending, Planning barrier, Servient callback,
public API, state transition, error taxonomy or old-API removal changes belong
to this predecessor. Therefore the existing API matrix and state machines need
no new rows. `owner_packages = ["hyper"]` names prospective third-party source,
not an existing installed ClinkZ binding.

The executable cell is Host `std`, HTTP/1 client, one connection/request and no
client task or shared pool. No no-std HTTP, TLS, Producer or firmware claim is
made. Shared portable cells remain regression obligations.

Resource projection: no new engine resource row, default, profile, WorkClass,
ledger, external owner or budget. The repair adds no storage or allocator and
removes allocations in the qualified discard order. Existing heap fields still
release normally; source retains no abandoned connection or job. Observer-first
disposal leaves no other body receiver contending for its channel mutex.
Source review plus the fixed local destructor graph supplies the finite-work
argument; elapsed time alone is not that proof. Actual adapter Layouts, receive
growth/overlap and paid poll limits are outside this predecessor and must be
declared before HTTP adapter admission. There is no applicable new performance
workload: this tranche does not implement an interaction or change a registered
workload/fixture/budget. Existing performance contract validation still runs;
measurements are not inferred from it.

## Executable prechecks and independent review

The registry names the runnable commands. The central precheck is:

```sh
python3 tools/architecture-fixtures/http-consumer-admission/run.py
```

It verifies the archive/registry source and exact dependency resolution, applies
the repair only in a temporary directory, and runs all eleven public-SDK cases
against both versions. It covers ordinary/retry queued and dispatched requests,
incomplete bodies with held data, both owner orders, live error observers,
idle/completed controls, actual TCP closure and two independent target reads
without a spawned client connection driver. Fixture format, authority checks,
the registered Consumer regressions including both ARM compile cells, and
Producer regressions are separate checks; none changes gate status.

This author supplies no independent acceptance. Review must assess the exact
diff, source identity, skip-only-after-closed semantics, live-observer controls,
disposal order, finite local ownership and whether this predecessor is narrower
than a shared Runtime change. Passing prechecks alone do not admit the tranche.

## Completion obligations after admission

Results belong to `docs/evidence/WP-600-http-local-disposal.toml`, using the
stable keys registered in `index.toml`:

1. `http-local-disposal-source`: the installed source is the exact locked archive
   plus the reviewed patch, including license/provenance and an explicit Cargo
   disposition. No parser, public API or unrelated source edits. The compiled
   fixture resolves that installed path once, not the unmodified registry copy.
2. `http-local-disposal-observers`: the same public-API cases pass against the
   installed dependency. Every complete-owner discard has zero allocator
   requests in the qualified order; each I/O owner releases once; the TCP peer
   observes local closure. Live response/body observers still receive their
   errors, already accepted data is preserved, and the reverse-order controls
   remain narrower than the allocation-free claim. Include source inspection
   of all changed Drop branches and absence of tasks, leak or replacement work.
3. `http-local-disposal-regression`: authority/format checks, unchanged Consumer
   and Producer registered commands, portable/ARM cells and dependency neutrality
   pass at the implementation revision. Report executed commands and feature
   graph. Independent completion review must cover that exact installed source.

The concrete installed-source command already exists:

```sh
python3 tools/architecture-fixtures/http-consumer-admission/run.py --variant installed
```

It is **completion-only**, not a current precheck. It cannot run successfully
until the source is installed after admission. The disposable positive cannot
be relabeled its result.

Completion releases only this dependency-disposal prerequisite. The
[HTTP Consumer target](WP-600-http-property-read-target.md) still needs its
own closed compiler/paid Form metadata projection, input/work/storage declarations,
public Runtime/selected-artifact/seal/cancellation/account proof and independent
admission. Root Cargo adoption must preserve the qualified dependency/features
or revalidate their impact; the standalone lock does not certify a different
application feature union. This candidate does not install or qualify an HTTP
binding, close WP-600, grant a gate transition or prove production/HTTPS/embedded
readiness.
