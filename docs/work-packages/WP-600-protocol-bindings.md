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
