# 0072 Consumer ValidatedThing Build Lifetime

Status: MIGRATED

Kind: reopened architecture decision and docs-only authority migration

Assessment baseline: `65dfdd6e4b3e47c6ee9c22c0085087e634564287`

Reopened projections:

- workspace topic 0063's requirement that the published Consumer record retain
  the validated source owner and convert its source charge to persistent-
  document accounting; and
- workspace topic 0065's downstream handoff conclusion that Servient retain the
  normalized `ValidatedThing` for the published generation.

The normalized representation, three-arena prototype, storage-neutral semantic
kernel, and borrowed Planning view selected by 0065 remain valid. This topic
changes only how long that proof owner lives in the first Consumer Property Read
path and which resource account owns it at that boundary.

## Decision

For the first Consumer Property Read aggregate, `ValidatedThing` is a build-
scoped TD proof owner. Planning may borrow `ValidatedThingView` only while it
constructs the complete owned aggregate. A Published consumed record retains:

- the immutable plans, lookup, candidates, and eager binding artifacts required
  for execution;
- one complete matching Consumer-capable Property Read binding registration;
- generation, lease, drain, cleanup, and other lifecycle records; and
- only the resource records and committed charges for that published runtime
  material.

It retains no `ValidatedThing`, normalized snapshot arena, TD view, source
charge, persistent-document charge for that snapshot, or capability to recover
or reinterpret the TD.

This is a conformance correction under the already active `DOC-RUNTIME-001`,
`ADMIT-MEM-001`, `ADMIT-TXN-001`, `PLAN-ARTIFACT-001`, and `PLAN-SET-001`
contracts. In particular, `ADMIT-MEM-001` already requires phase-local storage
to be released at the earliest safe boundary and forbids atomic publication
from retaining every phase representation. No requirement identity, active
v5.1 count, package dependency, milestone, or gate status changes. A new ADR is
not needed because this migration removes a contradicted workspace-derived
retention choice and projects the existing active ownership/resource rules; it
does not introduce a new cross-domain authority identity.

## Exact handoff and publication order

The order is strict:

1. TD completes normalization, Basic/equivalence proof, and the build-scoped
   `ValidatedThing` with its exact structured footprint and source ledger.
2. Planning borrows `ValidatedThingView` for preflight, complete coordinate
   materialization, every compiler bound, the all-bounds-before-start barrier,
   sequential eager compilation, lookup sealing, and final owned-draft sealing.
3. The last `ValidatedThingView` and every nested Property/Form/security view
   are dropped. No later check, callback, or publication operation may borrow
   them.
4. The owned `ConsumerPropertyReadDraft` is proved TD-lifetime-free: all plans,
   lookup rows, candidate records, artifacts, identities, diagnostics needed at
   runtime, and exact runtime footprint are owned and remain usable after the
   `ValidatedThing` is destroyed. The complete registration is independently
   owned by the Servient transaction rather than borrowed through the view.
5. Servient performs every fallible final check while the Snapshot still exists:
   aggregate completeness and counts, plan/artifact/registration identity and
   generation consistency, exact runtime-resource reconciliation, absence of a
   compiler/build cursor or temporary allocation, publication-slot ownership,
   and the final cancellation check. Success closes cancellation against this
   publication and yields a private single-use publication permit. No public
   state changes yet.
6. Using the already copied `ValidatedThingFootprint`, Servient destroys
   `ValidatedThing`. Its fixed arena drop first deallocates every Snapshot arena
   and releases each child-ledger source charge exactly once; only after those
   physical bytes are no longer live does Servient release the matching parent/
   global source allowance. The persistent-document account remains zero for
   this Snapshot.
7. The permit is consumed by one allocation-free, callback-free, non-yielding,
   generation-checked atomic install of the already allocated runtime record and
   consumed-handle registry entry. Only this step makes the generation
   Published and opens plan-lease acquisition.

The private permit is not another public lifecycle state. It may be issued only
when the reserved registry slot and all record storage make the final install
infallible. If an implementation cannot guarantee that property, it must keep
the Snapshot and cancellation ownership and fail before issuing the permit; it
may not add a fallible gap after Snapshot release.

## Ownership, resources, and rollback

