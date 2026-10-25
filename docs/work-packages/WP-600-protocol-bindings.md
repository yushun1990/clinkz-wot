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

This prerequisite does not implement qualified publication from the v6 shared
logical runtime. Its integration with the real HTTP native owner, complete
multi-participant qualification, final endpoint/dependency revalidation and the
Servient-local final-TD commit are still required under SAFE-PUBLISH-001.
