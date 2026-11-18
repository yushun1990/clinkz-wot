# Preparation, instance binding and interaction execution

Status: MIGRATED

## Reconstructed question

Baseline: `090e26c` (after #287). PREP-SELECT-001/PREP-REUSE-001 place
complete actual capability qualification in preparation; SAFE-AUTH-001 and
EXEC-SECURITY-001 retain live instance and policy checks. Reusable meaning alone
does not separate those responsibilities. This investigation uses source paths
and correctness counterexamples, without benchmark evidence.

| Path | Actual preparation and binding | Execution before this correction | Necessary dynamic work / misplaced invariant work |
| --- | --- | --- | --- |
| Finite Consumer | `planning/src/open_consumer.rs::prepare_inner` interprets Forms and selects actual compiler, schemas, media, security and execution; `core/src/consumer.rs::ConsumerDefinition::try_new` qualifies the mapping; `bind` captures access revision/work and bank; `StandaloneConsumerAssociation::try_new` attaches original logical authority | `consumer.rs::start_with_variables` searches mappings, checks declarations, resolves destination, authorizes, validates actual input, issues credentials and starts the sealed native call; `FiniteCallState` and `CallWork` recheck through progress, validation and unread handoff | Name selection is necessary when the caller supplies a name, but need not recur for a retained selection. Fixed-target resolution and complete `supports_data`/operation/URI eligibility scans are already-known facts. Actual variables, payload/selection, capacity, time, authority, dependency validity, permission, credential issuance and native result identity/shape stay live. |
| Continuing Consumer Action | `planning/src/open_consumer/action.rs` prepares independent Invoke/Query/Cancel mappings and data roles; `ConsumerActionDefinition::check_dependencies` qualifies schema roles and continuation compatibility; `ConsumerActionAssociation::try_new` captures per-operation access owners | `consumer_action.rs::Association::check_dependencies/check_operation`, `Selected::check_services/check_data` repeat support; `Selected::check` resolves the destination for the actual continuation and calls policy; the common Action kernel retains original invocation and exchange ownership | Rechecking the whole sibling inventory and asking schema/continuation compatibility again is invariant qualification. A new Query/Cancel selects its original mapping, but accepted work must not depend on unused siblings. Actual continuation destination, control intent, authority, operation-specific live dependencies and each returned output loan's validation remain necessary. |
| Consumer Notification | `planning/src/open_consumer/notification.rs` prepares owned program/configuration and schema roles; `ConsumerNotificationDefinition` qualifies schema support/native support; association captures access and bank | `consumer_notification.rs::CapturedServices::check_dependencies` calls the definition's complete schema/native support check around subscription, codec and policy callbacks; raw conversations independently authorize establishment, queued delivery, response and stop | Program/kind/schema support and unchanged security requirements need not be requalified for every delivery. Actual occurrence identity/member, payload, source proof when supported, recipient, permission, loss/capacity and original dependency validity do. A topic is no substitute for provenance. |
| Producer | `planning/src/producer_composition.rs::check_final_thing`, concrete providers and `publication.rs` qualify the entire final retained TD, actual routes/behaviors/services and closed native participation before local publication | `host.rs::poll_dispatch` resolves native candidates to a current instance/behavior, verifies ingress, validates actual input, authorizes, captures behavior and validates output; `pair_is_current` checks original identity/configuration/work; continuing Actions and notifications keep separate stores and emission authority | Final-TD coverage/readiness/collision qualification belongs before publication. Native candidates are per-ingress facts: resolving one to current semantic authority is necessary, as are authentication, payload validation and policy. Once accepted, the captured behavior must remain original. Publication inventories are not executable dispatch permission. No evidence here justifies eliminating those live checks or redesigning native matching. |

The caller-owned Read/Action paths also separate qualification from common
state transitions, but prove different storage representations. Their existence
does not make Host `check_dependencies` a cheap or properly scoped operation.
The Producer trace also exposes a concrete static traversal:
`HttpFiniteBinding::additional_validation_work` walks all mappings and their
schema/response roles whenever `host.rs::pair_is_current` asks for the declaration.
The declaration's validity must stay live, but recomputing its immutable
inventory is not required by that check. `program_for(candidate)` is separately
repeated for input validation, response-contract access and output validation.
Initial selection from actual ingress is necessary; its accepted mapping can
then be retained with the original work. Final publication cannot preselect an
unknown incoming recipient, and a raw caller's supplied candidate still needs
validation. Notification's per-subscription search for any unsupported named
subscription role is likewise invariant for an unchanged prepared collection.
HTTP's own artifact parsing/address verification remains concrete Binding work;
this task does not optimize it or prescribe a universal native artifact.

## Decision and alternatives

Retain the result of qualification with an explicit invalidation contract,
instead of repeating the qualification algorithm. Finite data/native capability
answers are stable within a non-reused qualification revision. Availability and
original resource declarations remain separate passive checks. Access already
has a non-reused security revision, which also covers security support. Changed
support invalidates the old definition; binding cannot refresh it. A revision
is dependency evidence, never permission or a cache key.

Add one finite Property Read binding that retains the selected mapping,
original association authority, instance bank and fixed resolved destination.
Its calls use the existing finite admission, native sealing, progress, result
and cleanup implementation. Variable destinations are resolved per interaction.
Name-based convenience calls still perform name selection. Dropping the handle
does not revoke; removing/replacing its association or parent does.

| Alternative | Disposition |
| --- | --- |
| Keep rescanning because callbacks can invalidate dependencies | Conflates detecting change with proving capability again, and repeatedly traverses immutable schemas. Revisions plus original declaration checks preserve invalidation. |
| Delete checks and declare everything permanently immutable | Cannot represent actual service revocation or changing supported capabilities. Fixed services can use revision zero; mutable providers must propagate change. |
| Cache answers keyed by TD/name/configuration | Adds lookup, eviction and lifetime questions; does not itself establish live dependency validity or original authority. |
| Prebind all names/eagerly prepare all families | Unnecessary for one selected operation and may retain unused owners. |
| Universal executable plan/VM/Runtime or scheduler | No missing execution instruction model is established by this defect. The existing call kernel and actual Binding artifacts suffice. |
| Migrate Action/Notification/Producer together | Adds continuation, occurrence and publication boundaries without improving the finite counterexample. Their traced defects remain follow-up scope, not an inferred all-family acceptance. |

## Migration and falsifiable boundary

The detailed contracts migrate to Planning, Binding SPI and Runtime Safety.
WP-200/WP-400 record the focused implementation/evidence. Qualification counters
and zero lookup-credit calls must prove no repeated support/selection work;
real HTTP and independent datagram calls must preserve actual Binding execution.
Changed revisions/declarations, callback-time invalidation, denial, replacement,
short work/funding, late/unread results and final release must still be checked.
No milestone/gate status or general runtime/profile/reuse claim follows.
