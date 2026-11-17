# Continuing Action execution across ownership representations

Status: MIGRATED

## Reconstructed gap

Baseline: `dbf904527d3d727c10a5d760a0d468db52c7b6cd` after #286. Topic 0081's migrated authority is
PREP-REUSE-001, ADR-0025 and the compiled-definition lifecycle; its historical
Plan-Set proposal is not an implementation contract. SAFE-PROFILE-001 requires
equivalent supported semantics while permitting distinct storage and driving.

Four independent questions must be tested:

| Dimension | Existing evidence | Remaining limit |
| --- | --- | --- |
| Compilation reuse | Explicit HTTP definition lowering and checked bind-many; specialized Action/notification definitions can also be retained | No general cross-family, cross-endpoint or cross-environment native artifact contract |
| Logical independence | Host definitions/associations/capacities, original issuer identity, Producer publication and native infrastructure separation | Sharing code or preparation alone grants no independent live authority |
| Executor independence | Manual progress, optional Host turns and deployment-supplied clocks/wakes | Host loops do not establish alternative storage or wake capability |
| Runtime/storage representation independence | Common finite kernel; caller-owned finite Read with real datagrams and no-atomic compilation | Continuing Consumer Action, notification and Producer ownership still require Host carriers |

`core::consumer_action` is std/pointer-atomic gated and retains boxed native
exchanges/Any tokens, Arc data services and Host account funding. Servient's
continuing implementation adds Rc invocation/control identity, RefCell slots,
Vec capacity and Waker interest. #286's typed Producer turns preserve those
owners. Their executor independence is valuable but does not remove them.

The existing finite kernel is not an appropriate continuing lifetime: native
settlement concludes one finite call, whereas Action acceptance retains a
continuation through later explicitly selected exchanges. A copied application
loop or a no_std flag would leave that semantic boundary untested.

## Selected increment and alternatives

Extract the actual Host Consumer Action transition algorithm into a state-only
Core kernel, and execute it with a separately funded caller-owned Action slot.
Keep concrete native work, tokens, input/results, services and authority in each
representation. Share prepared ValueSchemas, output shape and validator meaning.
The caller-owned slice is source-free NoSec Invoke with optional Query/Cancel;
it does not port TD preparation or a production transport. A downstream native
datagram bundle tests continuing execution and original physical owners.

| Alternative | Disposition |
| --- | --- |
| Another finite operation | Does not expose continuation/exchange lifetime separation |
| Wrap Host Action in a borrowed facade | Retains the representation requirements under investigation |
| Generalize FiniteCallState | Conflates exchange settlement with continuing invocation settlement |
| Port notification or Producer first | Credible later increments; would add publication/distribution responsibilities without isolating this Action counterexample |
| Universal Runtime, scheduler or mutable work registry | Unnecessary; exclusive caller-owned progress and the existing Host owner can execute the same semantic transitions |
| One native artifact ABI for all environments | Unnecessary; compatibility is checked within each actual Binding representation |

## Falsifiable boundary

Both owners must execute the common kernel. Real Invoke acceptance, pending
Query, remote Cancel uncertainty/acknowledgement and later completion must keep
one original invocation, independent permission and result ownership. Local
cancellation/replacement must preserve first cause, native poll error, Pending
and failed close, original late output/validation, and final local release.
Short credit must enter no corresponding callback; an unread completion requires
fresh permission at handoff. Same-slot foreign/reused identities cannot control
old work. Retained results must keep their original lease through disposal.

Constrained TD preparation, notifications, Producers, credentials, concrete
embedded I/O/wakes, arbitrary schema storage, general native reuse and production
qualification remain open. This increment cannot close 0081, runtime independence,
a milestone or an acceptance gate. The [runtime contract](../docs/spec/runtime-safety.md#caller-owned-continuing-action)
and [evidence](../docs/evidence/continuing-action-runtime-independence.md) own the
migrated scope and remaining limits; this file preserves investigation rationale.

## Output loan reassessment

At `fb8dee594f2df595cadcf01a8ad17317dad53f0c`, source and an executable datagram
case falsify the assumption that retaining an `ActionOutput` owner seals its
validated view. A safe `Cell` selector changes from the native schema-valid `7`
to `99` in authorization after validation; a later `result()` calls the facet
again and exposes unvalidated bytes. The same gap affects media/status/metadata.

| Alternative | Disposition |
| --- | --- |
| Require stability through documentation or a marker trait | Insufficient as the sole enforcement for the public safe-Rust view facet |
| Validate only again at owned settlement transfer | A subsequent view can still differ before application access |
| Retain the view inside the movable generic owner | Introduces a self-reference for inline outputs or couples result lifetime to the work slot; neither is needed |
| Copy into a fixed immutable output representation | Possible with separately funded backing, but unnecessary and restrictive for native owned/borrowed/inline representations |
| Validate each exact application loan under paid original bounds | Selected: retain the movable original owner, capture/validate/return one view at `result(budget)`, keep native progress validation and existing ownership/cleanup |

The selected contract distinguishes permission/owner transfer from validation
of a particular application loan. It guarantees each returned loan matches its
own validation, not equality of every historical native view. Loan errors retain
original evidence/funding and first cause; already-returned borrows remain valid.
`input()` similarly lends the retained original input owner's current view and
does not manufacture a dispatched-payload receipt. These semantics migrate to
the existing Runtime Safety/Core/Binding owners and evidence; no new gate,
milestone, universal Runtime, scheduler or copying requirement follows.
