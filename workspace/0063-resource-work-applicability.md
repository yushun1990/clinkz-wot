# 0063 — Consumer aggregate resource/work applicability

Status: DISCUSSING

This note is the scoped resource companion to workspace/0063. It classifies
the existing `docs/resource-limits.csv` schema against the replacement
Stage-A model. It neither changes that authoritative table nor proposes final
limit names.

## Accounting model

Consumer aggregate admission must preserve the six accepted admission-memory
accounts rather than collapse them into "temporary" and "persistent":

1. retained source;
2. temporary parse/validation/build;
3. persistent effective document;
4. persistent runtime material;
5. diagnostic material; and
6. cleanup state.

It must also preserve total live bytes, peak live bytes, largest single
allocation, and typed work. A reservation is owned by the Servient transaction
until it is either committed into Frozen ownership or released during private
failure settlement.

For direct typed input there is no parse buffer, but the owned `Thing` is still
retained source while validation and planning borrow it. Its source charge is
transferred into the transaction and released before Frozen.

## Applicability terms

- **Active** — directly required by the first aggregate-admission proof.
- **ZeroContribution** — the concept applies, but the first proof contributes a
  measured zero and must not reserve invented storage.
- **Deferred** — belongs to later runtime or multi-candidate work, not Stage A.
- **NotApplicable** — tied to a different input representation or operation.

## Source and validation accounts

| Existing family | Stage-A classification | Reason |
|---|---|---|
| `retained_source_bytes_*` | Active | The owned typed TD is live through validation and Planning, then released before Frozen. |
| `document_bytes_max`, `string_bytes_max`, and `extension_bytes_max` | Active | They bound the transferred typed document and its owned semantic content, including extension material. |
| `json_*` lexical/token/depth rows | NotApplicable | No JSON text parser participates in the direct typed-input path. A JSON convenience path must account for these before producing the validated typed value. |
| affordance/form/additional-response, schema, URI-source, and security-shape rows | Active | Basic validation and the typed census must traverse these even when the narrow plan projection later discards them. |
| `generated_effective_document_*` | ZeroContribution | Stage A does not synthesize or retain a second effective TD. |
| admission temporary/peak rows plus engine-live and largest-allocation rows | Active | Validation traversal, coordinate projections, compiler cursors, and compiler temporaries have bounded lifetimes and measurable peaks. |

The authoritative schema already contains several representation-neutral
semantic caps, including strings, extensions, affordances, forms, schema
nodes, security depth, and URI structure. It does not yet provide a complete
typed-TD census contract or all needed categories. A later accepted design
needs a coherent family covering at least:

- total owned string/blob bytes and largest owned value;
- property, action, event, form, link, security-definition, schema-node, and
  additional-response counts;
- collection lengths and nesting depth; and
- extension keys and values at every supported extension point.

There is also no explicit retained-source document-count row. The first proof's
exactly-one-document rule is therefore a transaction-shape invariant, while
its bytes use the existing per-owner and global retained-source limits.

Extensions are counted even when Consumer Property Read semantics do not
inspect them. "Ignored by planning" is not equivalent to "free to validate,
traverse, retain, or drop." The census is validation evidence and reservation
input, not a second semantic TD model.

## Persistent aggregate accounts

| Aggregate category / current coverage | Stage-A classification | Frozen owner |
|---|---|---|
| `compiled_plan_bytes_max` and `logical_plan_bytes_per_thing_max` | Active | Sealed plans plus aggregate tables under one reconciled compiled-material ceiling. |
| `form_binding_candidates_per_operation_max` | Active | Exactly one candidate per first-proof plan. |
| `binding_artifacts_*` | Active | Admitted artifact envelopes and binding-declared retained payloads. |
| `plan_sets_per_thing_max` and `plan_sets_global_max` | Active | Frozen aggregate identity owner. |
| `plan_pins_*` | Deferred | Runtime handles pin only after publication; the unpublished Frozen proof has no operation pin. |
| `compiled_runtime_bytes_*`, engine-live, peak, and largest-allocation rows | Active | Reconciled sum/peaks for all retained runtime material. |
| `generated_effective_document_bytes_max` | ZeroContribution | No TD or effective-document clone survives sealing. |
| binding registration limits | Active at registration/snapshot scope | Frozen retains the existing immutable snapshot owner; it does not charge a new registration copy or per-entry pin. |

The current table has no separate byte/count rows for candidate records,
artifact-reference records, runtime binding-plan references, target-index
entries, or immutable diagnostics. Stage A includes all of them in the
`compiled_plan_bytes_max`/compiled-runtime ledger rather than treating them as
free. Independent review should decide whether that aggregate ceiling plus the
largest-allocation limit is sufficient or whether stable sublimits are needed.