The complete build still charges every normalized allocation. The three sealed
arenas remain in the child ledger's source account, and the parent/global source
allowance remains held, until step 6. Mutable build arenas, traversal storage,
grow/seal overlap, allocation count, largest actual request, conversion peak,
and lifetime work retain their existing accounting. Plan, artifact, lookup,
registration-owner, record, and lifecycle storage is separately reserved and
reconciled under its existing compiled-runtime and owning capacity accounts.

The peak before publication therefore includes the simultaneous Snapshot and
complete runtime-plan material. Releasing the Snapshot does not retroactively
lower that observed peak. It deallocates exactly the Snapshot's live requested
bytes and subtracts their child source usage before releasing the matching
outer allowance; it does not move those bytes to persistent-document
accounting. The Published record starts with zero Snapshot source bytes and
zero Snapshot persistent-document bytes, while its compiled-runtime,
registration, lifecycle, diagnostic, and cleanup charges remain committed under
their existing owners.

The no-dangling-reference proof is structural as well as behavioral:

- public Planning output has no lifetime parameter derived from
  `ValidatedThingView` and contains no raw arena range or storage offset;
- binding artifacts and plan diagnostics copy or own every runtime fact they
  require;
- the registration owner is retained separately and is joined by checked
  identity/generation, not by a borrow hidden in an artifact; and
- an external compile fixture must drop `ValidatedThing` before selecting from
  and exercising the sealed draft/runtime record.

Failure handling is split at the permit boundary:

- before permit issuance, any validation, materialization, bounds, compiler,
  reconciliation, seal, identity, resource, cancellation, or publication-slot
  failure fixes the first cause, destroys the Snapshot, and rolls back every
  uncommitted plan/runtime/source reservation with no published record;
- permit issuance occurs only after all such checks and makes the remaining
  release-and-install sequence non-fallible and non-cancellable; and
- after Snapshot release there is no retry, callback, allocation, semantic
  query, compiler progress, or alternative publication path. An unpublished
  independently owned plan may still be destroyed if control aborts before a
  permit exists, but it never needs the Snapshot to roll back.

## Frozen API audit

Three methods were tied only to the superseded persistent-Snapshot handoff:

| Method | Independent admitted use | Disposition |
| --- | --- | --- |
| `ValidatedThing::reclassify_source_to_persistent_document` | None. Its only specified caller was first Consumer publication. | Remove from the frozen target API. |
| `ValidatedThing::retained_source_bytes` | None beyond the structured `footprint().retained_requested_bytes()` value used for build reconciliation and release. | Remove the redundant alias; retain `footprint()`. |
| `AdmissionLedger::reclassify_source_to_persistent_document` | No current production caller or separately admitted lifecycle requires this narrow transfer. It was added for the same Consumer handoff. | Mark as an old-API removal for the future admitted WP-100 source change; this docs-only migration does not edit Foundation Rust. |

`ValidatedThing::footprint`, every structured footprint accessor,
`property_count`, `readable_property_form_count`, and the borrowed view retain
independent build-time uses and remain frozen. Generic source, temporary,
persistent-document, and persistent-runtime reserve/release operations also
remain Foundation authority; only the narrow source-to-persistent transfer is
orphaned.

## Authority migration map

| Owner | Migration |
| --- | --- |
| `docs/architecture/10-primary-data-flows.md` | Freeze the last-view, independent-draft, final-check, Snapshot-release, atomic-publication order. |
| `docs/architecture/20-module-boundaries.md` | Make TD/Planning ownership build-scoped and restrict Published Servient ownership to execution material, complete registration, lifecycle, and resource records. |
| `docs/architecture/30-compiled-plan-lifecycle.md` | Remove Snapshot retention/reclamation and define the private publish-permit boundary. |
| `docs/architecture/50-servient-runtime-lifecycle.md` | Project Host/static release and publication semantics without merging their containers. |
| `docs/spec/foundation.md` | Preserve full build accounting and replace source-to-persistent conversion with exact source release before publication. |
| `docs/spec/runtime-safety.md` | Make the normalized owner build-scoped and freeze the no-fallible-gap rule. |
| `docs/spec/planning.md` | Require an owned TD-lifetime-free aggregate usable after `ValidatedThing` drop. |
| `docs/state-machines.toml` | Clarify that normalization `Complete` emits a build proof and that Consumer publication follows permit, Snapshot release, then atomic install. |
| `docs/api-ownership.csv` | Remove the two orphaned `ValidatedThing` methods from the target surface and mark the narrow Foundation transfer for later removal. |
| `docs/work-packages/WP-100-consumer-validated-thing-admission.md` and `WP-100-core.md` | Preserve normalized construction/accounting while changing the downstream lifetime, API removals, and readmission impact. |
| `docs/work-packages/WP-200-planning.md` | Strengthen future aggregate independence evidence without changing the completed exact-coordinate tranche. |
| `docs/work-packages/WP-400-servient.md` | Replace persistent-Snapshot retention with final-check/release/publication and future failure/resource evidence. |
| `docs/work-packages/index.toml` | Update only the planned/candidate WP-100 boundary and API/removal lists; keep its status and admission state unchanged. |

