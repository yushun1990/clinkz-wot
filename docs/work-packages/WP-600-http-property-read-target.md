# Host HTTP Consumer Property Read: implementation-admission target

This record identifies the prospective HTTP ADR-0013 candidate under
[ADR-0022](../ADRs/0022-first-production-binding-and-runtime-environments.org)
and [ADR-0023](../ADRs/0023-resource-ownership-and-policy-boundaries.org).
It is **not registered or admitted** in `index.toml`; it permits no production
change. The candidate must supply its shared contract, exact source/dependency
edit scope, prechecks and independent admission review before implementation.
Concrete HTTP SDK selection, dependency lock and qualification are implementation
evidence, not a predecessor SDK qualification package.
The prospective tranche identity is `WP-600-HTTP-CONSUMER-PROPERTY-READ`.

## Scope and predecessors

First close ADR-0023's resource ownership contract/schema migration and the
applicable Consumer readmission boundary. Current strict footprint/destructor
contracts remain binding until that explicit migration; this target cannot
waive them. Use controlled owners to settle the shared boundary. Do not require
Hyper qualification or its proposed dependency disposal repair as the shared
architecture's predecessor. SDK evidence is collected during HTTP implementation.

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
- Choose the concrete client and readiness runtime during implementation. The
  application supplies required runtime services and drives the manual Runtime.
  A direct call-owned connection is the smallest entry topology. No engine-owned
  executor, detached driver, pool, redirect, compression, HTTP/2, retry or
  reconnect loop is required. A fresh connection per read is acceptable; any
  additional owner needs bounded lifetime/retirement evidence.
- The returned call owns the request, socket/connection and response progress.
  Constructor rejection returns the unchanged request; network work begins only
  after accepted ownership, on poll. Core and raw call construction keep their
  recoverable backing. No allocation after raw acceptance is introduced into
  Core's envelope/seal installation.
- During implementation, pin/audit the selected client/parser and configure
  header count, header/read-buffer, body/trailer and connection limits. Distinguish
  engine-admitted carrier/handoff capacity, binding-controlled buffers and
  deployment costs under the migrated contract. Bound pre-body receive state
  and per-poll work as well as delivered bytes. A body-size wrapper alone proves
  no physical receive maximum. Keep complete protocol futures owned; a detached
  owner is not evidence of retirement.
- Network error, deadline, cancellation and invalid/oversized output retain the
  original call/lease until local socket/future release, then report the exact
  settlement. Socket closure certifies local resource release, not whether a
  remote read executed. Retain first cause and late-result sealing. Registration
  and last-root destruction must fit the explicitly migrated finite local
  disposal contract; allocation-free engine status/handoff remains required.

Concrete SDK poll/buffer/Drop discriminators belong to Binding implementation.
Preserve existing observations without promoting them into universal SDK
eligibility rules. If implementation demonstrates a conflict with the migrated
boundary, retain the smallest reproducer and reopen only that boundary.

## Admission and completion distinction

Before admission, close the migrated shared compiler/registration/input and
resource/ownership envelope, source/dependency edit scope and applicable
work/destructor guarantees. Name existing public APIs and the minimum TD/compiler
support projection; do not invent a generic lifecycle family. Selection and
qualification of the SDK/configuration/lock follow during implementation.
Specify exact test commands and requirement/evidence mappings in the registered
candidate. Run authority checks and relevant Core/compiler/Runtime and portable
regressions. Prechecks require neither a completed production adapter nor
advance concrete SDK qualification.

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
   Check actual connection/future destruction, finite nonblocking local Drop,
   truthful residual reporting and allocation-free engine status/handoff.
   Any SDK disposal allocation follows the migrated Host policy and measured
   finite local work; no blocking joins or replacement work.
4. Actual retained/transient/peak/largest allocations with ownership scope,
   configured SDK limits, declared uncertainty, baseline reactor/OS costs and
   original-account release. Measurements are not universal physical maxima.
   Fault injection covers supported engine/raw-constructor recovery. A terminated
   SDK OOM child, if used, establishes only the declared process-failure boundary.
5. Relevant unchanged Consumer/Producer public regressions, portable Core/Planning/
   Servient cells, dependency neutrality and applicable performance identities.

This completes only `http-consumer-property-read`. Production HTTP Producer,
its bidirectional loop, DNS, HTTPS/trust/security, streaming, embedded networking,
fleet concurrency and broad package closure remain separately scoped successors.
HTTPS qualification is a production-release dependency, so a local GET demo cannot
close WP-600 or be presented as v1 production readiness.
