# 0075 Consumer Borrowed Admission Construction

Status: MIGRATED — design input and discriminator independently accepted in
PR #128 at `4b2fd2f4a77d83ab78c360202f7cd000d8fd7e17`.

The selected conclusion is projected by [ADR-0021](../docs/ADRs/0021-borrowed-consumer-td-admission.org)
and the registered TD/Planning/Servient contracts.
[The TD admission record](../docs/work-packages/WP-100-consumer-validated-thing-admission.md)
owns implementation admission and its evidence authority. The original
investigation and prototype claims below are retained as history; they are not
current production authority or completion evidence.

This investigation starts from fetched `master`
`9854dd718757929de2711d3a3ad97d95fc141a5d`. It changes only this workspace topic,
its index, a non-production architecture fixture, and fixture/CI support. The
Snapshot specifications, public production APIs, work-package contracts,
candidate/admission states, roadmap, and gates are unchanged.

## Decision

Select **a TD-owned validated immutable Thing loan, resumable semantic lending,
and a Servient-owned transaction producing entirely owned runtime material**.
There is no complete normalized TD copy, source-storage census, or retained
source in Published. TD owns bounded validation and effective meaning; Planning
owns selection, complete output, and compiler barriers; Servient owns allowance
pairing, publication, and runtime lifetime. Bounded external JSON ingestion is
a separate future capability.

The discriminator is constructible for its explicit input/compiler envelope.
It joins the complete shared Basic program to paid whole-Thing inspection,
moving borrowed continuations, charged security/URI queries, real Core plans,
real mock binding artifacts, source destruction, and an allocation-free private
publication model. This is sufficient to propose a coherent replacement
authority revision for review. It is not production admission, complete
Consumer acceptance, a constrained runtime claim, or proof of generic compiler
allocation behavior.

Two details become concrete rather than remaining choices in 0074:

1. **Lend and acknowledge one coordinate; store no complete derived TD table.**
   TD keeps current effective-operation/security state and, when needed, current
   resolved URI storage. An event stays ready until Planning has copy credit.
   Planning stores scalar continuation coordinates and takes a fresh loan on
   each step. An event cannot borrow cursor-owned URI bytes across moving that
   cursor. This also prevents replaying URI/security work on insufficient copy
   credit. The fixture constructs this mechanism; a full table is unnecessary.
2. **URI queries need an explicit work algorithm.** The existing allocation-free
   TD URI kernel includes reverse output-path scans. It cannot be charged as
   one query or asserted linear. The discriminator admits a small atomic
   envelope and fully prepays a conservative quadratic debit. Production may
   use that form only where the checked profile can supply its maximum atomic
   debit. Otherwise TD must supply a byte-resumable resolver with the same
   semantics, preserving parse/path/output coordinates across Pending. This
   is an implementation support boundary, not permission to reject an input
   that satisfies a production profile's promised URI envelope.

These findings refine 0074; none forces a full Snapshot or falsifies its main
ownership direction. Capacity-charged full ownership remains the fallback if
a supported backend cannot provide bounded borrowed inspection, or an actual
deployment requires input ownership/strict ingestion immediately.

## Reconstructed authority and implementation

