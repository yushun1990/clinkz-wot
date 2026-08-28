# 0062 Consumer Plan-Set Handoff Closure

Status: DISCUSSING

Kind: implementation-discovered architecture handoff investigation

Priority: HIGH

Target: the missing WP-200 -> WP-400 handoff needed to construct and publish the v5.1 Consumer Property Read plan set without Servient-owned TD interpretation

## Scope and authority

This topic records an implementation-boundary defect discovered immediately after completion of `WP-300-CONSUMER-PROPERTY-READ-BINDING` and before admission of the corresponding WP-400 Consumer tranche.

It does not itself change active v5.1 authority, admit Rust source, register the Consumer architecture gate, or activate `PLAN-INDEX-001`, `PLAN-LAZY-001`, `PLAN-CACHE-001`, `BIND-PROGRESS-001`, fallback, subscriptions, or production Zenoh.

The question is narrower: what Planning-owned value lets `Servient::consume` publish a usable consumed Property Read plan set when the already-completed WP-200 tranche compiles one exact `(property_name, property-form index)` coordinate at a time?

## Established repository facts

The following facts are already authoritative or completed implementation evidence.

1. `PLAN-SET-001` assigns every consumed handle generation one Servient-owned aggregate compiled-plan-set record. Planning constructs immutable set material; Servient owns the build transaction, publication, pins, operation leases, drain, and reclamation.
2. The general Planning algorithm owns enumeration of target contexts, effective operations, forms, and binding candidates. Servient does not reinterpret a TD to recover planning decisions.
3. The completed WP-200 Consumer tranche deliberately exposes only an exact-coordinate compiler entry:

   ```text
   PropertyReadPlanCompiler::consumer_call(
       property_name,
       property_form_index,
       ...
   )
   ```

   Its completion proof uses multiple readable properties and multiple readable forms and proves that document order cannot silently replace the supplied coordinate.
4. `select_consumer_property_read` consumes immutable `PlanBuildOutput`, property name, and narrowed `InteractionOptions`; it cannot receive a TD, raw Form, binding object, or support probe.
5. The completed WP-200 tranche returns one unpublished eager single-coordinate draft. Publication, leases, draining, and reclamation are intentionally left to WP-400.
6. The completed WP-300 tranche supplies the selected `OutboundRequest`, Host client call, static client slot, exact-request rejection, cancellation settlement, and Core response-validation handoff required after selection.
7. WP-400 v5.1 authority requires `consume` to publish an immutable consumed Property Read plan generation before returning the handle, and requires `read_property` to select through that published generation rather than entering the legacy `ConsumedThing` path.
8. The current legacy `ConsumedThingHandle` still scans the TD at call time to find a supporting Form, and current `Servient::consume(td)` has no explicit Property Read build coordinate.

## The closure defect

The completed narrow tranches do not currently compose into the required public Servient path.

A WP-400 implementation cannot legally choose any of the following shortcuts:

- **Call-time TD/Form scanning.** This directly violates the v5.1 Consumer boundary and the negative evidence already required by WP-200.
- **Servient-owned startup TD enumeration.** Moving the scan from call time to `consume` does not fix ownership. Target/form/effective-operation enumeration belongs to Planning; Servient may orchestrate Planning but must not implement a second planner.
- **Selecting the first readable property/form merely because the current WP-200 compiler needs one coordinate.** WP-200 explicitly proved that its exact constructor cannot acquire target semantics from document order.
- **Changing public `consume(td)` to require one property/form coordinate.** This would move an internal compilation coordinate into the application facade, narrow one consumed Thing to one preselected property, and create a new public-contract decision not required by current v5.1 authority.
- **Re-entering legacy `ConsumedThing`, `BindingRequest`, `supports_with_thing`, or bare client-binding arrays.** Those are the exact target backflows the Consumer architecture proof is intended to eliminate.

Therefore an ADR-0013 WP-400 source admission that starts directly from the current WP-200 single-coordinate draft would be incomplete: it would leave the source of the published consumed plan-set contents unspecified.

## Candidate closure

The smallest coherent closure is a new narrow WP-200 Consumer aggregate-plan-set tranche before WP-400.

Planning should own one bounded eager **Consumer Property Read plan-set build** over the captured validated TD and one immutable complete Consumer-capable registration projection. For this first proof it should:

