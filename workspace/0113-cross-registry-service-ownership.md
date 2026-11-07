# Physical service ownership across independently composed registries

Status: MIGRATED

RES-OWNER-001 requires one original funded owner for shared physical backing;
RES-HANDOFF-001 forbids charging that backing again at aggregate/local handoffs.
PR #260 proves raw alias matching within one finite registry construction and
explicitly leaves sharing across registries/Servients unproven. That limitation
is evidence scope, not an exemption from either requirement.

The raw-service reproduction at `3b98056` uses independently constructed HTTP
registrations with the same actual data, access or both service objects. A
60,000-byte backing produces a 69,152-byte reservation at **each** registry,
whether their banks are the same or separate. A second construction under a
100,000-byte common bank refuses and rolls back. Dropping the first registry
releases its charge while the second retains its independent duplicate charge.
Thus raw composition can run under sufficiently large accounts, but its counters
cannot represent exactly-once physical ownership. The API cannot import the
already available `ConsumerServiceOwner` into a registry.

Selected correction: import explicit retained data/access owners into finite
registry construction. Each owner keeps its first account; each registry funds
its own records, references and actual native registration separately. Service
adoption must return its raw protected input on refusal. The raw convenience
constructor remains registry-local startup adoption and explicitly excludes
independently adopting an alias elsewhere. Composition roots propagate owners,
not a pointer key or a claim of equivalent configuration. Independently funded
data and access owners may be combined; bindings still capture fresh policy,
credentials/revisions, logical authority and instance/call/result accounts.

Alternatives: declaring raw sharing universally unsupported avoids a local
change but leaves a concrete composability need unconstructible. Requiring all
callers to pre-retain services removes useful local startup adoption without
improving the explicit path. A global pointer cache or universal manager adds
hidden identity/domain/lifetime policy. Configuration equivalence cannot prove
shared allocations or current permission. Native resource ownership stays with
concrete services/Bindings.

Affected authority: Foundation physical ownership/handoff, Planning finite
registry admission, API projection, WP-200 implementation and prior reuse
evidence. Existing logical identity, security, runtime and Binding contracts
remain binding. No requirement meaning, architecture direction, package gate
or milestone changes; the current owners can describe this bounded correction
without a new ADR.

Falsifiers: a second backing charge on retained import; a registry admitted
without its local costs; lost protected owners or partial leases on refusal;
permission/credential/revision bleed; an original account released before a
normal or late result; or service destruction after its funding disappears.
Use exact account deltas and actual service destruction alongside real HTTP/TLS
execution and differing retirement orders. Host finite scope does not establish
portable ownership or universal Action/notification/Producer service composition.

The correction is projected into Foundation, Planning, the API matrix, WP-200
and the existing reusable-Consumer evidence. The focused HTTP/TLS regressions
exercise the stated falsifiers. Migration is an implementation claim, not
independent acceptance or broader family/profile qualification.
