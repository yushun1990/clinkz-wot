# Security composition: scoped implementation evidence

## Architectural correction

The initial Basic/HTTPS increment's static singleton requirement was an
implementation limit, not a usable composition contract. It is replaced with
bounded owned immutable SecurityRequirement composition and explicit complete
Consumer conjunction selection. Core contains no HTTP credential, TLS, routing
or verifier cases. Its value can be lent through PR #271's ValueExecution without
Arc, thread, runtime or Host ownership requirements. Existing Host services and
opaque grants retain their own supported ownership representation.

The normative owners are [TD interpretation](../spec/td.md),
[Core security](../spec/interaction-core.md),
[Planning selection and qualification](../spec/planning.md#selected-executable-security-preparation)
and [finite Binding collaboration](../spec/binding-spi.md#finite-security-verification).
[HTTP](../../protocol-bindings/protocols/http/README.md) owns its executable
placements, principal composition, protocol limits and public construction APIs.
[The investigation](../../workspace/0118-security-composition.md) records alternatives
and the independent pre-implementation challenge. Existing architecture already
requires composition and independent authority; this correction adds no universal
manager, transport/session IR or new governance gate.

TD's Combo fields now preserve absence versus explicit empty arrays. Shared
synchronous/resumable validation rejects missing/both operators and cardinality
below two. Valid corpus coverage now uses separate valid oneOf/allOf definitions;
the old fixture accepting both is intentionally invalidated. Cycles and excessive
expansion refuse preparation before selection, including when an earlier anonymous
branch could otherwise appear usable. Unavailable interpretation remains a branch
that cannot certify Producer support.

## Executed subset and evidence

`protocol-bindings/protocols/http/tests/security_composition.rs` uses the public
prepared Engine Consumer and completely qualified Producer, real TCP/TLS I/O,
independent Basic/API-key verifiers, application authorization and original account
owners. It exercises:

- Effective conjunctive lists, Combo allOf/oneOf, nesting and Form replacement;
  API-key-only execution with no Basic verifier; independent service/native pairing.
  An omitted credential name remains absent in shared meaning while actual HTTP
  placement supplies Authorization, avoiding an HTTP default in generic preparation.
- Consumer choice of a supported branch alongside unsupported alternatives, with
  full Producer refusal for the same advertised expression; missing components,
  duplicate placements, indistinguishable alternatives and static header conflicts.
- Independent TLS peers supplying wrong, partial, extraneous or conflicting
  authentication evidence; no weaker/anonymous fallback and no behavior dispatch.
- HTTPS NoSec with no authentication service, still subject to application policy.
- Cross-verifier callback invalidation before behavior and successful delivery;
  effectful conjunction Write retaining known completion after API-key revocation.
- API-key rotation, independent instances sharing selected meaning, cancellation,
  retained original funding and final release; cyclic/expansive whole-expression
  refusal through public preparation; mutated typed scheme discriminators cannot
  turn advertised authentication into NoSec. Conjunction Write includes both a
  complete successful exchange and revoked delivery after completed effects.
- Independent acceptance counterexamples: an earlier verifier increasing the next
  verifier's work must refuse before that callback; proxy/extension Combo metadata
  cannot hide cycles or excessive expansion. Both reproductions failed before
  their corrections. Valid unavailable containers still permit Consumer selection
  of another complete branch, while retaining full Producer refusal.

The existing `security_https` and `security_write` suites continue to exercise
certificate trust/identity refusal, exact audience/operation/registration and
incarnation binding, policy changes inside callbacks, wrong/missing/revoked Basic
credentials, replacement, late native outcomes, lost responses, budget refusal,
authorization and original resource cleanup. Their singleton-only negative Combo
and HTTPS NoSec assumptions are superseded. Structured data, protocol-native
finite outcomes and continuing Action/notification owners retain their scopes.

SecurityRequirement's changed representation is checked in the existing caller-
owned fixture by mechanically updating its deliberately unsupported Basic value;
no authenticated Embedded execution or new Embedded work is added. Workspace,
independently linked Consumer, supported feature-cell and design authority checks
verify integration. The PR records exact tested revisions and command outcomes;
no fast diff-hygiene CI result substitutes for these execution claims.

## Limits and intentionally rejected alternatives

Supported HTTP branches are NoSec, Basic/Authorization, API key/X-Api-Key, and
Basic AND API key. Distinguishable alternatives between these are supported for
finite HTTPS Property Read/Write. Each conjunction requires identical independently
verified identity and claims; multi-principal aggregation and privilege union are
intentionally rejected. Different named credentials sharing one placement are
not distinguishable verifier domains in this implementation. Failure never retries
another branch or refreshes original work. TLS trust, authentication and application
permission are independent, with exact audience grants and non-reused invalidation.

General OAuth2/Digest/Bearer/PSK/extension/Auto mechanisms, proxy authentication,
other placements or header names, requested TD scopes, challenge/refresh workflows,
authenticated collections/Actions/notifications, DNS/mTLS/CRL/OCSP and authenticated
constrained execution remain unsupported. Standard unsupported scheme operands are
retained for qualification, and uninterpreted extensions remain explicitly
unavailable. Preservation is not execution support. Bounded processing can refuse
otherwise valid larger descriptions; it never truncates them.

No whole Security protocol/conformance, deployment resource ceiling, production
security, architecture/milestone closure or v1 acceptance is claimed. No benchmark
or throughput target determined this design. Prior Basic and finite lifecycle
invariants remain obligations with the explicitly superseded restrictions above.
