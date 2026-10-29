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
This manual composed path caps an individual prepared definition set but does
not yet reserve aggregate live bytes across concurrent Things, stage a complete
native-owner account, or independently track the original participant set for
retirement. The caller must retain every participant and acknowledge its native
cleanup; the test's shared UDP socket is not a fully funded Host implementation.

The same execution-only logical owner can register Action behavior without a
finite runtime. Core's raw Action contracts and Servient's bounded invocation
store separate application continuation from the initiating Binding exchange.
The owner pins the original instance, invocation reference, principal and
selected invoke/query/cancel mapping associations; controls recheck current
authority and policy. Cancel/abandon do not assert completion, and a late result
remains charged until application retirement and settlement acknowledgement.
Lost handles cannot orphan work: the owner has a separate paid retirement path;
root Drop uses only the application's qualified finite local abort for unfinished
work, not a poll or a claim of completed native delivery.
Native correlations, replies, cancellation and infrastructure cleanup remain at
the Binding. The raw Host kernel itself has no native Action adapter or TD
qualification; the composed handoff is separate. Production Action mapping,
schema validation and automatic progress remain unimplemented. It does not
model Property observation or Event notification as finite exchanges: their
establishment, delivery, reciprocal responses and stop require separate logical
conversation owners.

Other runtime availability remains narrow Property Read: the Host HTTP Consumer,
bounded loopback Producer, static/Host fixtures and manual Consumer runtime.
The existing `HostSemanticCompiler` Copy/atomic restriction, eager all-bounds
transaction, plan-set epochs, route-stage carriers and one-route Producer root
are un-frozen implementation details for those adapters. They MUST NOT set
eligibility or shape for new capability families. Replacement must remove a wrong
abstraction, not preserve it through aliases or a compatibility facade.

Anonymous TDs, empty affordance keys, extension security and no-atomic execution
are target requirements; the current Consumer's id/key restriction and
pointer-atomic runtime remain explicit gaps. Unknown security schemes are now
preserved, but executable provider support is not implied. TD vocabulary
availability on a no-atomic target proves tooling compilation only. Identity/shape
result seals are not full DataSchema validation. No general runtime, production
shared-native infrastructure, notification implementation or complete Action
support is claimed here.

The withdrawn event broker/subscription queues, old TD-at-call-time facade,
Zenoh/pico implementations and admission prototypes remain in Git. Their deletion
withdrew availability, not the enduring product goals. Historical evidence and
commands stay tied to their original revisions.


The same source-free execution crate also contains the independent finite
`consumer::ConsumerRuntime`. Planning owns open registration/selection and paid
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
