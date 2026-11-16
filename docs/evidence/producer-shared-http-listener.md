# Independent Producer publications on one native HTTP listener

This is scoped implementation evidence under the existing Producer SPI. It is
not independent acceptance, a package/milestone closure or production qualification.

## Architecture finding and implementation

SPI-NATIVE/INGRESS/RETIRE-001 already support one physical owner with independent
logical participants. Current `FiniteIngressBinding` candidates, original reply
opportunities, immutable configurations and exact closed coverage suffice. The
restriction was the concrete finite HTTP participant's exclusive socket binding,
not a missing Engine routing or server/session abstraction.

`HttpFiniteListener` owns the socket and atomic route inventory. Separate
`HttpFiniteBinding` participants select it through `with_listener`; complete
description preparation, captured providers/services/behaviors, managed admission,
final qualification and publication remain the original production path.
Native head selection routes to one original participant without access to any
Engine publication, handler or authority object. Configurations compare the actual
listener identity, not a fabricated compatibility tag. No Core, Planning or
Servient implementation/API changes are required. The existing runtime instances
adapt native participation; the architecture gains no per-Thing Runtime rule.

The [selected alternatives](../../workspace/0127-producer-shared-http-listener.md)
and [native contract](../spec/binding-spi.md#independent-http-listener-participation)
own the decision and exact bounds. The earlier preparation evidence retains its
independent-listener scope; compiler reuse and native sharing are separate choices.

## Real execution

`protocol-bindings/protocols/http/tests/composed_thing/shared_listener.rs` exercises
real TCP, the HTTP compiler, structured value/additional-response validation,
actual managed banks and complete TD publication. It shares one explicitly
compiled Read definition to preserve #283's orthogonal reuse boundary.

- Independent alpha/beta TDs, actual registrations, names, routes, behaviors,
  policies and banks start on the same address without a second listening socket
  or port-sharing option. Only alpha publishes; its native turns queue beta's
  early input without dispatch. Alpha serves while beta remains unpublished;
  beta subsequently publishes and handles its original queued request.
- An unread alpha result does not block beta's publication or native receive.
  Fresh beta denial leaves its behavior untouched and does not affect alpha.
  Original normal and schema-validated 422 replies use the actual recipient.
- Closing alpha's native lease permits a same-route replacement with another
  name/registration/behavior, while its unread original settlement still prevents
  old logical retirement. The replacement serves its own value; acknowledgement
  reports the original recipient/value/delivery and releases only old work.
- A distinct replacement route stages/publishes while an old accepted reply is
  still Delivering. Explicit old close retains its original value and effect,
  reports NotDelivered, retains first cause and prevents retirement until its
  result is acknowledged. Replacement work executes once without replay.
- Complete final-description mutation with an unsupported retained Form refuses
  managed admission. Cross-participant native collision refuses the entire route
  set; its nonconflicting prefix is not accidentally reserved. A separately
  staged contender loses logical publication, disposes its queued early socket
  and original account, and leaves the serving sibling intact.
- A partially received request accepted before replacement cannot move to the
  successor after it finishes its head. A slow head does not block a sibling's
  valid request. A new request to the same native path reaches the successor.
- Thirty-two incomplete heads fill the native bound and another request waits in
  the OS backlog. Caller-clock expiry frees head slots and a staged recipient's
  queue without dispatch or logical revocation; both published siblings then
  serve fresh traffic.
- Mismatched listener address/timeout and secure sharing refuse before native
  setup. Deliberately incompatible clock progress makes shared infrastructure
  terminally unavailable; a staged Thing then refuses final native coverage,
  retains publication failure and disposes its queued input without dispatch.
- Closing/retiring/dropping one exposure leaves siblings usable and the port
  occupied. Each exposure bank releases independently; reused instructions retain
  the original compilation charge until their final owner drops. Final native
  lease release makes the socket bindable while a passive listener handle remains.

Focused command:

```sh
cargo test --locked -p clinkz-wot-binding-http --test composed_thing shared_listener -- --nocapture
```

Exact tested revisions, package counts and remote checks belong in the PR. These
tests supplement existing standalone HTTP/HTTPS, qualified publication, collision,
result/cleanup and reusable-instruction regressions; they do not replace them.

## Limits

The proved shared topology is Host/manual loopback cleartext NoSec Property Read
under one logical Servient, with matching address/timeout and one caller clock.
The optional infrastructure is bounded native backing, not an aggregate RAM,
throughput or production failure-containment qualification. Progress and expiry
require receiving turns; no background task, scheduler or OS readiness wake is
introduced. Native/OS pressure can refuse connections, and all participants
necessarily share physical listener availability and its declared native limits.

Same method/path generations cannot overlap: close the old native lease first,
or select a distinct new native route. The existing complete publication commit
does not promise an uninterrupted native same-route takeover. Input waiting only
in the OS backlog has no original logical recipient until native acceptance.
Local successful response writes do not prove remote receipt.

Shared TLS/security services, continuing Actions, mixed finite/continuing families,
cross-Servient trust/account combinations, Producer Host-loop composition,
constrained profiles, broad concurrency/load and v1 production qualification
remain separate work. Existing specialized/exclusive mappings retain their scope.