The independent [#127 review](https://github.com/yushun1990/clinkz-wot/pull/127#issuecomment-6008629803)
accepts investigation of this direction, explicitly **not** a constructed
replacement, migration, readmission, or gate. Its concerns were rechecked
against source, rather than inherited as conclusions:

| Obligation / question | Repository owner and actual boundary |
| --- | --- |
| Finite policy, accounted owners, bounded progress | [Foundation](../docs/spec/foundation.md), [runtime safety](../docs/spec/runtime-safety.md), [resource schema](../docs/resource-limits.csv), [ledger](../foundation/src/resource.rs), [work budgets](../foundation/src/budget.rs). Actual Foundation primitives exist; their composition into this transaction does not. |
| Entire Basic before trusted Planning | [Thing validation](../td/src/thing.rs), [WP-100 admission contract](../docs/work-packages/WP-100-consumer-validated-thing-admission.md), [shared Basic kernel](../tools/architecture-fixtures/validated-thing-schema-kernel/src/basic_kernel.rs). Complete validation includes unrelated Actions/Events/security/schemas, not only readable Forms. Basic needs no ID; Consumer preflight does. |
| Resumable typed access | [Typed Basic accessor](../tools/architecture-fixtures/validated-thing-schema-kernel/src/basic_typed.rs) and [schema accessor](../tools/architecture-fixtures/validated-thing-schema-kernel/src/typed_access.rs) contain synchronous scans, `.nth`, and native lookups. They are semantic oracles, not this paid driver. [Typed field enumeration](../tools/architecture-fixtures/validated-thing-schema-kernel/src/thing_build.rs) provides useful borrowed slots/iterators but its JSON canonicalization is unnecessary here. |
| Context and opaque input | [Context](../td/src/components/context.rs) has private entries; its serializer imposes a stronger context condition than Basic. [0064](0064-serde-json-retained-representation-impact.md) / [0065](0065-liballoc-btreemap-retained-representation-impact.md) invalidate physical accounting from caller collection lengths. Borrowing removes engine ownership of that storage, not the need to inspect its content. |
| Effective TD semantics | [Defaults](../td/src/td_defaults.rs), [URI semantics](../td/src/core/data_type/uri.rs), [shared URI witness](../td/tests/support/uri_semantic_kernel_probe.rs), [semantic-view witness](../td/tests/support/semantic_kernel_probe.rs). Defaults, inheritance, definition interpretation and RFC3986 meaning belong to TD. Planning owns URI-template compilation. |
| Complete aggregate, zero early starts | [Planning specification, first aggregate](../docs/spec/planning.md#first-consumer-property-read-aggregate), [WP-200](../docs/work-packages/WP-200-planning.md). Every property has a lookup row; original Form indices and property key order survive; all bounds precede every start. |
| Current public boundary | [PlanBuildInput](../planning/src/lib.rs) accepts raw `&Thing`; its name proves no validation. [Property Read leaf](../planning/src/property_read.rs) builds one selected coordinate. Neither supplies the new trusted aggregate or an exact aggregate PlanFootprint. |
| Owned plans/artifacts | [Core plan](../core/src/plan.rs), [compiler SPI](../core/src/binding_compiler.rs), [external owned handoff](../tools/architecture-fixtures/validated-thing-planning-handoff/src/planning.rs), [0072](0072-consumer-validated-thing-build-lifetime.md). A lifetime-free generic wrapper alone does not exclude a borrowed payload/callback. Published source retention was already removed from the target. |
| Publication/lifecycle | [Primary flows](../docs/architecture/10-primary-data-flows.md), [module boundaries](../docs/architecture/20-module-boundaries.md), [Servient lifecycle](../docs/architecture/50-servient-runtime-lifecycle.md), [states](../docs/state-machines.toml), [WP-400](../docs/work-packages/WP-400-servient.md). Actual [legacy consume](../servient/src/servient.rs) / [ConsumedThing](../core/src/thing.rs) retain an Arc<Thing>; they cannot be the new bounded facade. |
| Scope / unaffected evidence | [Authority selector](../docs/spec/v5-authority-reset.toml), [tranche DAG](../docs/work-packages/index.toml), [PLAN](../PLAN.md), ADRs [0008](../docs/ADRs/0008-compiled-plan-lifecycle.org), [0015](../docs/ADRs/0015-borrowed-resource-profiles-and-linear-work-budgets.org), [0019](../docs/ADRs/0019-consumer-one-shot-authority-entry.org), [0020](../docs/ADRs/0020-strict-json-value-decoding.org). Strict input is promised by today's refinements and must be explicitly deferred in migration. It is not silently satisfied by ordinary parsing. |

The active root obligations are still `DOC-RUNTIME-001`, `ADMIT-MEM-001`,
`ADMIT-TXN-001`, `RES-LIMIT-001..003`, `CONSTRAINED-WORK-001`,
`CONSTRAINED-PROGRESS-001`, `CONSTRAINED-OWN-001`, `API-RESOURCE-001`, and
Planning's cost/set/artifact/request guarantees. Their present Snapshot-specific
refinements remain binding until replaced. Historical TD memory identities
being inactive does not repeal those refinements.

Production TD has no validated admission feature, owner, or paid driver. The
shared source candidate extracts rules and private inspection only under
`tools/architecture-fixtures`; its Rust types are a separate source universe.
This prototype does not route through public synchronous validation or raw
production PlanBuildInput.

## Ownership and lifetime flow

```text
caller owns Thing                     Servient owns startup policy/registration
  immutable external loan               shared stable build loans + copied identities
              \                        /
               ConsumerTransaction<'td, 'environment>
                owns local/parent allowance tokens, phase and first cause
                  |
                  +-- TD structural/Basic continuation
                  |     external borrowed iterators + charged nonrecursive frames
                  |     one monotonic TD lifetime debit; fixed diagnostics
                  v
                TD-private ValidatedThing<'td> proof
                  |
                  +-- TD semantic continuation / current derived bytes
                  |     fresh step loan -> Planning -> acknowledge
                  v
                complete Planning preflight (owned counts/envelope, no TD lifetime)
                  |
                Servient reserves runtime/record/slot envelope
                  |
                owned logical inputs + candidates + lookup + all compiler bounds
                  |
                sequential pure compilation; owned eager artifacts
                  v
                complete Frozen draft, no TD/config/registration loans
                  |
                release build scratch; reconcile actual owners; final checks
                  v
                private permit -> non-fallible install -> Published generation
                  |
                immutable runtime + separate complete registration owner
                leases -> Draining -> bounded reclaim -> Reclaimed
```

Names in this flow describe private roles, not frozen new public signatures.
The TD entry consumes a checked transaction projection, not raw limits. An
opaque proof contains the external input loan, checked counts/support identity,
and remaining TD lifetime allowance. Only TD can construct it, after both
whole-input resource inspection and complete Basic. No open downstream trait,
unsafe lifetime erasure, serializer success, or caller assertion mints it.
Planning sees only semantic values/opaque sequence views and original
coordinates. It has no Thing, raw map, Context entries, arena offset, or storage
pointer accessor.

The transaction is a movable value. A reference in its suspended TD state may
point to the externally borrowed immutable Thing or external immutable startup
configuration/registration. It may not point into transaction-owned scratch.
Owned scratch is accessed by private scalar positions and short loans. A ready
derived event remains owned by TD across Pending and is re-lent without replay;
Planning acknowledges only after its own debit/copy/check succeeds. Cancellation
can discard it without inspecting or destroying the input graph.

Preflight is tied to the same transaction's immutable proof, policy,
registration identity and generation. Its owned counts are not a reusable
authorization token for some other TD. Keep that association in the private
transaction owner/nonce; no public pointer identity is needed. A new input,
registration generation or policy capture starts a new transaction.

The output boundary must close actual concrete artifact and callback lifetimes.
Require owned artifacts at the aggregate admission boundary (including a
`'static` constraint where applicable), inspect each supported binding's actual
payload/registration behavior, and exercise post-source destruction. A generic
type without a lifetime parameter is insufficient. `'static` does not prove
absence of unsafe/raw-pointer dependencies or honest resource declarations;
those remain binding-authoring and external evidence obligations.

The compiler continuation has the same movement rule: it cannot keep a loan or
pointer into transaction-owned logical inputs across Pending. Each callback
receives a fresh short input loan; suspended compiler state is owned or uses
checked scalar coordinates into its declared storage. The closed supported
compiler set must supply that proof along with its allocation/abort contract.

The bounded public progress shape should consume its owner: return that same
owner on Pending, an owned source-free result on Complete, or the first fixed
cause with any remaining cleanup owner on failure. A terminal generic cursor
left alive beside the result can keep a Rust input lifetime in scope even when
its input field is None. The external fixture's completion helper destroys that
terminal build object before returning Draft; migration must make this boundary
structural in the production progress API, not require callers to discover a
special drop order. Per-step semantic loans remain short borrows internally.

All data used after publication is owned: IDs, property lookup names/ranges,
original indices, resolved/template execution facts, content/codec/scopes where
needed by that binding, candidate and artifact identities, artifacts, response
validation material, footprint, and lifecycle/cleanup records. Copy only facts
required by the admitted operation. Opaque metadata and unrelated TD schemas
are validated/resource-inspected but not retained merely for convenience.
The first slice keeps the completed narrow response-sealing contract; general
response-branch/schema planning remains deferred by WP-200. When an admitted
operation needs a template or validator program, Planning compiles only its
required TD-lent facts into accounted owned execution material during
materialization. It cannot retain a schema loan for call-time validation. That
operation-specific program is not a copied full TD, and this fixture does not
prove that deferred program compiler.

The caller must keep Thing immutable/live until the build's input loans end.
The borrow ending does not deallocate caller storage. A synchronous convenience
wrapper can drive this borrowed operation within its own caller-owned scope;
an owning public adapter cannot advertise bounded terminal cleanup unless its
source owner has a separately proved bounded destructor. Do not carry legacy
Arc<Thing> retention into the new runtime to preserve old consume ergonomics.

## TD semantics and progress

Use one TD rule/discovery source for public Basic and all trusted storage
backends. The synchronous public adapter may retain its existing formatted
errors; the bounded adapter emits fixed inline rule/owner/field/index/ordinal
diagnostics. First semantic rejection follows the shared program, including
schema OneOf-before-flags and child order. Diagnostic formatting is a separate
explicitly budgeted convenience, not part of bounded admission.

Structural inspection precedes semantic validation. It visits every supplied
known field, private Context entry, schema child, string, map association,
sequence member, opaque Value and Number. Applicable counts/depth/text/Number/
total-content limits are checked before entering variable work. Resource
rejection and Basic Invalid are distinct; a structurally inadmissible input
does not have a promised Basic error ordering. No new Profile/Full or combo-cycle
predicate is added. Complete Basic then covers security references/definitions,
root schemas/URI variables, all Properties/Actions/Events and their forms,
including unrelated and non-readable coordinates.

Borrowed traversal uses one bounded nonrecursive frame workspace. Retain actual
iterators or slice positions rather than reconstructing a map walk for an
ordinal. All frames own only scalars/external loans. Moving/growing the frame
block pays for each element, retains old/new charges during transfer, and
pre-admits the actual checked replacement Layout. Opaque caller map allocation
history is baseline, not a number inferred from len(). Object storage order is
not TD meaning: inspect serde objects in backend iteration order; deterministic
Property order comes from Thing's BTreeMap key order, and Forms use slice indices.
No canonical sort of every opaque object is necessary.

Native iterator seek/advance is still work. The fixture deliberately charges
`length + 1` for these opaque-container primitives, rather than pretending each
`next` costs one instruction. That is a conservative logical envelope for the
tested standard BTree/dense serde-map backends. It can yield quadratic *charged*
work for a linear enumeration. The fixture's byte comparisons are individually
resumable and recorded. Production must audit the supported dependency backends
and bind a defensible primitive envelope; no hash lookup/private node-layout
assumption or target-cycle guarantee follows from these logical units. An
unsupported backend must fail configuration or receive a supported TD adapter,
not be traversed through an unbounded fallback. A compact borrowed index is an
available optimization only with its own charged construction/comparison/
capacity/destruction evidence. It is not necessary for this discriminator.

Repeated semantic work pays again. Preflight and materialization may enumerate
the same immutable input twice; each pass has monotonic positions and consumes
the **same remaining TD lifetime allowance**. That declared second pass is not
a replay at each Pending. Explicit operation lists are inspected item by item;
missing operations use the shared fixed defaults. Explicit empty operations
stay empty. TD supplies effective security and definition meaning; explicit
empty security overrides inheritance. Planning alone applies the first slice's
exactly-one-NoSec eligibility.

For a profile requiring resumable URI work, TD's continuation phases are
component discovery, merge-point search, prefix emission, segment inspection,
output-path pop, query/fragment emission, and ready. Retain external source
spans, byte/segment positions, path-start/pop offsets and current output length;
never a reference into the output buffer. Use the typed URI's already parsed
component spans where the supported backend provides them. Any remaining
parse/scan, including reverse slash search, retains its byte position and pays
before advancing. Append/insert/shift/grow work and old/new capacity overlap are
charged explicitly; only the completed UTF-8 output is lent. This preserves the
shared resolver's semantics without asserting linear cost or replaying a path
after Pending. The fixture proves the small prepaid atomic alternative, not
this resumable algorithm; production eligibility requires its separate witness
when the atomic alternative cannot fit the profile.

The trusted query interface is a paid continuation/lending interface, not an
unrestricted collection of synchronous allocation-free helpers. Cheap completed
scalar facts can be re-borrowed without hidden progress. Source-dependent lookup,
operation filtering, URI derivation, and copying must either retain a completed
fact or consume explicit work. A failed copy debit leaves the same ready event;
it cannot advance, accumulate credit across calls, or recompute that event.

Number inspection keeps the existing AP lexical-access boundary for bounded
typed admission. Check every borrowed Number lexeme against finite supported L,
including opaque Numbers. Within-limit opaque `1e309` remains a legal supplied
value. Only the five existing extension arithmetic predicates use the shared
finite-binary64 projection amendment; non-Numbers remain absent and typed
f64/i64 schema fields retain their existing behavior. Debit the full admitted
atomic input/work envelope before projection and check cancellation around it.
The fixture coalesces at most five already captured projections into one fully
prepaid atomic group; the production per-Number continuation can reuse the
existing [projection primitive](../tools/architecture-fixtures/validated-thing-feature-boundary/td-boundary/src/projection_step.rs).
No new decimal arithmetic or strict Number decoder is needed here.

Every progress owner has per-call class budgets plus non-resettable lifetime
remainders. Multi-class/atomic debits are checked together before decrement or
work. Zero budget makes no hidden traversal, query, allocation, callback or
publication progress; cancellation may still choose rollback. Insufficient
atomic credit returns Pending with the same state and no carried partial credit.
The checked profile must be able to provide the largest supported atomic
operation eventually; a caller repeatedly supplying less cannot demand progress.
An exhausted lifetime allowance is terminal Limit, not a reason to refill it.

## Typed byte/resource meaning

Choose a precise **logical typed-content oracle**, not wire length or physical
source bytes. For the replacement typed entry, `document_bytes_max` should cap
the sum of UTF-8 bytes in supplied string/map-key/URI/Number leaves plus eight
bytes for each fixed-width scalar leaf of the typed field traversal. Known
field labels, absent slots and structural tags are not byte leaves; node/member/
depth maxima bound their work separately. Context/metadata/opaque contents count
just as other supplied contents do. AP Number lexical text counts once as a
leaf. Dates are their eight fixed components in the shared typed field program;
there is no RFC3339 serialization/parse. String lengths and counts do not claim
anything about caller capacities, allocator metadata, stack or process memory.

This is the fixture's explicit oracle and the selected migration input. The
migration must register the typed-field traversal and its exact scalar widths
as the owner of this oracle and update every affected interpretation of
`document_bytes_max`. Future ingestion uses that field's ingestion projection
for actual consumed wire bytes; that operation distinction must be explicit.
If retaining the old field name with two named oracles is unacceptable to the
authority review, choose a dedicated logical-content control in that review,
not an implicit serialization estimate. No resource row is changed here.

## Planning and Servient phase contracts

| Private phase | Required result / owner | Failure and progress boundary |
| --- | --- | --- |
| Entry/capture | Servient validates applicable policy/support; captures complete registration identities and reserves local/parent build capacity and cleanup owner. | Configuration errors precede the progress machine. No unchecked raw limits; unrelated NA cells remain ignored by the operation projection. No variable allocation or source scan before allowance acquisition. |
| TD inspect/Basic | WP-100 produces the private proof, checked counts, fixed diagnostic and remaining lifetime debit. | Failure/cancellation/drop releases controlled frame/scratch blocks with prepaid release; caller Thing stays caller-owned. No Planning/compiler work before the complete barrier. |
| Complete preflight | WP-200 enumerates every property/readable original Form; checks ID, one-registration capability, NoSec and supported execution inputs; computes checked counts, logical/output capacities, remaining Planning work and compiler envelope. | Missing ID fails first, before runtime reservation/materialization/bounds/start. No plan IDs, artifact, lease or partial published lookup. Preflight is owned and bound to this transaction. |
| Reserve runtime | WP-400 reserves generation/registry slot, persistent output, runtime record, complete-registration retention where attributable, and rollback/reclaim capacity from the preflight envelope. | Logical envelope is not a contiguous allocation request. Reservations stay uncommitted until final reconciliation/publication. Release every acquired child before its parent allowance on failure. |
| Materialize | WP-200 copies complete owned logical inputs/candidates and every lookup row, including empty ranges. TD derives URI semantics through lending. | Later-coordinate error/Limit/cancellation fails the whole aggregate with zero starts. No retained event borrow into owned scratch. All output/growth requests and copies are prepaid. |
| All bounds | WP-200 calls each pure compiler bound once against owned inputs and captures final/cursor/temporary/typed-work maxima. | No start until the entire barrier passes. First Consumer permits only nonzero BindingPolls bounded by the existing per-step compile field; checked coordinate count × that field bounds aggregate work. Bad/later bounds yield zero starts and release the whole draft. |
| Compile/reconcile/seal | WP-200 drives one pure compiler cursor at a time with one coordinate remainder, aborts it exactly once if needed, admits actual artifact/output footprint and freezes one complete owned draft. | No protocol I/O, credential callback, handler/task or external operation lease. Actual final/cursor/temporary excess fails unpublished; successful earlier coordinates are rolled back too. Source/config/registration/derived loans all end at this boundary. |
| Build cleanup / final checks | WP-400 waits for stepped scratch/rollback work to finish, reconciles all actual local/parent charges and identity/generation/registration/cancellation/slot facts. | Still unpublished/cancellable. Source-independent use can occur here. No semantic recount, second lookup or artifact-slot rewrite by Servient. Preserve first cause through any required cleanup. |
| Permit / install | WP-400 issues one private move-only permit under the registry's exclusive installation authority and installs the preallocated complete record. | No allocation, callback, yield, source query, new cancellation decision or fallible check between permit and install. Cancellation linearizes at the last check; after permit the successful install wins. Readers see absent or complete. |

These phases are internal ownership succession, not new public lifecycle states.
Keep Building -> Frozen -> Published -> Draining -> Reclaimed, with unpublished
Failed and no selectable partial result. Build scratch must be terminal before
permit; do not hide an arbitrarily large destructor in its final interval.

Purity alone is not a work or storage bound. Every supported compiler's bounds,
start, step and abort callback needs an admitted fixed/input-envelope primitive
cost and workspace contract before invoking it. The returned coordinate work
bound does not retrospectively pay for computing that bound. Charge callback
entry before invocation, acquire declared cursor/temporary capacity before
start, and preserve the one coordinate remainder across Pending. The inspected
mock has a fixed scalar bound/start and one paid completing poll; that narrow
observation cannot authorize an arbitrary implementation of the public SPI.

Servient's registry slot/generation and allowance tokens prevent duplicate
publication/reuse. Immutable startup registration storage remains outside the
transaction, and Published retains the matched complete registration separately
from the owned draft. Static and Host erasure have their existing distinct
owners/costs; copied identity checks join them. Final identity/generation/resource
checks and the permit must be synchronized with slot mutation and cancellation,
so a concurrent invalidation cannot enter the install interval.

Published selection uses the owned property lookup and only that property's
contiguous readable range, then resolves plan/artifact/registration by captured
generation under a plan-set lease. Absent property, empty range, and original
strict index mismatch remain distinct errors. Drain closes new leases at its
linearization point; existing leases keep immutable material and the separate
registration alive until operations/cleanup settle. Reclaim is byte/item-budgeted
and releases children before the record's persistent parent allowance. Neither
runtime nor reclaimer can consult TD.

## Resource ownership and failure

At each actual ownership boundary, account this disjoint live set:

```text
core live = TD frames + current derived scratch
          + Planning continuation/temporaries + owned logical/lookup/candidates
          + live compiler cursor/temporary + owned eager artifacts
          + attributable registration/erasure/record/slot storage
          + fixed diagnostics + cleanup/reclaim capacity
system live = core live + still-live input owner + other engine owners
peak = maximum observed simultaneous live sum, not sum of phase maxima
largest request = maximum actual checked Layout, not aggregate allowance
```

Inline cursor slots, fixed URI buffers, unused output capacity, alignment and
selected allocator surcharge remain costs. Caller allocation history is the
typed baseline. If an engine-owned upstream ingestion/Discovery owner lends
the source, its original global source charge stays live; labeling it caller
input cannot erase it or double-charge it. End a loan without releasing a still
live physical owner.

Servient reserves aggregate envelopes with logical allowance objects. Each
physical allocation then obtains matching local and parent authorization for
its proposed Layout before calling the allocator. Commit actual capacities,
including spare capacity and old/new grow overlap. On allocator failure undo
the uncommitted request, preserve the first cause, and retain any old block.
Copies/moves and future release costs are paid before each acquisition. A
post-reserve Vec capacity observation cannot authorize an excess allocation.
Source counts cannot bound compiler-owned storage.

Planning's exact output ledger separates logical strings/rows, lookup/candidate
capacity, eager artifacts, cursor/temporary overlap, erasure/registration costs,
and reclaim metadata. First Consumer has no lazy slots or retained TD bytes.
An artifact's conservative declaration and actual footprint require independent
binding evidence; the aggregate enforces them and per-Thing/global bounds. The
current Foundation ledger observes the largest reservation it is given: do not
feed an aggregate logical envelope into that primitive and then report it as a
contiguous allocation. Keep envelope accounting separate from actual Layout
requests in the replacement composition.

Terminal failure owns cleanup until it is terminal. TD release is a fixed
catalog of trivially destructible frame/byte blocks with prepaid deallocation.
Planning variable output needs a nonrecursive bounded draft/reclaim owner, not
an unbounded Vec<Plan> Drop. An abandoned cursor must either perform only
proved fixed prepaid release or hand off pre-reserved cleanup to Servient; it
must not allocate a cleanup record during abandonment. A compiler cursor has
one exactly-once abort owner; an extension with unbounded abort/destruction is
ineligible for the bounded path until it supplies a valid cleanup contract.
Rejection/cancellation never drops the caller Thing. Cleanup failures cannot
replace the original admission cause or make a subset published.

## WP responsibilities and interfaces to migrate later

| Owner | Owns in the selected design | Does not acquire |
| --- | --- | --- |
| WP-100 / TD (+ Foundation primitives) | Applicable TD support checks; logical-content/count/Number oracle; complete paid typed inspection and shared Basic; private proof; effective operations/security/URI lending; TD work/lifetime/scratch/diagnostic/release. | Plan selection, compiler coordination, registry slots, publication, input history census or a complete copied TD. |
| WP-200 / Planning | Trusted proof-only aggregate entry; ID/capability/NoSec preflight; checked complete counts/envelopes; owned logical inputs, candidates, every lookup row; all-coordinate bounds barrier; sequential compilation, actual output footprint, source-independent Frozen draft, draft rollback/reclaim mechanics. | TD default/security/URI rules, raw Thing access, runtime selection publication, protocol I/O, parent/global registry admission. |
| WP-400 / Servient | Transaction capture and driving; parent/local pairing and runtime/record/slot reservation; cancellation/identity/resource reconciliation; cleanup ownership; permit/install; complete registration retention; runtime leases/drain/reclaim and facade migration. | Another Basic validator/planner, semantic recount, a second lookup, Published TD access or protocol-owned correlation. |
| WP-300 / binding | Pure declared compiler behavior, owned concrete artifacts, honest final/cursor/temporary/work/abort bounds; existing binding execution/lifecycle. | Owning the aggregate transaction or retaining TD/registration callbacks in artifacts. |

Retain one checked Consumer policy boundary with private operation-specific
TD/Planning projections. The existing independently exposed Snapshot admission
config/footprint is not the new transaction interface. TD, not Servient,
validates its implementation-supported Number/URI/traversal envelope. Policy
identity and immutable capture survive every phase; missing/unsupported finite
configuration is separate from an admissible-policy input Limit.

## What the prototype establishes

The [fixture](../tools/architecture-fixtures/consumer-borrowed-admission/README.md)
compiles TD's existing source candidate with a private borrowed module. It reuses
the shared Basic/schema rule/discovery kernels, typed field enumeration, checked
arena transfer/release machinery, and differential URI resolver. An external
crate accepts only the private proof; it creates actual Core logical plans and
the existing MockCompiler's concrete artifact envelopes. Production files are
not patched.

| Attack | Executable result / limit |
| --- | --- |
| Whole Basic / first error | The shared complete Basic corpus compares acceptance and the exact inline first rejection against the existing synchronous kernel. Production/source-candidate differential tests remain the separate semantic authority evidence. |
| Hidden replay / lookups | Large vs interrupted budgets/moves give identical complete traces. Long common-prefix security names account actual compared bytes; native lookup and `.nth` are absent from the new driver. Full supplied opaque content is traversed. Conservative native-primitive debits remain explicit. |
| Resource truth / grow / drop | Thread-local allocator instrumentation matches frame allocation count, actual Layout bytes, grow overlap peak, largest request and release. Failure at every observed frame request, rejected layout before allocator entry, cancellation/abandonment during an in-flight grow all release exactly once. |
| Number islands | Within-limit opaque 1e309 survives; lexical one-over limits reject before projection. A 200-byte numeric predicate suspends on 199 units with no projection, debit or credit accumulation, then projects on a complete debit. Shared numeric semantics are reused. |
| Borrowed semantic / derived storage | Original non-first Form indices, explicit-empty ops/security, inherited NoSec and relative URI dot-segment/query/fragment resolution join Planning. UTF-8 validity and total scope bytes are established before readiness inside paid TD projection; scope sizing requires a complete per-item debit. Counters at the UTF-8 check and source iterator verify that ready derived events move and suspend under repeated zero/short copy or cleanup credit without URI revalidation or TD/Planning scope rescans. |
| Source independence | Concrete Core plan and mock artifact are selected after Thing and complete registration destruction. The registration supplies the actual compiler and captured candidate identity. Original href/content coding/scopes survive in owned output. Artifact targets equal owned resolved plan targets. |
| Compiler barrier | Later URI/eligibility failure, allocator failure at every checked materialization request, and injected second bounds failure yield zero starts. Materialization failure also yields zero bounds. Every successful bound precedes all starts. No pure compiler progress is confused with transport execution. |
| Publication | A private one-slot model installs the owned complete draft with no allocator activity after final checks. Identity rejection leaves no published record and releases the draft. This is not concurrent Servient evidence. |
| Capability/lifetime negatives | Compile-fail examples prohibit raw-source access, unchecked proof construction, raw Thing Planning input, retained event self-borrow across a cursor move, and input destruction while its continuation remains live. |

The prototype intentionally uses a fixed sixteen-property/coordinate/scope
output envelope and an inline current URI buffer with at most 256 input bytes.
Its output drop is bounded by that fixed catalog; it does not establish the
production variable-output reclaimer. The existing mock allocates exactly one
copied target per artifact and completes in one declared BindingPolls debit;
the fixture inspects/pre-admits that request and measures it independently. It
does not infer safe allocation from a generic compiler's footprint scalar.

The 64-bit Host witness has 253 output heap bytes, 253 output heap peak, largest
request 36, 17 actual output allocations, and 10,200 inline Draft bytes. Those
are different measurements, not a claimed 253-byte transaction or portable
profile ceiling. A deeper borrowed opaque traversal observes four frame
allocations, 15,744 peak requested bytes, 10,496 largest request and 29 paid frame
transfer actions on this local toolchain. The source graph remains caller
baseline. No comparison against an unrelated Snapshot fixture's maxima is
reported as a measured saving.

The same Host run reports `size_of` of 1,200 bytes for Validation, 1,120 for Read,
13,720 for Build and 10,200 for the result owner. Those contain their nested
fields, so summing Build and Draft would
double-count its inline draft; moving a large result may add stack overlap.
The fixture's TD memory limit controls frame requests, and its output limit
controls Draft plus output heap. It does not present those separate local
ledgers as full transaction accounting. All fixed continuations, result/stack
overlap and caller-supplied WorkBudget storage belong in the production catalog
and supported target evidence, with allowances acquired at entry. There is no
variable source cache or second owned graph hidden outside the observed frame
and output requests.

## What remains unproved

There is no constructed general production Consumer transaction here. In
particular the prototype does not prove:

- the complete production profile projection/applicability matrix or all URI/
  template/Number supported envelopes; its paid URI island is fixture-specific;
- atomic local/global parent pairing, real registry slot/generation races,
  registration retention/Host erasure, or reconciliation of every simultaneous
  production owner under one peak;
- generic compiler allocation/cursor/temporary honesty, resumable Pending/failure
  after start, exactly-once abort under every extension path, or all allocator
  failures in the existing infallible mock artifact allocation;
- the production variable draft rollback/reclaimer, lease/drain concurrency,
  compiler cancellation races or application-facade API ergonomics;
- runtime Host/static semantic trace parity, no-atomic targets, constrained
  stack/cycles/RAM, AP parsing cycle ceilings or real protocol execution;
- any bounded strict JSON ingestion, Directory/Discovery ingestion commitment,
  source physical capacity bound, whole-process memory bound, authority
  migration/readmission/completion/gate or release claim.

Existing complete-registration handoff, compiler/response/selection, numeric
feature/target and controlled-allocation probes remain reusable at their own
boundaries. They are not silently broadened to discharge these missing joins.

## Snapshot-era disposition

| Mechanism | Disposition / independent reason |
| --- | --- |
| Complete normalized typed node/edge/byte TD graph | Remove from typed Consumer. Unused metadata does not need copying to preserve validation or build owned plans. Flat owned storage may remain appropriate for future ingestion. |
| Mandatory three retained / four temporary sites | Remove prescribed source catalog counts. Every actual frame/derived/output/cleanup site still needs exhaustive accounting and release. The fixture reuses the old frame allocator without allocating source node/edge/byte arenas. |
| Exact-length TD seal | Remove. Controlled actual capacity, immutability and charged overlap replace the copied TD seal; output still freezes semantically. |
| Whole-TD post-copy fieldwise equivalence | Remove when the proof refers to the same immutable input. Keep Basic/query parity, derived URI agreement and owned-output completeness. An alternate ingestion backend still needs shared-meaning evidence. |
| Source retained footprint / source-to-persistent-document transfer | Remove from this path. Build-only TD-free runtime from 0072 remains required. Source/document accounts remain for actual owned-source capabilities. |
| Snapshot config and public footprint handoff | Replace with checked transaction projections and actual owner/output accounting. Applicable policy/support, live/peak/largest/request/release truth remain. |
| Canonical sorting / opaque map reconstruction | Remove from borrowed structural inspection. Keep property key ordering and original Form indices; future ingested representation may need its own indices. |
| Owning normalized-source continuation / reseal scaffolding | Not a typed prerequisite. Retain nonrecursive frames, movement proof and short loans into any owned scratch; owned ingestion must solve its own continuation lifetime. |
| Strict literal JSON / field-policy decoder / RFC3339 lexer | Future ingestion capability. Preserve ADR-0020 meaning and the shared decode/rule evidence; do not implement its Thing decoder here. |
| AP access and numeric amendment | Keep independently for bounded Number inspection and the existing five arithmetic predicates. AP is lexical access, not arbitrary-precision arithmetic. |
| All Basic, structural inspection, all bounds, PlanFootprint, immutable output | Keep. These guarantees do not depend on owning a Snapshot. |
| Cleanup capacity, child-before-parent release, final permit/install, leases/drain | Keep. Runtime/draft owners still have real costs and lifetimes. |

Do not delete historical fixtures or label their controlled-storage proofs
invalid. Their necessary status as typed Consumer prerequisites changes; their
own narrower observations do not.

## Future bounded ingestion extension

TD should own a closed internal source-access boundary. A future admitted
decoder may create controlled flat storage and lend it to the same Basic and
semantic program without constructing an ordinary Rust Thing or re-copying a
normalized Snapshot. The backend cannot be caller-implemented as a trusted
proof factory. Storage facts/lexical decoding may differ; TD field policy,
Basic/default/security/URI meaning stay singular. Planning's lending interface
and owned output do not change.

That capability accounts bytes and all source construction from its first
controlled allocation, retains source/global charges while borrowed, and
supplies bounded parse/build/cancel/drop/release with literal JSON semantics.
Source release and reconciliation happen before its publication permit, under
its own ownership proof. Ordinary serde/authoring remain unbounded caller
provisioning; wrapping their result in this loan advertises no absolute ingestion
bound. Directory/Discovery or later v1 claims must reopen their exact ingestion
requirements before claiming bounded external runtime input.

## Exact inputs for subsequent authority migration

The next authority revision can use this selected flow, lending/acknowledgement
mechanism, byte oracle, phase ownership, explicit atomic support rule, and
evidence limits. It must resolve the following concrete inputs in the reviewed
revision rather than inventing them during production implementation:

1. A cross-domain ADR superseding the typed normalized-owner choice, explicitly
   deferring strict ingestion for the first Consumer slice and retaining
   ADR-0020 for its future capability. Record the longer caller input loan and
   removal/replacement of legacy owning consume semantics.
2. Exact applicable resource projections and supported numeric/URI/container
   envelopes for each admitted profile. Bind maximum atomic debit to available
   per-step credit; select bounded atomic vs resumable URI support concretely.
   Register the logical typed-content oracle and all affected meanings of
   `document_bytes_max`, counts/depth and extension/Number limits. Audit pinned
   BTree/serde feature backends; add a supported adapter if their primitive
   envelopes differ.
3. Concrete allocation catalogs/formulas for TD frames/current URI scratch,
   Planning output/cursor/temporary, static/Host registration/erasure/record/slot,
   diagnostics and rollback/reclaim. Separate logical aggregate reservations
   from actual contiguous requests; pair local/parent allowances before each
   allocation and record simultaneous owner peaks, including upstream source.
4. Exact production opaque proof/semantic lending and transaction/draft API
   signatures, with no raw Thing aggregate entry, unchecked constructor or
   self-borrowing suspended owner. Specify acknowledgement and whole-debit
   Pending behavior, first-cause/cleanup ownership, end-of-loan boundary and
   concrete owned artifact restrictions.
5. Replace normalization/seal/equivalence-only states and the eight Snapshot
   readmission items with these actual TD/progress/resource/query/output
   obligations. Retain compiled-set and publication lifecycle/linearization;
   define the variable output reclaimer and extension abort eligibility before
   implementation admission.
6. Update the exact authoritative owners together: architecture 10/20/30/50;
   Foundation/runtime-safety and Planning specs; WP-100 admission/package,
   WP-200 and WP-400 contracts; API ownership and resource/state projections;
   Number amendment references where their config/feature entry changes;
   ADR/artifact/authority registrations; affected tranche requirements, evidence
   applicability and mechanism checkers. Reaffirm disjoint completed leaf,
   Producer and decode/semantic evidence at the reviewed diff. Change PLAN only
   for the durable prerequisite change, never this task/PR state.
7. Independent assessment of this exact prototype and then the migrated
   authority revision. Keep the validated-Thing tranche candidate until a
   separate reviewed admission transition. Later production acceptance must
   supply full global/accounting, complete-registration, compiler failure,
   variable cleanup, real publication/lease/drain, Host/static and target
   evidence. No gate is registered or closed by this investigation.

There is no unresolved product decision blocking this design task. An immediate
external-JSON or source-ownership deployment commitment would be the precise
input that reopens the chosen scope. The remaining technical risks above are
named evidence/implementation boundaries, not a concealed full-Snapshot
dependency.

## Local verification

On the 64-bit Host with Rust/Cargo 1.99.0, the following checks pass:

```sh
cargo test --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-admission/Cargo.toml
cargo test --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-admission/Cargo.toml --features order
cargo test --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-admission/Cargo.toml --no-default-features
cargo clippy --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-admission/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path tools/architecture-fixtures/consumer-borrowed-admission/Cargo.toml --no-default-features --target thumbv7em-none-eabihf
cargo test --locked --manifest-path tools/architecture-fixtures/validated-thing-schema-kernel/Cargo.toml --features validated-thing --test thing_basic --test owning_basic
cargo test --locked -p clinkz-wot-td --lib uri_semantic_kernel_probe
cargo test --locked -p clinkz-wot-td --lib external_planning
tools/check-design-artifacts.sh
```

Each new-fixture configuration runs sixteen runtime tests and five compile-fail
doctests, including the shared 180-case Basic corpus. The existing semantic and
owning-Basic checks run two and six tests; the prior external owned handoff runs
one. The shared URI suite runs eight tests, including cache invalidation for
every buffer mutation and rejection of invalid UTF-8 after a cached loan.
The prior handoff retains an existing unused-constant warning. Authority checks
preserve the active requirement/resource/state/DAG projections and all 66
performance cases. Formatting and diff hygiene pass. The existing mainline
toolchain is Rust 1.95.0; its workflow gains only this fixture's three Host
configurations and thumb compilation. Target compilation is not target runtime
acceptance, and local checks are not independent architecture acceptance.
