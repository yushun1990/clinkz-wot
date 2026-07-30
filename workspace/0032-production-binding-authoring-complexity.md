# 0032 Production Binding Authoring Complexity

Status: MIGRATED

Kind: owner-raised API and extension-constructibility investigation

Priority: HIGH

Target: the public Protocol Binding registration and execution SPI as experienced by real third-party Zenoh, zenoh-pico, and future protocol authors

## Scope and authority

This topic records a Project Owner concern that the complete registration, route lifecycle, generation, resource, permit, response, and cleanup contracts may be type-correct yet too difficult for practical third-party Binding authors. It does not prescribe a simpler SPI or weaken the ownership model. Codex owns the technical decision.

## Repository observations

- A complete Binding bundle carries identity, configuration, capabilities, compiler compatibility, execution components, profile cells, footprints, ingress, readiness, status, overflow, reactor, and cleanup declarations.
- The route contract includes prepare, readiness, activate, commit-to-closed, permit-gated accept, response delivery, abort, shutdown, and cleanup.
- Current constructibility evidence is based on host/static mock authors.
- Production-author usability and real Zenoh/zenoh-pico integration remain WP-600 evidence.
- A Binding crate must not depend on Servient or receive hidden dispatch authority.

## Questions for investigation

1. What concepts must every simple server-only Binding author understand and implement?
2. Which declarations can be derived, safely defaulted, generated, or grouped without hiding ownership or resource truth?
3. Do the current mock authors exercise protocol-reactor, transport-buffer, correlation, ingress, and cleanup difficulties found in real bindings?
4. Should a non-authoritative Zenoh authoring spike run during WP-300 implementation to expose concrete defects earlier?
5. Which API failures would justify reopening the frozen SPI rather than adding helpers?
6. Can official helper layers remain outside the normative core while preserving third-party portability?
7. What evidence distinguishes public constructibility from acceptable production-author ergonomics?

## Constraints

- Do not move protocol syntax, transport state, or hidden dispatch into Core or Servient.
- Do not infer missing lifecycle, cleanup, or resource declarations through unsafe defaults.
- Do not require a production Binding implementation before the currently admitted narrow source slice can begin.
- Reopen public contracts only on concrete ownership, portability, resource, or implementability evidence.

## Expected decision output

Codex should define the production-author evidence required at WP-300 and WP-600, decide whether an early protocol-author spike is warranted, identify safe helper or generation opportunities, and migrate any stable usability requirements to the Binding SPI, work packages, fixtures, or examples.

## Decision

The reviewed narrow WP-300 public mock authors prove constructibility, not
production ergonomics, and remain sufficient to begin that exact source slice.
A bounded, non-authoritative Zenoh authoring spike should follow narrow WP-300
completion and precede the broad WP-300 candidate. It uses public crates only,
implements one server-only Property Read registration, and records concepts,
boilerplate, generic/code-size pressure, protocol-reactor integration,
correlation, ingress, and cleanup friction. It grants no WP-600 source
admission or completion credit.

Every binding author must still declare identity/compatibility, advertised
capabilities, complete compiler/execution pairing, preparation/ingress policy,
retained/transient footprints, route progress, response ownership, and cleanup
outcomes. Helpers may group those declarations, generate application-closed
static enums, or provide explicit synchronous/no-successor adapters. They may
not infer protocol state, omit applicable resource limits, detach work, or
hide a lifecycle phase. Official helper layers remain conveniences over the
public SPI rather than a second normative extension mechanism.

The public SPI is reopened only when the spike or a production backend shows a
concrete ownership loss, impossible portable representation, unaccounted
resource, unimplementable lifecycle, or a required unsafe/private dependency.
Repetitive but mechanically generatable declarations justify helpers, not a
semantic relaxation.

WP-600 must then prove both public construction and production operation for
Zenoh and zenoh-pico, including reactor/transport storage, correlation,
ingress, cleanup, feature cells, and realistic end-to-end Property Read.

## Migration

The evidence split, early-spike boundary, helper rules, and reopening predicate
are projected into `docs/spec/binding-spi.md`,
`docs/work-packages/WP-300-bindings.md`, and
`docs/work-packages/WP-600-protocol-bindings.md`. This topic is `MIGRATED`.
