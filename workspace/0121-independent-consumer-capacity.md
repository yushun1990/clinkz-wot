# Persistent Consumer associations and finite capacity

Status: MIGRATED

Review 10's independent-capacity direction is implemented first through the
existing finite HTTP path. The relevant obligations are independent: revoking a
logical association must fence its work without destroying a sibling, retaining
prepared readiness requires no vacant Call Slot, and an unread result still
requires its original backing after the execution slot is reused.

The selected boundary replaces the bundled `ConsumerRuntime` constructor with
passive `ConsumerAssociation` views and explicitly supplied `ConsumerCalls`. The finite slot
registry survives as a work owner because its reserved-slot checkout, passive
controls, unwind restoration and orphan cleanup already implement useful finite
mechanisms. It no longer owns one instance or its definition inventory. The
association owns checked policy/mapping references and passive root authority;
selected binding records fund their own lifetime rather than pinning unselected
mappings through a whole-root reference. Work snapshots original operands and
retains its work bank and any separate instance outstanding-call debit.

Compared alternatives: constructing the old root for each call still bundles
root loss with work and repeats binding/storage construction. Growing an
association-local call registry on demand removes idle allocation but preserves
the capacity topology. Returning only self-contained call handles would simplify
storage but remove the existing independent cleanup owner after handle loss.
A universal executor or mandatory fleet pool adds no needed capability. An
explicit finite work owner permits dedicated, shared or on-demand capacity while
retaining the already qualified finite lifecycle; caller-owned finite Read remains
a separate supported representation.

Cross-family challenge: a continuing Action must retain invoke/control mappings,
continuation and future control/result/cleanup funding after its initial exchange;
returning a finite slot is not that invocation's retirement. An established
notification conversation needs continuing cursor, loss, stop and response state
without traffic; passive association readiness is not an active subscription or
an occurrence-policy expression. A Producer exposure requires complete final-TD
qualification, behavior and closed native readiness before publication; neither
finite association construction nor spare work capacity establishes that claim.
The shared ConsumerInstance identity remains usable across facets, while these
owners remain family-specific. No common session or work-state hierarchy is
introduced. F1/F2/F3 remain outside this finite handoff.

The exact contracts are migrated into Runtime Safety, Foundation, Core, Planning,
Binding SPI and their existing architecture/API/state/work-package projections.
WP-400 links the real HTTP capability regressions. Historical scaling measurements
remain historical, not evidence for the replacement layout. No new governance
layer, architecture gate, milestone transition or acceptance claim is added.

## Managed lifecycle and explicit progress

The independent split exposed two separate questions: what owns registration
liveness, and what preserves cleanup when a work-controller facade disappears.
The selected lifecycle keeps registered instance/facet authority in the actual
logical Servient owner. Application handles are non-revoking; explicit APIs
remove/replace registrations. A standalone move-only owner remains useful for
raw composition and is consumed, not cloned, on managed adoption.

Automatic authority-to-driver notification was evaluated against explicit
lifecycle-command scheduling and cyclic/timed progress. The silent-peer evidence
proves that I/O-only scheduling misses passive changes; it does not establish a
universal requirement to discover/wake independent drivers from ordinary Drop.
The native indication registry, eventfd backend and concurrent maintenance/accounting
are withdrawn from this finite increment. Retaining them behind an unused feature
would preserve both unnecessary complexity and known defects. A future automatic
runtime capability needs its own concrete requirement and qualified evidence.

Explicit lifecycle changes fence original permission before releasing mappings,
while accepted calls and settlements keep original context/accounts. Unselected
mappings retire outside borrows; retained handles keep only funded tombstones.
The application schedules original work after lifecycle changes and abandonment,
with actual timer wakes where needed; logical deadlines install no timer.
This preserves independent work-store cardinality without an Engine scheduler.

The cleanup-reachability correction remains independent and necessary: losing the
work-controller facade vacates and locally disposes abandoned original calls;
checked-out work retains its lease/restoration duty. An unrelated surviving handle
cannot strand abandoned work. Finite local abort is a complete provider conformance
obligation, not a guarantee inferred from HTTP or a metadata tag.

Detailed contracts belong to Runtime Safety, Foundation and Core; this topic
records the decision rationale and alternative dispositions, not current task/CI state.

## Public addressing and management recovery

Keeping registrations after handle loss exposed a concrete missing capability:
the owner had no public way to reacquire targeted access. Merely requiring an
external handle table would preserve that API gap. The selected finite owner now
resolves TD IDs in its own registry and configurations by explicit local name.
Ordinary operations/removal need no persistent snapshots. Snapshots remain useful
for original-revision-conditioned management; internal generations cannot infer
the intent of a delayed unqualified ID command.

Different access policies do not inherently create different public Things.
Named associations under the same authority preserve configuration selection
without union/fallback; update fences that association while original accepted
work keeps its context. The TD-aware facade rejects absent IDs only for this
named registration path. General anonymous TD support and low-level composition
remain valid. A universal multi-family Engine facade, inferred configuration
selection and automatic scheduling add no capability needed for this correction.
Runtime Safety owns the exact scope, duplicate, update and lookup contracts.

## Indexed addressing and independent slot allocation

The initial ID lookup charged and scanned every registered instance, and eager
configured-maximum storage defeated sparse startup. Automatic `max(slot) + 1`
also refused after a caller registered `u32::MAX` despite available capacity.
These are concrete API/resource defects, not reasons to restore handle-only access.

The selected correction uses the primary records themselves as a sorted ID
table, plus a sorted local index of the same passive identities by slot. Named
configurations are sorted within each instance. Binary lookup has deterministic
comparison bounds even for adversarial common-prefix IDs. First-hole search on
the slot index decouples allocation from arbitrary caller coordinates; no new
public Thing identity is introduced. Both indexes are committed before linked
destructors can reenter, and actual contiguous capacity stays funded.

Hash tables offer average constant lookup but require collision/probe and backing
qualification. Balanced trees improve management-update complexity but add node
storage/allocation accounting. Sorted contiguous records are the smaller sound
choice for this finite path: logarithmic admission lookup, no row-number aliases,
explicitly paid linear insertion/removal, and lazy accounted growth. This does
not qualify high-rate mutation over very large populations; a later concrete
requirement could justify another index with its own work/memory proof. Current
lookup is corrected here rather than deferred to measurement or larger budgets.