Every retained string is charged exactly once by its final owner. Enumeration
may borrow source strings while measuring them, but final plan construction
must occur only after the shape reservation is held.

## Compiler accounts

`BindingCompilerBounds` already separates:

- final artifact footprint;
- cursor bytes;
- peak temporary bytes; and
- typed `WorkBudget`.

Stage A first calls `bounds` on each final plan/candidate pair without making
progress. It sums the declarations with checked arithmetic and reserves all
four classes before the first `start`. Measured artifacts are admitted against
their individual declaration and then against the transaction-wide persistent
ledger.

Cursor and temporary charges return to zero before Frozen. An unpublished
completed artifact remains charged while later coordinates compile. On
failure, live cursors are aborted before their reservations are released;
completed unpublished artifacts are dropped during the same private settlement.

## Work applicability

The current typed `WorkBudget` classes remain authoritative. Stage A maps work
as follows:

| Work family | Stage-A use |
|---|---|
| `JsonSchemaNodes` | Schema validation only; it must not be relabelled as generic typed-TD or aggregate work. Parse work is zero for direct typed input. |
| `UriBytes` and `SecurityBranches` | Target/default resolution and the explicit first-proof NoSec predicate. |
| binding compiler classes | Per-coordinate bounded `step` progress. |
| registration scan, property/form enumeration, material construction, index/seal, and join reconciliation | Required, but no current `WorkClass` accurately names these operations. This is an explicit predecessor gap. |
| cleanup/reclaim classes | Cursor abort, unpublished material release, and later plan-set reclamation. |
| call/response/decode classes | Deferred to runtime WP-400. |
| fallback/multi-candidate selection classes | Deferred until PLAN-INDEX policy is admitted. |

Stage A must not mischarge the missing operations to `BindingPolls`,
`CleanupItems`, or `JsonSchemaNodes`. A DECIDED migration needs one or more
accurately named classes—at minimum separating representation-neutral document
traversal from aggregate-plan build/reconcile work—before bounded production
progress is constructible.

## Reservation sequence

```text
validated source charge transferred
    -> reserve enumeration work + temporary ceiling
    -> enumerate and measure exact coordinates
    -> reserve plan-set slot + persistent shape ceiling
    -> materialize final plans/candidates/targets
    -> obtain compiler bounds from those exact values
    -> reserve artifacts + cursors + compiler temporaries + typed work
    -> start/step/admit artifacts
    -> reconcile measured persistent ownership
    -> release source + all temporary accounts
    -> commit persistent ledger into Frozen
```

Each arrow is observable in the composite fixture. A compiler callback before
compiler reservation, or final-plan allocation before shape reservation, is a
constructibility failure.

## Cleanup and reclamation

For the current pure caller-owned compiler cursor, generic cleanup-operation
record/byte rows make a **ZeroContribution** during pre-Frozen admission:
`abort` consumes in-memory state synchronously and creates no external cleanup
job. If a future compiler obtains an external resource, its SPI and resource
declaration must change explicitly; Stage A must not assume that behavior.

Plan-set reclamation remains **Active**. Frozen persistent charges stay live
through `Published` and `Draining`, then are released under the existing
bounded plan-reclaim work class before the generation can be reused.

## Deferred runtime families

The following remain outside Stage A even though they are necessary for the
later Consumer runtime:

- concurrent call slots and queueing;
- request/response buffers and codec state;
- deadlines, cancellation after publication, and retry/fallback;
- cache state and subscriptions; and
- multi-candidate fairness or availability accounting.

They must not be charged to aggregate construction merely to make the resource
table appear complete.

## Failure requirements

Every checked sum, multiplication, and representation conversion can fail
before the corresponding allocation or compiler progress. Such failure records
the first cause and enters private Building settlement. After settlement:

- retained source is zero;
- temporary validation/build memory is zero;
- unpublished persistent runtime memory is zero;
- compiler cursor and temporary memory is zero;
- the snapshot lease is released;
- no persistent ledger is committed; and
- the unpublished plan-set generation is invalidated.

This is the resource meaning of authoritative `Building -> Failed`; it does
not add a new public lifecycle state.

## Decision-time schema impact

A later DECIDED review should compare this projection row-by-row with
`docs/resource-limits.csv`. The likely authoritative change is a typed,
representation-neutral TD census/reservation family and any missing exact work
classes revealed by that comparison. No schema change is made while this topic
is `DISCUSSING`.
