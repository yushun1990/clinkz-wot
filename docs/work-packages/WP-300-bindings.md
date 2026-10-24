# WP-300 — Runtime capability boundaries — v6.0

## Target and dependency

Separate finite, notification and exposure controls. Native receiving/matching/distribution/correlation and physical retirement belong to Bindings; each logical recipient gets independent semantic acceptance. Replace wrong physical interfaces rather than wrap them.

Exact package dependencies belong to [index.toml](index.toml). Runtime and optional
preparation are distinct: dynamic preparation integration needs WP-200, while
execution-only composition does not. Package completion is not a blanket start
barrier; exact available predecessors and affected authority must support the
chosen slice under ADR-0013. No new runtime tranche is admitted by this migration.

## Authority and completion

Applicable requirements: SPI-COMPOSE-001, SPI-NATIVE-001, SPI-INGRESS-001, SPI-DELIVERY-001, SPI-RETIRE-001.

Required evidence follows actual claims: Practical downstream registration, native I/O for claimed capability, zero/many candidates, original identity/reply backing, shared-owner isolation and final-use retirement. No mandatory protocol-count or universal session gate.

Retained Property Read code/tests and historical tranches remain useful within
their recorded contracts. Their old status does not close this package under
v6. Exact successor source scope/API/state/resource changes, removals, checks and
independent review are determined from the current repository for each material
slice, not from a saved next-action list. New interaction families and Bindings
are outside this architecture migration.
