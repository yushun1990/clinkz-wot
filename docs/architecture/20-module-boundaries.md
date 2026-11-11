# Module boundaries — v6.0

`MOD-DEPS-001`: Dependencies MUST follow this acyclic direction (`A -> B`
means A depends on B):

```text
Core                        -> Foundation
TD                          -> Core, Foundation
Planning                    -> TD, Core, Foundation
Binding runtime             -> Core, selected native/platform/payload services
Binding preparation         -> Binding runtime, Planning, TD
Servient execution          -> Core, Foundation
Servient preparation facade -> execution, Planning, TD
Discovery                   -> Core, TD, selected client adapters
Application / umbrella      -> selected capabilities and implementations
```

Core MUST NOT depend on TD, Planning, Discovery or a concrete Binding, including
optional features and reexports. Planning/Servient MUST NOT name protocol cases.
A Binding MUST NOT depend on Servient or retain application-dispatch authority.
Native executable types belong to the Binding runtime; optional preparation
produces those same types. Shared payload/schema/security execution contracts
belong in Core, with implementations supplied as composable services. No new
Contracts crate, universal runtime backend or Host/Static crate split is required.

| Component | Owns |
| --- | --- |
| Foundation | Linear capacity/work, time and identity primitives |
| TD | Documents/extensions and shared WoT interpretation |
| Planning | Bounded selection/support and pure construction coordination |
| Core | Semantic interaction/data/security/ownership contracts and outcomes |
| Servient | Logical instances, current permission, publication, behavior dispatch, accepted work and cleanup |
| Binding | Native mapping, matching/captures, correlation, I/O, distribution, infrastructure and its limits |
| Discovery | Explicit introduction/exploration/retrieval client workflows |
| Application/platform | Linked composition, behavior/services, deployment and environment recovery |

Runtime registrations need no compiler. Execution-only compositions MUST NOT
acquire an upward preparation dependency. An optional preparation coordinator
may own bounded normalized input and install generation-checked prepared entries;
Core and native callbacks cannot reconstruct them from source documents.

## Retained pre-v1 implementation baseline

This is a target dependency graph, not a claim that every edge is already
extracted. Current Core has only Foundation as its project dependency. TD owns
semantic loans and Planning owns compilers. HTTP has optional `prepare`.
Servient still unconditionally composes TD/Planning: its Property Read output,
rollback and Producer build owners name Planning. These are restricted adapters.
Their output/reclaim owners must move to the execution owner before claiming an
execution-only Servient feature; a feature alias is insufficient.

The separate `clinkz-wot-servient-finite` Host execution slice links only Core
and Foundation. `RawProducerServient` owns the logical registry, unique authority
and actual behavior set. Independently typed `RawFiniteRuntime<B>` values own
their captured Binding/configuration and one bounded exchange/settlement each;
none owns a copy of current logical permission. Servient progress borrows the
one logical owner and checks current recipients/behavior under a fixed paid
count/name-byte lookup envelope. Native progress/close needs no logical owner.
Accepted work retains its original instance through settlement acknowledgement;
replacement cannot retarget it, and retiring one Binding does not revoke the
instance or its siblings. Weak issuer association fences new acceptance after
the logical owner is dropped. This synchronous manual Host facet uses local
reference retention and claims no thread mobility or automatic scheduling.

The complete native exclusive input/reply survives refusal and close. Effect,
delivery, originating Binding, accepted target and complete late result survive
until settlement acknowledgement. The target shares registered backing; output
is application-backed until return and the Engine accepts only the declared
complete output representation. Raw data still obeys Property result-shape rules.
Required output-validation mode additionally uses the captured native provider's
actual prepared service, after prepaid bounded result traversal. Caller-clock
deadlines fence unfinished ingress, dispatch and pending delivery; local fencing
does not require credits, while native/cleanup polls do.
The direct HTTP finite facet supplies its prepared JSON/type service without the
older installation/permit ABI or a Binding-to-Servient dependency. Neither mode
qualifies/publishes a TD or replaces the funded preparation-dependent Producer
root. `activate_raw` remains local activation even when output checking is
required. The optional `servient::producer` facade now joins complete final
qualification to source-free staged publication for its single-read subset.
It captures the actual preparation provider at startup, stages/funds the final
description and behavior before native setup, rechecks the closed native
participant, then commits locally at the one logical owner. Detailed supported
scope and resource/evidence limits belong to WP-200/WP-400/WP-600; the legacy
installation path remains an independent restricted implementation. Runtime-only
finite and HTTP normal dependency graphs retain their original direction.