`docs/resource-limits.csv` needs no row change. The existing source, temporary,
peak, largest-contiguous, compiled-runtime, cleanup, and work limits cover the
overlap and release sequence. `docs/requirements.csv`, ADR-0019, PLAN, the
passed Producer gate, and all existing completion evidence remain unchanged.

## Eight-item readmission impact

All eight WP-100 pre-readmission items remain required and incomplete as a set.
This migration neither satisfies nor waives one:

1. **Public surface:** changed. Compile-only/compile-fail evidence must reflect
   removal of the two `ValidatedThing` methods and the eventual narrow
   Foundation transfer removal; the remaining construction/view API is
   unchanged.
2. **Allocation catalog:** unchanged. The current three retained and four
   temporary site proof, exact checked formulas, grow/seal overlap, and one-
   `Layout`-per-reservation obligations remain.
3. **Semantic equivalence:** unchanged. The fieldwise corpus and numeric Basic
   agreement remain necessary.
4. **Planning view:** strengthened, not completed. Besides the PR #111 non-first
   coordinate query, future evidence must build owned output, end every view
   borrow, drop `ValidatedThing`, and continue using that output without TD
   parsing or copied rules.
5. **Feature matrix:** unchanged. Host/static, capability off/on, downstream
   unification, and actual thumb cells remain required.
6. **Progress/cancellation:** unchanged for normalization. Snapshot destruction
   still uses the prepaid fixed allocation catalog; the later Servient publish
   permit is WP-400 integration evidence, not a new WP-100 progress terminal.
7. **Resource proof:** changed. It must prove complete source/temporary/peak
   accounting through simultaneous plan residency, exact source release with
   zero persistent-document conversion, zero Snapshot charge in Published, and
   no aggregate-as-contiguous accounting.
8. **Impact reaffirmation:** changed only in disposition. PR #69's work classes
   and ledger behavior remain historical/current source evidence, but its now-
   orphaned narrow transfer must be identified for future removal. PR #70, the
   completed WP-200/WP-300 evidence, resource schema, and passed Producer gate
   remain unchanged unless their own exact-head impact review finds otherwise.

## Exact-length sealed Snapshot impact

The shorter lifetime removes the runtime reason to keep exact-length arenas
after publication; it does not by itself prove that sealing is unnecessary
during construction. The current accepted boundary still uses exact-length
node, edge, and byte arenas because they provide an immutable allocation-free
semantic view, a fixed allocation catalog, deterministic requested-byte truth,
and bounded fixed-shape destruction while Planning borrows the Snapshot. The
merged three-arena and shared-semantic-kernel prototypes remain valid evidence
for those build-time properties.

Conceptually, a future build-only representation could retain charged observed
capacity instead of exact length if it still proved semantic immutability,
allocation-free views, one-`Layout` accounting, peak overlap, fixed bounded
cleanup, and plan independence. That alternative might remove seal-copy work
or might increase admitted source/peak capacity; current evidence does not yet
establish the trade. Changing it would affect readmission items 2, 4, 6, and 7
and the normalization state machine. This PR therefore records the impact only:
exact-length sealing remains the frozen WP-100/readmission contract and no
representation, allocation-site, or API expansion is authorized here.

## Migration disposition

This docs-only migration changes no production source, Cargo manifest,
resource row, feature graph, work-package admission state, existing evidence
manifest, or gate status. It does not readmit WP-100, admit WP-200/WP-400,
register the Consumer architecture gate, or revise PR #111's limited prototype
claim. The exact authoritative projections listed above now own the changed
lifecycle; topics 0063 and 0065 remain historical rationale with their retained-
publication projection superseded by this topic.
