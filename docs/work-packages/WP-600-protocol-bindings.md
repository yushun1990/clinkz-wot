# WP-600 — Concrete protocol integration — v6.0

## Target and dependency

Continue Host HTTP/HTTPS production direction; add useful supported capabilities against successor contracts. Zenoh/pico remain optional future Bindings. Keep concrete SDK/security/transport policy in the Binding.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: SPI-COMPOSE-001, SPI-NATIVE-001, SPI-INGRESS-001, SPI-DELIVERY-001, SPI-RETIRE-001, PREP-COVER-001, EXEC-SECURITY-001.

Required evidence follows actual claims: Real peer interoperability, malformed/native input limits, publication, cancellation/late results, resource pressure and final physical release for each advertised mapping/profile. HTTP cleartext evidence is not HTTPS/security qualification; no hardware claim from Host tests.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.

## HTTP Producer data prerequisite

The existing single-route HTTP service implements the prepared type-only schema
subset from WP-200 using its actual JSON decoder and Core's value-type relation.
The source-independent native program retains the assertion. Serving requires an
actual `ProducerOutputValidator` facet, borrowed from the captured provider/program;
numeric service limits alone cannot establish it. Installation-only providers
omit this facet. Schema/encoding
failure becomes a Core validation error before native delivery and maps to HTTP
500 with JSON null. The generic shape seal remains a separate check.

Preparation refuses unimplemented schema assertions before native setup. The
former extension-only recursive schema acceptance is removed, so nested/schema
facets accepted at older revisions are not inherited as supported behavior.
The HTTP policy/compatibility revision changes with this narrower executable
contract. Core/HTTP runtime-only dependency cuts remain unchanged.

Evidence is the real-peer `producer_serving` and pure `producer_preparation`
targets, with the existing installation/funding/Consumer loop regressions. The
changed codec runs under the existing 4096-byte result limit and manual dispatch
allowance; native parser/SDK backing retains its existing Host deployment policy.
No general validation performance, constrained execution or SDK maximum is claimed.
Historical evidence keeps its original revisions and no completion state changes.

This prerequisite alone does not implement qualified publication from the v6
shared logical runtime. Complete multi-participant qualification, aggregate
admission, final endpoint/dependency revalidation and the Servient-local final-TD
commit remain required under SAFE-PUBLISH-001.

## Direct shared finite HTTP participation

`HttpFiniteBinding` implements `FiniteIngressBinding` directly over the real
nonblocking listener and shared Hyper transport. It uses none of the old
Producer installation, accept-permit, request or sealed-response ABI. The native
owner contains no authority/handler/registry. `HttpFiniteConfig` captures the
owned prepared program/schema, exact native operands, candidate and timeout;
one actual logical behavior can serve independent participants. The runtime-only
normal graph still excludes TD, Planning and Servient.

The concrete limits remain loopback/nonzero HTTP/1.1 bodyless GET, exact path/Host,
one connection/correlation, 16 headers/8192 bytes, 4096 I/O bytes per native poll
and JSON results <=4096 bytes. Readiness keeps semantic acceptance closed.
Malformed native input disposes its connection without dispatch and preserves
the listener. Request timeouts span the original input/reply. Retirement aborts
the exact connection and listener; any started write reports Unknown delivery.
Drop disposes native owners before program/services, without network waits/tasks.
An opaque native issuer and monotonic correlation fence the original reply;
numeric Binding/instance equality cannot substitute a sibling's opportunity.

Raw output also has to fit the concrete HTTP encoding. Malformed JSON, unsupported
media/coding or a body above the native 4096-byte limit refuses before any response
is queued or written, even if the Engine's result envelope is larger. The native
mapping error closes through the retained owner and settles the original value/
effect as NotDelivered. Only an original error result maps to the fixed 500/null;
a substituted error cannot certify delivery of an original successful output.

The `finite_producer` target uses actual HTTP peers and the public Consumer, with
sources destroyed before execution, shared mutable behavior, type/Unicode/exact
integer rejection, zero/short/exact credits, failed setup/cleanup, original-result
retention and socket reuse. The old funded public root and its account/transport
regressions remain restricted independent evidence. It is not wrapped by the new
path; retiring it requires replacement of its funded execution guarantees plus
the successor final-description commit. No old API becomes successor authority.
Neither successful HTTP I/O nor required output validation is qualified TD
publication. Multi-route HTTP matching, HTTPS/security, constrained execution,
aggregate capacity and release qualification retain their separate boundaries.