1. enumerate Property Read property/form coordinates in Planning at build time;
2. retain deterministic source order in immutable plan-set material;
3. compile each admitted coordinate through the already-completed exact `PropertyReadPlanCompiler::consumer_call` semantics rather than adding a second artifact compiler;
4. use one admitted Consumer-capable complete registration only, so no client capability index or multi-binding fallback is required;
5. produce one owned aggregate `PlanBuildOutput`/equivalent set draft whose logical plans, artifact envelopes, and references share one plan-set generation;
6. preserve every retained coordinate required by the admitted bounds rather than silently truncating competing readable forms;
7. support immutable-plan-only selection by addressed property name plus optional exact form index;
8. when form selection is omitted, select only according to the deterministic order already frozen into the immutable set; this is initial plan selection, not post-failure fallback;
9. reject an explicit form index that is absent from that property's published plans rather than rescanning the TD or selecting another coordinate;
10. drop the TD, registration build inputs, compiler inputs, and temporary enumeration state before the handle becomes callable.

The first aggregate tranche must remain deliberately smaller than broad Planning:

- no `PLAN-INDEX-001` capability index;
- no lazy artifact or cache/single-flight state;
- no second Consumer binding candidate and no automatic fallback after a candidate failure;
- no write/action/observe/collection planning;
- no advanced binding/media/subprotocol/security selector;
- no Servient lifecycle or binding execution implementation.

A bounded linear scan of the immutable first-proof plan set at call selection is acceptable unless implementation evidence shows it violates an already-active bound. The purpose of inactive `PLAN-INDEX-001` is not to force Servient back to the TD or to require an index before the first Consumer proof.

## WP-400 consequence

After the aggregate Planning handoff exists, the WP-400 Consumer tranche can remain narrow and mechanically compositional:

```text
consume(td)
  -> Planning-owned aggregate Consumer Property Read draft
  -> Servient atomic plan-set publication
  -> ConsumedThingHandle with generation-bearing plan-set ownership

read_property(name, options)
  -> acquire operation/plan-set lease
  -> Planning immutable-set selection
  -> OutboundRequest::property_read(...)
  -> selected Host call / static ClientRequestSlot
  -> validate_untrusted_binding_output(...)
  -> InteractionOutput
  -> exactly-once call + plan lease settlement
```

Servient may own the aggregate record, generation, publication state, leases, admission, drain, cancellation ownership, and cleanup. It must not own effective-form enumeration, raw Form interpretation, candidate construction, or a duplicate selection algorithm.

The existing legacy `ConsumedThingHandle` implementation may remain for unmigrated capabilities, but the target `read_property` evidence must poison its TD scan, `BindingRequest`, support-probe, and bare-client-binding edges.

## Dependency consequence

If this candidate survives independent review, the implementation order becomes:

1. completed `WP-200-CONSUMER-PROPERTY-READ-PLANNING` exact-coordinate tranche;
2. completed `WP-300-CONSUMER-PROPERTY-READ-BINDING` tranche;
3. new narrow WP-200 Consumer aggregate-plan-set tranche;
4. WP-400 Consumer Property Read Servient tranche depending on both the aggregate Planning handoff and completed WP-300 execution contracts;
5. cross-package Consumer Property Read architecture gate;
6. real Host Zenoh Consumer Property Read evidence.

The new WP-200 tranche is an upstream constructibility closure, not broad WP-200 completion and not a reason to reopen the completed WP-300 execution contract.

## Required independent review

Independent review should challenge at least these questions before this topic becomes `DECIDED` or an ADR-0013 tranche is admitted:

1. Does active v5.1 authority already imply a different legal source for the aggregate Consumer plan-set contents?
2. Can the first proof aggregate all readable Property Read coordinates under one complete registration without activating `PLAN-INDEX-001` or fallback semantics?
3. Is deterministic immutable plan order sufficient for omitted form selection, or is another active policy owner required?
4. Should the aggregate result reuse/extend `PlanBuildOutput` or introduce another Planning-owned set-draft type?
5. Can Host and application-static profiles consume the same semantic aggregate without forcing erased storage into the static profile?
6. Are current active count/work/byte bounds sufficient for the aggregate first proof, or does aggregation expose a genuinely missing active requirement?
7. Does any proposed solution accidentally move TD interpretation, candidate selection, or artifact lookup into Servient?

## Rejected immediate progression

Directly admitting or implementing WP-400 before this handoff is closed is rejected. The missing coordinate/set construction source affects package ownership, public Consumer facade correctness, immutable plan-set semantics, and the negative legacy-backflow proof. It is therefore an architecture-sensitive predecessor issue, not a local Servient implementation detail.

## Migration condition

This topic may become `DECIDED` when independent review establishes one exact legal Planning -> Servient handoff and falsifies the rejected shortcuts above. It becomes `MIGRATED` only after the conclusion is represented in the appropriate Planning/WP-400 authoritative owner and the necessary ADR-0013 source tranche or tranche dependency is independently admitted.
