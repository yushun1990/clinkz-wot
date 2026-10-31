# TD semantics — v6.0

TD is the shared description interpreter. This domain contract replaces broad
semantic clauses formerly embedded in the Consumer admission work package;
that record now describes only its historical narrow implementation.

`WOT-SEM-001`: TD MUST preserve TD/TM structure and apply TD 1.1 operation
applicability/defaults, Form context, base/relative URI and template meaning,
security references/override/composition and DataSchema meaning once through
shared rules. Planning and Bindings MUST NOT implement independent variants.
Common semantic normalization precedes selected execution; runtime uses owned
prepared facts. Invalid description, unsupported processing profile and capacity
exhaustion remain distinguishable. Valid anonymous Things and empty affordance
names MUST NOT be rejected merely to manufacture local execution identity.

Operation applicability follows TD 1.1 Tables 5–8/11 and default-value tables,
including root collection Forms. The shared interpreter retains original
coordinates and effective operation sets; it does not choose a protocol method.
A Form's `security` overrides the Thing's security requirements. Multiple named
requirements compose conjunctively, as allOf; oneOf selects one alternative per
interaction. Producer qualification must support the retained advertised
alternatives, while Consumer selection can use a supported alternative without
downgrading required allOf members. Credentials/current permission remain runtime
facts, not preparation certificates.

Action `synchronous` is an optional TD 1.1 field: true describes a complete result
in the invocation response; absence supplies no synchronicity claim. It MUST NOT
be defaulted to true or treated as TD 2.0-only. `safe` and `idempotent` inform
policy and do not certify whether a previous uncertain attempt had an effect.

`WOT-EXT-001`: Unknown extension associations MUST survive document round trips.
The supported bounded context profile resolves vocabulary IRIs independently
of prefix spelling; local aliases and expanded IRIs MUST agree. Contexts may be
pinned/provisioned. Unsupported remote context processing MUST be reported;
it MUST NOT trigger implicit retrieval from execution. If unresolved meaning
can affect a selected mapping, refuse that candidate rather than guess a default.
Protocol vocabulary interpretation belongs to the concrete Binding. No generic
mandatory-extension marker, new protocol-matching vocabulary or Core protocol
switch is introduced. An unknown selected security scheme refuses execution;
an unused extension definition MUST NOT by itself invalidate another supported
candidate. Preservation is not proof of support.

TD structural validity is not execution support or permission. Source-independent
normalized snapshots/paid lending are valid preparation representations; no
mandatory complete normalized TD copy is selected. Required passes and retained
input have explicit work/byte/lifetime owners. Ordinary Serde remains a typed
input API; it does not establish bounded wire ingestion. ADR-0020's explicit
strict-ingestion literal JSON/duplicate/null/Number and shared field-decoding
contract remains a separate capability, not a prerequisite of every typed use.

The bounded semantic loan now resolves a selected Form at Thing, Property,
Action or Event scope, retaining original coordinates, effective operations,
security, typed affordance schemas and local-context extension associations.
A Thing-root Form's loan also includes the described Property schemas with each
Property's own local context, without lending or selecting individual Forms.
`collection_properties` is available only for a root loan; its scopes, schema
branches and name/context resolution use the existing bounded arena and work.
Selected Combo references are reachable through the same shared security loan.
Unknown security definitions survive TD decoding and round trips, including
when unused; selecting one still requires an actual supporting security service.
The older Consumer Property Read compiler remains a narrow NoSec/Property
adapter, not a rule that limits shared TD admission or new capability compilers.

Applicable standard: [TD 1.1 §§5–8](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/).
Binding-specific defaults, such as HTTP methods and its multi-operation method
restriction in §8.3.1, belong to Binding preparation. Shared meaning includes
schemas/security/defaults, not transport interpretation.
