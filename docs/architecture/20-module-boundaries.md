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
and Foundation. `RawFiniteServient` owns the logical registry, unique authority
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
It does not qualify or publish a TD, validate DataSchema/security schemes, or
replace the preparation-dependent Property Read Servient. Its `activate_raw`
gate permits only this explicitly raw local service; full Producer exposure
still requires final TD coverage, resource admission and publication.

Other runtime availability remains narrow Property Read: the Host HTTP Consumer,
bounded loopback Producer, static/Host fixtures and manual Consumer runtime.
The existing `HostSemanticCompiler` Copy/atomic restriction, eager all-bounds
transaction, plan-set epochs, route-stage carriers and one-route Producer root
are un-frozen implementation details for those adapters. They MUST NOT set
eligibility or shape for new capability families. Replacement must remove a wrong
abstraction, not preserve it through aliases or a compatibility facade.

Anonymous TDs, empty affordance keys, extension security and no-atomic execution
are target requirements; the current Consumer's id/key restriction, closed
security enum and pointer-atomic runtime remain explicit gaps. TD vocabulary
availability on a no-atomic target proves tooling compilation only. Identity/shape
result seals are not full DataSchema validation. No general runtime, shared
native infrastructure or new Action/Event implementation is claimed here.

The withdrawn event broker/subscription queues, old TD-at-call-time facade,
Zenoh/pico implementations and admission prototypes remain in Git. Their deletion
withdrew availability, not the enduring product goals. Historical evidence and
commands stay tied to their original revisions.