An additional whole-Thing Producer selector loans shared semantics for every
Thing/Property/Action/Event Form and effective operation to actual linked
preparation registrations. Its owned definitions are distinct from the logical
registry and native participants. The preparation-dependent Servient handoff
requalifies the complete final description against the selected definitions and
checks each operation has exactly one ready native contributor; only then does
the source-free owner commit one logical description, behavior set and permission
cell. A participant can lease shared native infrastructure for multiple Things.
HTTP Property Read plus a linked test-only real-UDP continuing Action exercises
independent readiness, one commit, query/cancel, replacement and retirement;
it is not a production UDP Binding or a generic Action transport.
Direct raw composition remains caller-funded. The optional owned Host handoff
retains original participant identities and managed controlled backing through
native/work/result disposal, as specified by [Binding SPI](../spec/binding-spi.md)
and [Runtime Safety](../spec/runtime-safety.md). Native OS/SDK/shared infrastructure
has its separate actual Binding owner; a linked test socket does not establish
production resource qualification.

The same execution-only logical owner can register Action behavior without a
finite runtime. Core's raw Action contracts and Servient's bounded invocation
store separate application continuation from the initiating Binding exchange.
The owner pins the original instance, invocation reference, principal and
selected invoke/query/cancel mapping associations; controls recheck current
authority and policy. Cancel/abandon do not assert completion, and a late result
remains charged through application retirement, acknowledgement transfer and
actual settlement disposal. Retained result/target/error data are borrowed from
that original owner.
Lost handles cannot orphan work: the owner has a separate paid retirement path;
root Drop uses only the application's qualified finite local abort for unfinished
work, not a poll or a claim of completed native delivery.
Native correlations, replies, cancellation and infrastructure cleanup remain at
the Binding. The raw Host kernel itself has no native Action adapter or TD
qualification; TD-driven mapping/validation and complete publication are separate
composed capabilities. Automatic progress is not claimed by these manual facets.
It does not model Property observation or Event notification as finite exchanges: their
establishment, delivery, reciprocal responses and stop require separate logical
conversation owners.

Executable availability is established by source and scoped tests linked from
[PLAN's coverage](../../PLAN.md#capability-coverage), not by this boundary model.
The retained Property Read adapters'  `HostSemanticCompiler` Copy/atomic restriction, eager all-bounds
transaction, plan-set epochs, route-stage carriers and one-route Producer root
are un-frozen implementation details for those adapters. They MUST NOT set
eligibility or shape for new capability families. Replacement must remove a wrong
abstraction, not preserve it through aliases or a compatibility facade.

Anonymous TDs, empty affordance keys, extension security and no-atomic execution
are target requirements; retained narrow adapters may still have id/key or
pointer-atomic restrictions, without constraining open composition. Unknown security schemes are now
preserved, but executable provider support is not implied. TD vocabulary
availability on a no-atomic target proves tooling compilation only. Identity/shape
result seals are not full DataSchema validation. The boundary model itself grants
no implementation, complete-family or production qualification claim.

The modern [finite execution boundary](../spec/interaction-core.md#finite-execution-and-ownership-representations)
now separates shared value/result/progress contracts from Host ownership. Core
exposes borrowed meaning and output plus a common finite transition kernel;
Servient's caller-owned Read slot and its existing Host Consumer use that kernel.
The narrow [executable proof](../evidence/constrained-finite-execution.md) includes
actual test-Binding datagrams and no-atomic compilation. Its native facet depends
only on Core/Foundation, and the runtime retains no TD/Planning dependency. This
establishes a bounded alternative representation, not all-family Embedded support.

The withdrawn event broker/subscription queues, old TD-at-call-time facade,
Zenoh/pico implementations and admission prototypes remain in Git. Their deletion
withdrew availability, not the enduring product goals. Historical evidence and
commands stay tied to their original revisions.


The same source-free execution crate contains the finite `ConsumerOwner` lifecycle
boundary, non-revoking `ConsumerAssociation` views and independent `ConsumerCalls`
work storage. Registered instances/facets have one actual logical owner; dropping
an application handle never removes them. Standalone ownership remains a separate
low-level representation whose unique authority is consumed on adoption. Persistent
registration and finite capacity have independent cardinalities; neither implies
protocol selection, an executor or an authority-change notification service.
Planning owns open registration/selection and paid
owned compilation; Core owns immutable completed Consumer definitions and service
contracts. Execution consumes those definitions without an upward TD/Planning
edge. The shared opaque Host bank now lives in Core; existing Servient account
users retain their module re-export. Legacy owners use move-only partial-release
leases instead of access to the bank's private reserve/release operations. No
Planning-to-Servient dependency or Binding runtime dependency is added by this
Consumer path. Finite calls and notification conversations remain separate. The notification
executor and its lifecycle tests now also live in `servient/finite`, with no
TD/Planning dependency or old-path compatibility re-export. Both facets retain
the independent Core Consumer authority reference. A finite facet no longer
manufactures the logical Consumer incarnation.

Thing-root Event subscriptions and Action status queries can share a logical
instance while using independent notification and finite facets. Their prepared
Event-role and Action-output tables remain distinct. A composed Producer captures
the root Event publication route and root Action query route from their actual
native participants and qualifies the entire final description before publication.
MQTT keeps subscription/occurrence attribution; HTTP keeps invocation inventory
and aggregation. Neither native owner inherits the other's logical lifecycle.
