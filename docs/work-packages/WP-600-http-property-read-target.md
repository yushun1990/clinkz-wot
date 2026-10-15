# Host HTTP Consumer Property Read: implementation-admission target

This record identifies the smallest next ADR-0013 candidate under
[ADR-0022](../ADRs/0022-first-production-binding-and-runtime-environments.org).
It is **not registered or admitted** in `index.toml`; it permits no production
change. The candidate must supply its exact declarations, dependency lock,
prechecks and independent admission review before implementation.
The prospective tranche identity is `WP-600-HTTP-CONSUMER-PROPERTY-READ`.

## Scope and predecessors

One complete Host Consumer-only registration, one eager consumed generation and
one active call/result opportunity through `ConsumerPropertyReadRuntime`.
Two distinct property targets in one TD prevent an adapter that ignores selected
artifacts from passing. Repeated reads reuse capacity only after terminal release.
Use the passed Consumer gate and its ten completed/current predecessors, including
borrowed TD, complete aggregate/output ownership, compiler support, Consumer-only
registration, Core call backing/seal and manual Runtime. The passed Producer gate
remains regression evidence, not a new HTTP Producer implementation.

The prospective production owner is `protocol-bindings/protocols/http`, depending
on public Foundation/Core/Planning/TD contracts and Host transport dependencies;
Servient is a dev dependency only. Add the actual crate to WP-600 owner metadata
when admitted creation supplies its manifest. No placeholder crate is needed now.

## Selected entry envelope

- HTTP/1.1 Property Read GET, bounded `application/json` response bytes, explicit
  local cleartext permission and `nosec` only. Use IP-literal endpoints initially
  to avoid a hidden blocking resolver. This is integration entry, not the full
  production networking/security claim. Other schemes, methods, encodings,
  security and operation cells reject before execution; required HTTP defaults
  are compiled once, with effective URI/security meaning still owned by TD.
- Close the demonstrated form-metadata projection gap in the same admission
  candidate: current `ValidatedPropertyReadForm` and compiler input do not expose
  form extensions/response overrides. Entry accepts default GET forms with empty
  form extensions and absent response overrides; reject other forms as unsupported
  before execution through a paid TD fact and a protocol-neutral support check.
  Reject even an explicit `htv:methodName=GET` until vocabulary support is admitted.
  Preserve extensions in ordinary TD round trips. Exact carrier/API changes and
  TD/Core/Planning source paths must be named in admission, including unchanged
  compiler/all-bounds/rollback regressions. No raw-TD callback, hidden validation
  promise from the application or binding-specific branch in shared Planning.
- Hyper low-level client connection plus Tokio socket/timer readiness. The
  application supplies the running reactor and drives the manual Runtime.
  No engine-owned executor, `tokio::spawn` connection driver, pool, redirect,
  compression, HTTP/2, retry or reconnect loop. A fresh connection per read is
  deliberately acceptable for this first bounded envelope; optimize only from
  later measured need.
- The returned call owns the request, socket/connection and response progress.
  Constructor rejection returns the unchanged request; network work begins only
  after accepted ownership, on poll. Core and raw call construction keep their
  recoverable backing. No allocation after raw acceptance is introduced into
  Core's envelope/seal installation.
- Pin and audit the actual client/parser versions. Set explicit header count,
  header/read-buffer, body/trailer and connection limits; charge actual buffer
  capacities and old/new overlap. Bound pre-body receive state and per-poll work
  as well as delivered bytes. A single `Limited` wrapper is insufficient proof.
  Keep complete protocol futures inside the call; do not build a detached owner
  to compensate for missing evidence.
- Network error, deadline, cancellation and invalid/oversized output retain the
  original call/lease until local socket/future release, then report the exact
  settlement. Socket closure certifies local resource release, not whether a
  remote read executed. Retain first cause and late-result sealing. Registration
  and last-root destruction must fit the Runtime's finite local disposal contract.

The candidate may use minimal disposable public-Hyper discriminators for poll,
buffer and Drop feasibility. These are pre-code checks, not another qualification
work package or architecture gate. If the selected API demonstrably cannot satisfy
one boundary, record the smallest reproducer and correct only that boundary.

## Admission and completion distinction

Before admission, close the exact supported compiler/registration/input envelope,
source-backed limits and work/destructor declarations, dependency/features and
engine/deployment accounting split. Name the existing public APIs used and any
minimum TD/compiler support projection; do not invent a generic lifecycle family.
Specify exact test commands and requirement/evidence mappings in the registered
candidate. Run authority checks, relevant Core/compiler/Runtime and portable
regressions, plus any small dependency feasibility probes that can run before
production exists. Do not demand a completed production adapter as a precheck.

Production completion must then demonstrate:

1. Actual immutable consume/read paths to two selected targets and repeated reads
   against a separately implemented HTTP peer, with no legacy selectors, TD
   retention, raw installed-client bypass or unowned connection task.
2. Exact limits and one over for headers/body/trailers, chunked and missing-length
   responses, malformed framing, slow/truncated/disconnected peers, and saturation
   with unchanged input. Neither Content-Length nor a count-only channel certifies
   physical input capacity. Explicitly reject unsupported upgrade/encoding behavior.
   Include form method/header/response overrides as unsupported-input cases, proving
   the new paid eligibility fact survives all-bounds checks and cannot be bypassed.
3. Caller cancel/drop, deadline/result races, invalid normal/late success and
   accepted errors through Core seals, drain/root loss and repeated final reuse.
   Check actual connection/future destruction, finite local Drop and truthful
   residual reporting without allocation, blocking joins or replacement work.
4. Actual retained/transient/peak/largest allocations, configured SDK envelope,
   baseline reactor/OS costs and original-account release. Fault injection covers
   supported engine/raw-constructor recovery. An intentionally terminated SDK OOM
   child, if used, establishes only the declared process-failure boundary.
5. Relevant unchanged Consumer/Producer public regressions, portable Core/Planning/
   Servient cells, dependency neutrality and applicable performance identities.

This completes only `http-consumer-property-read`. Production HTTP Producer,
its bidirectional loop, DNS, HTTPS/trust/security, streaming, embedded networking,
fleet concurrency and broad package closure remain separately scoped successors.
HTTPS qualification is a production-release dependency, so a local GET demo cannot
close WP-600 or be presented as v1 production readiness.
