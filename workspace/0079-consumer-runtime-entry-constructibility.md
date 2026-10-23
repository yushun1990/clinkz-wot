# Consumer runtime entry constructibility

Status: MIGRATED

## Question and evidence boundary

Does accepted unpublished aggregate completion justify admission of the entire
proposed WP-400 Consumer transaction/facade? No. Complete lower-layer nodes
establish dependencies, not a constructible runtime ownership or product boundary.

The examined baseline is `5ed0ad7c1713794a2c7cacac2094f57d3d14b81c`.
The aggregate's accepted evidence explicitly retains a manual cleanup root and
its original committed account. It proves neither Servient parent pairing nor
publication. The binding evidence proves dual-role execution, not a client-only
installed bundle. [Review 07](../docs/reviews/review-07-product-runtime-architecture.md#registration-is-currently-producer-centered)
and [topic 0076](0076-composable-capabilities-and-runtime-boundaries.md) require
these distinctions; their deployment proposals are not architecture authority.

## Concrete findings

| Finding | Repository evidence | Consequence |
| --- | --- | --- |
| Consumer construction requires a server | `StaticBindingRegistrationInput<B>` and `StaticBindingRegistration<B>` require `PollServerBinding`; Host input stores a mandatory server and its validator requires Producer capability | WP-400's Servient-only source envelope cannot construct a genuine Consumer-only product. A dummy server makes the evidence misleading. |
| Lifetime-free output is not a shared Host owner | Planning's `Root<A>` uses `Cell`, `UnsafeCell`, a non-atomic reference count and an explicit `Rc` marker; draft, slot and rollback are local | `Arc<Draft>` or `Mutex<Draft>` cannot make that graph transferable. A local driver is constructible, but is a distinct runtime choice needing exact handle/cleanup/scheduling evidence. |
| Child accounts cannot authorize parent capacity | Foundation allows two equal independent ledgers with equal public reservation owner tuples | The private Servient transaction must retain the real parent/global reservation and bind policy, phase, registration and generation. A ledger getter or numeric equality does not repair this. |
| Host call backing is not fallible compiler backing | `HostBindingCallBox::new` uses `Box::pin`; installed Host start invokes the raw client, then allocates a seal decorator or accepted-overrun owner | The closed compiler's fallible slot proof cannot certify call construction. Do not infer recoverable allocator failure, bounded callback cost or physical backing from declarations. |

The [historical external discriminator](https://github.com/yushun1990/clinkz-wot/blob/5091685/tools/architecture-fixtures/consumer-runtime-entry/README.md)
owns executable cases and their limits. Its local owner retains actual public
Planning output/root/account, exercises abandonment and bounded reclaim, and uses
the concrete artifact after source and registration destruction. That proves a
local unpublished ownership alternative, not a thread-safe Published record,
call lease, root-drop recovery, global governor or runtime admission.

## Selected correction and alternatives

The smallest independently reviewable implementation successor is a complete
Consumer-only registration precursor, with the existing compiler and sealed
execution contracts and additive Planning adapters. Its exact candidate is
[WP-300's admission record](../docs/work-packages/WP-300-consumer-only-registration-admission.md).
The dependency, ownership and completion projections are migrated there and to
Binding SPI, Planning, WP-300/WP-400 and API ownership. This is a role-completeness
correction under existing ADR-0009 authority, not a new runtime architecture.

The proposed two representations have distinct owners: a typed client-only
static bundle and an erased client-only Host bundle. Existing Producer and
dual-role constructors remain valid. Share Core's private execution/sealing
mechanics; do not implement a second validator or seal issuer.

Rejected for this step:

- A dummy server would preserve the concrete product defect and fictitious
  owner/declaration pressure.
- Removing the server bound from the existing generic bundle, or introducing
  a role typestate framework, would put every Producer method and existing
  registration into the change boundary. Additive complete role-specific bundles
  are smaller; ordinary private helper extraction may avoid duplicated mechanics.
- Doing this inside the entire WP-400 transaction would combine independent
  rollback and validation truths: registration construction does not require
  publication, leases, a scheduler or a global governor.
- A Contracts extraction, generated image or Host/embedded crate split does not
  resolve the immediate complete-owner mismatch and lacks production evidence.
- Selecting unsafe Send/Sync, a local Host runtime, or an atomic Planning root
  now would settle an independent ownership decision without its required proof.

## Migration and remaining decision boundary

The narrow role-only decision is migrated; topic 0076 remains DISCUSSING for
broader product/runtime choices and the concrete Host handoff question recorded
there. Registration admission does not depend on choosing that runtime. Conversely,
WP-400 cannot classify those findings as disjoint from its own scope.

The WP-400 entry contract now requires the role-only precursor and an executable
choice for Host/static root, call and parent-capacity ownership before its
admission candidate. That future correction must reconcile exact resource/work
envelopes, source/build-loan termination, publication-slot/cancellation races,
cleanup transfer, and child-before-parent terminal release. The existing
multi-owner checkpoint remains separate; it is neither a substitute dependency
nor an extra global gate for this narrow work.

Prior completion evidence is reaffirmed only at its recorded scope. This change
adds no production code, admission, completion, gate transition or milestone
claim. No existing accepted claim is falsified by these successor findings.
