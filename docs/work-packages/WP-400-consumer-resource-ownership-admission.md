# Consumer resource ownership readmission

`index.toml` owns the state of `WP-400-CONSUMER-RESOURCE-OWNERSHIP`. This is a
record of the revised ADR-0013 readmission candidate for the cross-package
successor to the accepted strict Consumer path under ADR-0023. The [scoped specification](../spec/consumer-resource-ownership.md)
and [schema overlay](../resource-contracts/consumer-ownership-v2.toml) own exact
semantics and row/API/diagnostic changes. Production code and existing evidence
records are unchanged.

Production implementation remains stopped pending independent acceptance of the
correction to the [binding/compiler configuration conflict](../../workspace/0086-consumer-owned-artifact-configuration.md).
The [configuration amendment](../spec/consumer-resource-ownership.md#closed-consumer-binding-configuration)
now specifies the missing closed-compiler/API projection, actual-client
correspondence and exact source/evidence scope. `planned/candidate/current`
remains the disposition until independent review of this exact revised boundary.
The original admission is historical evidence, not acceptance of the amendment.
All six completion keys remain outstanding; no schema-2 production result is claimed.

The [accepted PR #180 review](https://github.com/yushun1990/clinkz-wot/pull/180#pullrequestreview-5530769829)
covers candidate `a7d4692fa1bdb4227213ed1940c5ba3976107867` against base
`7d3dfb84a2b9b5385027acbc58684dead01721d7`, with no remaining blocking findings.
Merged master `c145cb94ab65ea43725569675877ba836d5d9b70` has the same tree as
that reviewed candidate. PR #181 recorded its original admission; PR #182
withdrew that admission after the compiler conflict was reproduced.
Completion review is another independent decision after the new source/evidence
exists. HTTP admission and successor accounting measurements remain separate.

## Reconstructed baseline and impact

Reconstruction baseline for this amendment: post-#183 master
`2593392c00013d6f5d32a9c2205fa4a65ec75479`.
Foundation reservations change checked counters; they do not allocate SDK memory.
Core's immutable footprint checks, complete preallocated Host envelope and static
slot preserve accepted owners and both result seals. Planning composes actual
artifact/cursor/erasure/rollback Layouts. Runtime holds original paired accounts,
registration scratch until retirement, conservative complete-call cancellation
and transfer allowances, and residual status before finite final-root disposal.
The current captured registration has synchronous local destruction and no async
retirement hook. These implementation facts motivated an explicit owned scope,
not a replacement allocator or a new retirement framework.

The existing Runtime completion was independently accepted at
`b5517e8d9721084ba005d03c64f3cf32c7a1a79f`
([PR #168 review](https://github.com/yushun1990/clinkz-wot/pull/168#pullrequestreview-5513459509)).
Its registered code/test revision is `cf2e0dec522f0854bcc898e1d437cab6e5fca57b`.
The Consumer gate was independently accepted at
`a22d97e4932c5126f93d7a6fe52cd13d5dc95630`
([PR #171 review](https://github.com/yushun1990/clinkz-wot/pull/171#pullrequestreview-5514507904)).
Use the original evidence files for their exact measurements and exclusions.
Their states remain complete/admitted/current and passed for the strict path.
Admission verification confirms all ten predecessor tranches in the transitive
closure are complete/admitted/current with passed registered evidence, and
WP-000 is complete. The successor uses its own admission to edit only the
registered scope; controlled prototypes cannot substitute for production
completion.

| Existing evidence / owner | Disposition now and on successor implementation |
| --- | --- |
| WP-100 call values | Preserve name-free request/value semantics, validation and constructor recovery. Shared Core error/context changes require renewed return/error Layout and fixed-diagnostic evidence where affected. |
| WP-100 borrowed TD | Preserve semantic inspection/Basic, paid lending, interpretation 2 and source lifetime. No TD source/API change. Rerun applicable native admission/lending and portable dependents; the configuration repair introduces no new TD dependency. |
| WP-200 planning-selection | Preserve complete registration/candidate/artifact identity forwarding, ordering and deterministic selection. Run strict and owned configuration substitution negatives without adding a second projected digest or changing selection algorithms. |
| WP-200 compiler-support | Extend the actual closed compiler and private Host adapter only. Reproduce all strict capacities and generic/wrapped-compiler exclusions. Measure compiler/support/startup adapter, native cursor/output, retained Host slot/outer owners and temporary/transport overlap. Recheck fixed callback/abort/destruction work, allocation failure before native start, same-slot continuation and exact owner recovery. Strict key compatibility does not preserve old Layout measurements. |
| WP-200 aggregate and output-ownership | Preserve source-independent output, all-bounds-before-start, paid progress, exact root/account transfer and bounded rollback. Compiler/artifact storage stays engine-held. Remeasure build/cursor/output/rollback/bridge nodes and registration backing in all cells even if some remain equal; exercise multiple coordinates under one configured owner, identity forwarding, full rollback and source destruction. |
| WP-300 Consumer binding, Consumer-only registration, Host call construction | Preserve selected artifact identity, complete startup composition, both semantic seals, exact input recovery, complete Core-first backing and accepted-overrun retention. New contract capture, backing validation and disposal eligibility require new evidence; old declarations/measurements prove only strict code. |
| WP-400 Consumer Runtime | Preserve atomic install, first cause, generations, normal/late/unread retention, complete transfer, real parent provenance and residual-before-destruction. New reservation scope, scratch declarations and disposal policy need exact source readmission and all affected fault/pressure/layout measurements. |
| Passed Consumer and Producer gates | Preserve original scoped verdicts. Reproduce semantic cases after implementation. Neither verdict certifies schema 2. Before successor composition is claimed accepted, independent completion review must map every affected gate obligation to new exact-revision evidence. Reopen any old gate/tranche only if its original strict contract/source is actually invalidated. |
| Strict schema/profile and old resource traces | Historical quantitative truth remains. Schema 2 resource deltas, identity and live/peak/largest/callback/terminal allocation results are new evidence. Compare semantic owners/leases, not equal SDK bytes. |
| PR #178 and retained Zenoh owner probes | Keep version/configuration observations. Allocating Host disposal alone would no longer disqualify; blocking destruction, lost original tasks, surviving callbacks and uncontrolled ingress still do. No SDK is qualified here. |

Topic 0086's finding is migrated into the configuration owner, API matrix, scope
and prechecks in this amendment. Its production reproduction stays as strict
baseline evidence; migration is not implementation or admission acceptance.
Topic 0081 remains DISCUSSING and intersects the identity boundary. Its specific
constraints are reconciled explicitly: the new key is registration eligibility,
contains no engine quota, Thing/generation/endpoint/account/process identity,
requires no per-Thing compiler or policy copy, and places no owner in the payload.
One registration may compile multiple values; equal format/configuration does
not prove content equivalence. A later separately owned immutable payload remains
possible under a new reviewed physical-ownership contract. Shared programs,
parameterization, cold-start/persistence claims and package extraction are
outside this amendment. This bounded disposition makes 0081 non-blocking here;
any API that contradicts these constraints stops implementation under ADR-0013.
Topics 0076/0082 remain outside the one-shot change. Topic 0083's retirement findings
are retained exclusions; this tranche supplies no Zenoh repair. Topic 0047's
broad authoring projection remains deferred. A new intersecting counterexample
stops source work under ADR-0013 rather than expanding this scope implicitly.

## Exact source/readmission boundary

When implementation changes shared Core declarations, error/seal/carrier Layouts
or Runtime accounting, apply ADR-0013 impact review to the affected completed
tranches and their transitive evidence before treating them as current for that
new revision. Mark intersecting records `review-required` until independent
reaffirmation with new measurements, or reopen them if their original contract
is falsified. The unchanged strict baseline is not reopened by this admission.
The admitted successor may not leave old measurements labelled current for new
Layouts, nor overwrite them as if their schema interpretation had changed.

One successor tranche spans the necessary Core and Servient edits; splitting a
new footprint interpretation from its matching policy, backing checks and funded
cleanup would leave an unusable or unsafe intermediate contract. Keep strict
constructors available for unaffected Consumer/Producer evidence. No compatibility
shim that guesses a contract from equal numbers is permitted.

Allowed production edits after revised admission: Core's closed Consumer compiler
configuration, support descriptor/private Host adapter, passive client metadata,
resource declarations/complete Consumer registration, captured normal/late result
bounds and fixed diagnostics; Servient's checked policy, existing Host/static reservation/disposal paths and
their fixed reports. Update only their direct exports and necessary registered
API rows. A portable allocation-free SHA-256 dependency may be wired only in
Core's manifest and existing locks for the specified fixed encoding; it grants
no compiler extensibility or runtime service. Foundation generation, numeric CSV,
TD and Planning algorithms/public API, compiler input/cursor/artifact types, shared
route/permit semantics, legacy cleanup and protocol implementations are excluded.
Existing predecessor fixture directories may change only for scoped tests,
layout/work/failure measurements and matching lockfile wiring, not alternative
production algorithms or a new admission path.
If an actual implementation needs any excluded source or public semantic change,
amend and independently review the scope first. Actual Layout changes in shared
Core types require Producer regression/measurement evidence even though Producer
semantics remain strict.

The controlled prototype in the existing `consumer-runtime-entry` fixture uses
real Foundation accounts and the real Payload representation with two mock
binding owners. It tests a proposed boundary, not an alternate seal, actual
Servient implementation, production boundedness or measured elasticity benefit.
It needs no new fixture crate, SDK, allocator framework, global gate or process.

## Prechecks and completion evidence

Run the registered prechecks at admission and for affected changes. Schema
checks pin all 196 old rows and their values; verify the exact 20-row overlay, known/disjoint row
membership, supported cells and unchanged complement. Controlled cases must
distinguish complete policy identities when only a non-schema constructor
operand changes; equal ResourceLimits cannot substitute for that complete key.
Pre-admission vectors cover scratch/result/timeout changes, and the checker
separately perturbs all footprint/input/result/work/timeout/scratch scalars.
Controlled cases must also demonstrate private pressure/refusal and reuse,
protected engine cleanup funding, complete backing rejection, single physical
pool ownership/last-user retirement,
and residual-before-allocating-disposal with strict-policy exclusion.

The existing resource validator now checks eight canonical binding-key vectors:
capacity, contract, both alternative compilation cells, private policy and both
result bounds change independently. It verifies all 64 strict namespace keys,
near misses, unsupported cell/operand controls and separation from complete
engine-policy vectors. The overlay metadata pin is revised solely for this
additive configuration encoding; numeric rows, row meanings and previous policy
vectors are unchanged. This is contract evidence, not a replacement compiler.

`resource_ownership_configuration` retains the real strict native/static/Host
rejection and exact-owner controls, extends paid continuation rejection and
executes three target compilations through one complete owner. Equal eligibility
keys yield unequal target payloads; different Thing/plan identities can yield
equal payloads with distinct envelopes. These cases constrain 0081 evolution;
they do not prove cross-generation physical sharing. No precheck claims a
production owned constructor, private-policy correspondence, callback/destructor
bound or changed Layout. Those are defined completion obligations below.

Completion evidence has the stable keys below in
`docs/evidence/WP-400-consumer-resource-ownership.toml`. No result is predeclared
passed and no workload budget is edited to turn a failure into success.

| Key | Required new production evidence |
| --- | --- |
| `consumer-owned-identity` | Run native/static/public supported Host owned construction and real Planning compilation, including the all-bounds barrier and paid continuations. Client, compiler, declarations and registration must correspond for every configuration operand; mismatch returns the same complete owner before operational effects. Prove the private key derives from actual captured policy, remains stable under occupancy changes and changes for behavior-affecting policy changes. Retain arbitrary/wrapped compiler exclusions. Check strict/owned/cell/disposal/result/format/binding/generation substitutions and original Host client address/destructor. Strict keys stay exact for all 64 capacities. Changing any independent engine-policy operand changes its complete policy key without changing an otherwise identical binding key; provisioning still checks full bounds. Missing/zero/exact/one-over/overflow cases retain input and parent totals. Freeze both encodings against the registered vectors. |
| `consumer-owned-handoff` | Actual Core/Servient normal and late results reject excessive retained backing (including spare String capacity) while preserving first cause, accepted call, original leases/accounts and prebacked cleanup. Both preacceptance allocation failures return the same request; postacceptance seal installation allocates zero. |
| `consumer-owned-accounting` | Actual Host/static provisioning, constructor/build/output/call/overlap and final disposal measured by allocation site and owner. Include configuration construction/capture costs and complete compiler/support/adapter/cursor/slot/output/temporary/aggregate/bridge layouts from the impact table, for strict and owned cells; equality to old measurements must be demonstrated. Re-run fault injection at every changed allocation path, including private Host adapter and pre-start slot. Enforce affected local/parent scopes, zero/exact/one-over; no parent release before physical children. Compiler/artifact payloads remain fully counted. |
| `consumer-owned-pressure` | Two independently authored controlled registrations/roots use the same real engine bank and a deliberately small private growth budget. Refusal preserves a live call, unread/late result and cleanup destination; release permits reuse. Optional shared private backing is charged once at its actual owner; cancelling one user preserves another and final retirement waits for all users/jobs. No required shared manager or fleet claim. |
| `consumer-owned-disposal` | Separate engine allocations from finite SDK-local allocations in a controlled Host destructor. Commit residual first; retain all applicable funding through disposal; no progress/join/replacement/detach; report complete only after required local retirement. Test supported allocation errors and strict-policy rejection. A process-termination probe, if used, proves only that boundary. |
| `consumer-owned-regression` | Reproduce the existing Consumer gate's 19 commands and applicable Producer/Foundation/portable regressions. Map unchanged semantic case IDs and new resource deltas to exact revisions, then obtain independent acceptance of successor composition. ARM compile cells remain layout/surface evidence only. |

## State, workload and result projection

No state or transition is added to `compiled-plan-set` or `in-flight`. Existing
publication, first-cause, late-result, complete transfer and terminal-ack traces
keep their semantic oracle. New resource traces carry the explicit contract key
and map every reserve/release to its real engine/private owner. Changed byte
deltas are not parity failures; a lost owner, premature parent release or new
Drop-to-success transition is. No private SDK pool becomes an engine state machine.

| Existing performance contract | Exact disposition |
| --- | --- |
| `PERF-GW-009/010` v2 | Preserve metadata fixture identity and engine allocation budgets. Use matching operations as regressions; remeasure changed complete envelopes. SDK-private allocation counts are reported separately, not subtracted from an old all-allocation result. |
| `PERF-GW-028` v1 | Preserve cancellation matrix, owner-loss/late-value/transfer/lock budgets and strict fixtures. Consumer invoke cases apply here; this tranche cannot claim its subscribe/response/emission matrix complete. |
| `PERF-GW-029`, `PERF-CS-021` v1 | Preserve plan lifecycle, generation and zero-retained-owner oracles. Re-run eager one-shot cases and affected physical Layouts; lazy/whole-matrix evidence remains separate. |
| `PERF-CS-020` v1 | Preserve typed slot/alignment/acknowledgement and allocation budgets. Consumer request/cleanup cases apply; no subscription/server/emission qualification follows. |

All six keep their current manifests, fixture identities and budgets for their
original scope. The 66-case verifier checks contracts, not production measurements.
`result.schema.json` v2 has no ownership-contract identity and is **not** a schema-2
result carrier: do not emit an owned measurement as a passed strict-profile result
or hide its identity in free-form notes. No release workload is reinterpreted here.

New scoped observations live in the successor completion TOML under the six keys
above, with exact source/test refs, schema revision 2, typed-input revision 2,
operation/cell/disposal/profile/value identity, all independent policy constructor
settings and their complete key, fixture source digest and actual
command. Separate measured engine/private/deployment storage and allocation
counts; record current/peak/largest/overlap and terminal release against the
configured field/owner budgets and zero-loss/zero-postacceptance-engine-allocation
oracles above. Those finite observations do not claim a full release-performance
matrix. Promoting owned measurements into release performance requires its own
versioned manifest/fixture/result-schema projection before that claim.
No latency, hard RAM, allocator-metadata or whole-device claim follows here.

## Remaining blocks

Recommendation: accept the revised bounded configuration/API/scope amendment
for readmission after independent review of its exact revision. This resolves
0086 at the contract boundary and reconciles 0081 without requiring shared
programs. The reviewer must reconstruct the contracts, exact diff and executable
evidence independently and post findings as **GitHub PR review comments**.
Only that independent acceptance may authorize the candidate's admission
transition; the author does not resume production from passing prechecks.
No new precursor tranche or global gate is required.
Successor completion is blocked on new production evidence above.
HTTP Consumer admission then consumes that accepted shared boundary and matching
predecessors; HTTP method/response metadata eligibility, SDK selection/pinning,
real peers and input/overload/cancellation/disposal qualification remain in its
own tranche. Producer HTTP, HTTPS/security, embedded production, broad WP-100/
200/300/400/600 closure and ObserveProperty remain outside this admission.
