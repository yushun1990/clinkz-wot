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
