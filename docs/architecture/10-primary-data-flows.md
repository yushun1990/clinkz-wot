# Primary data flows — v6.0

## Shared meaning and preparation

```text
immutable TD/TM + origin/context profile
  -> TD structural validation and shared semantic interpretation
  -> normalized operation/form/schema/security facts + original coordinates
  -> Planning selection + actual Binding/codec/validator/security support
  -> owned prepared mapping (or owned first-use preparation input)
  -> Servient instance admission and independent current authority
```

Interpretation and preparation perform bounded local work, no native I/O,
credential acquisition or application dispatch. Optional retrieval of a context,
TD or credential is a separately authorized workflow. Execution uses owned
selected mappings and prepared validation/security contracts without source-TD
reparsing. A declaration or native program never grants permission.

## Consumer

```text
intent + logical target + operands
  -> select allowed supported candidate / bounded first-use preparation
  -> policy, validation, execution/result/cleanup capacity
  -> owned Binding conversation and scoped current grant
  -> native exchange(s)
  -> selected response/data-role and payload validation
  -> semantic result / continuing Action reference / notification conversation
```

Native correlation, number of messages, transport progress and wire control are
Binding concerns. Remote effect, local delivery and local cleanup are separate
facts. Fallback before effects is distinct from retry after uncertain commitment.

## Producer

```text
private description + behavior + actual providers
  -> validate known dependencies and fund setup/cleanup
  -> closed native participation (when needed to learn endpoints)
  -> bind actual endpoints; finalize TD; recheck complete support and readiness
  -> one local publication of final TD/registry/current authority

native ingress/authentication/matching
  -> zero/many original logical candidates + captures + retained native backing
  -> independent identity/publication/policy/capacity/data checks
  -> scoped semantic acceptance
  -> Servient chooses and invokes captured behavior outside locks
  -> validated result + original reply/distribution capability
  -> Binding delivery and settlement
```

The receive path does not require a random Thing's serving permit to discover
its recipient. Matching grants no authority. A Binding never receives a handler
registry or direct dispatch callback. Physical ingress may progress while a
logical instance is closed. Its declared reject/buffer/control policy cannot
call behavior early. The final publication step performs no new fallible native
preparation. Remote observations and cached descriptions are not globally atomic.

Application Property changes and Event sources enter through Servient's semantic
source authority; Binding conversations own native distribution. An Engine-local
recipient has independent logical delivery policy. A broker's subscribers need
not become Engine records. Shared immutable packet backing does not imply shared
permission, a copy per recipient or a universal Engine broadcast queue.

## Discovery

`DISC-CLIENT-001`: Discovery owns introduction, exploration, bounded description
retrieval/continuation and TD admission as an explicitly composed client.
Adapters own their protocol mechanics; Core MUST NOT switch on discovery
protocols. Retrieved links/descriptions are untrusted: destination/credential
policy, input/result/work limits and cancellation apply before and during
retrieval. Introduction establishes neither peer authenticity nor interaction
permission. A Directory-described interaction MAY reuse Consumer capabilities;
direct retrieval MUST NOT be forced into Property Read. Directory server,
storage, replication and query-engine policy remain outside the Engine.

## Failure and ownership

Every crossing identifies who owns pending work, retained backing, first cause,
current authority and release. Rejected input returns its complete owner;
accepted work retains original identity through late completion. Failures report
logical settlement, local release, remote residue and infrastructure state
separately. Architecture examples select no universal transport topology.

## Host Discovery client contract

The first remote slice uses directly provisioned TD-server or Directory
collection endpoints, HTTP/1.1 GET and the W3C Discovery Recommendation's array
listing. A configured Directory collection ends in `/things`; its parent is the
API base for resolving relative navigation. This is an application-provisioned
exploration endpoint, not automatic interpretation of a Directory TD.

A trusted retrieval destination binds URL identity, explicit socket pin,
transport trust and optional credential audience before I/O. TLS verifies the URL
host against provisioned roots; a pin never replaces certificate identity.
Cleartext is an explicit loopback development option. Credentials cover only the
same origin and exact path; no retrieved Form, ThingLink, context, redirect or
challenge can expand that audience. A separately configured session is required
for another destination. Core and Servient acquire no Discovery service.

Pagination follows checked RFC 8288 `next` links, with the same collection
origin/path, unchanged requested limit and an offset immediately after the
returned members. Canonical collection identity and its `etag` must remain
consistent across paginated responses. Missing, ambiguous, cyclic, empty-progress
or out-of-policy navigation refuses the page. Results preserve identifier order
and reject cross-page repeats. Continuation exhaustion does not imply a trusted
or immutable remote inventory beyond the peer's supplied collection state.

The session bounds URL/header/wire/body bytes, JSON structure, TD size, work,
results, pages and elapsed time. A page is exposed only after complete framing,
JSON checks and existing bounded TD inspection/Basic admission of every member.
Admission preserves unknown metadata; it asserts neither full JSON-LD processing
nor selected interaction support. Planning still owns selected shared semantic
resolution and actual provider qualification. No transport interpretation is
added to the TD semantic layer.

The application owns each returned immutable description. It explicitly selects
a Thing and supplies actual Engine registration, instance authority, interaction
destination policy, credentials and capacity. Retrieval credentials/trust are
not implicitly transferred. A Directory-issued description cannot authorize
native execution. Source/session disposal remains independent of prepared
execution and original result-resource ownership.

Failure, cancellation, timeout or abandonment of an in-flight page terminates
that attempt without retry or partial-page delivery. The socket/driver and its
funding remain request-owned, with no detached task or pool. A dropped future
retains `Interrupted` as the session's first terminal cause; stop preserves it.
Previously returned complete pages remain application-owned. Local socket release
makes no claim about remote request processing or third-party storage cleanup.
The [WP-500 scope](../work-packages/WP-500-discovery.md) and
[evidence](../evidence/remote-discovery.md) identify remaining limits.
