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

## Joined execution and protocol-independence evidence

For the composed path prioritized in [PLAN.md](../../PLAN.md), a Binding's real
native runtime supplies closed installation, actual captures/correlation,
delivery and final-use retirement. The Servient retains the single current
logical authority and behavior owner; native callbacks never select/call
handlers. Prepublication native visibility is permitted only with bounded
closed ingress policy. Each eventual candidate independently passes current
logical acceptance, while reply opportunities and sharing topology remain
Binding-native. One logical Thing implies no particular number of native
resources or physical subscriptions.

The existing direct HTTP finite participant can exercise the seam without
making request/reply or one-listener-per-Thing universal. Later select
contrasting native behavior to challenge sharing, multiple recipients, reply
cardinality and cancellation. A real downstream Cargo-linked registration
must be possible without Core protocol cases/whitelists. These are
claim-scoped integration checks, not a required number of protocols, new
universal matcher/session layer or separate generality-proof gate.

## Consumer capability composition

The [first open Consumer slice](WP-200-planning.md#open-consumer-integration)
replaces Property Read-only eligibility as the general registration model.
Capture actual startup-linked capabilities/configuration and required services;
selection indexes only narrow candidates and never certify execution. Bindings
may supply one facet without dummy server/notification methods. Preserve exact
owner/input recovery on refusal and immutable compiler/executor pairing.
Runtime-only registration remains compiler-free; HTTP's existing
`try_consumer_execution` already supplies that half of the boundary.

Reuse `PollClientBinding` / Host `ClientBinding` only for the working finite
facet. Reuse [ConsumerNotificationBinding](../../core/src/notification.rs) and
`NotificationParticipation` for continuing conversations; their owned `Program`
already has no `Copy` bound. Common selection/preparation hands off to these
distinct controls, never to a universal call/session. Discovery remains an
explicit workflow under WP-500, even when it uses a selected Consumer operation.

Continuing Consumer Actions use the distinct `ConsumerActionMapping` and
`ConsumerActionExchange` native facet. A selected invoke mapping produces its
opaque continuation; independently selected controls must explicitly support it.
Native destination expansion precedes captured access checks. Exchanges retain
pending/failed retirement and late replies with complete owners, and owe finite
local abort. No transport correlation becomes logical authority. Real HTTP joins
the existing qualified Producer; the independently Cargo-linked Action fixture
uses different invoke/control implementations and real datagrams to test this
boundary, including blocked/error cleanup and late invalid completion. Its
evidence supplements the existing synchronous finite and notification regressions.

HTTP SSE does not determine general fields, thread/atomic requirements, retry,
reply cardinality or one-connection-per-subscription policy. Native streams,
correlation, framing, reconnect and shared-service leases stay Binding-owned.
Test that stopping one logical participant cannot retire siblings and that final
physical destruction waits for remaining native work. Multi-family finite execution and raw notification composition with independent
test Bindings supply this evidence at their claimed scopes. The typed
`ConsumerNotificationPreparation` facet now hands TD-selected Property/Event programs and
actual services to the same notification participation API. The registered MQTT
Consumer shares native topic interests and distribution across independent
logical Consumers; real Mosquitto integration joins the existing qualified
Engine Producer to those Consumers, including stop, loss, replacement and final
release. The [MQTT subset](../../protocol-bindings/protocols/mqtt/README.md) retains
its concrete limits and control/response refusals;
an operation enum or registration flag is insufficient.

The Host finite `ClientBinding` declares coarse operation eligibility explicitly;
complete support still comes from actual compiler/services. `start_finite` replaces
the Host Consumer-only Property Read entry, and requests carry the selected
operation and optional application input. Retained static/dual-role adapters stay
explicitly Property Read-only. Neither facet owns Consumer currentness: finite
calls and `ConsumerNotificationBinding` participants retain the same externally
owned logical instance reference. Production HTTP supports Property Read/Write on
this Consumer path, with native GET/PUT selection and separate 200 value/empty 204
completion mappings. The finite Producer validation SPI now checks both original
candidate input and output; a shared HTTP listener retains independently qualified
Read/Write definitions and refuses native collisions. Concrete limits and
response refusals are in the [HTTP README](../../protocol-bindings/protocols/http/README.md#composed-property-readwrite).
HTTP SSE remains an optional mapping; the real datagram
participants are test Bindings, not a new production protocol.

The Producer publication facet now uses explicit Property/Event sources and
`PublicationData` variants over one narrow native publication lifecycle. Actual
observe/subscribe mappings retain separate targets, schemas and captured services;
they do not become finite calls or universal sessions. The MQTT mapping justifies
observe/unobserve as SUBSCRIBE/UNSUBSCRIBE under its non-retained QoS 0 subset.
Property and Event tests reaffirm native sharing, partial-control ownership,
publication uncertainty and final physical release. The
[MQTT README](../../protocol-bindings/protocols/mqtt/README.md) owns that subset.

## Native collection read facet

The finite HTTP capability now supports one native Thing-root ReadAllProperties
mapping beside independent Property mappings, with explicit logical targets and
native method/path collision refusal. Collection codec support is explicit;
`FiniteValidator` captures additional traversal work before dispatch. The
[Binding SPI](../spec/binding-spi.md#native-thing-root-readallproperties) and
[HTTP subset](../../protocol-bindings/protocols/http/README.md#thing-root-readallproperties)
own those contracts.

ReadMultipleProperties, WriteAllProperties, WriteMultipleProperties and long-lived
collection operations remain unimplemented. This scoped capability changes no
work-package dependencies, gate, milestone or production qualification status.
