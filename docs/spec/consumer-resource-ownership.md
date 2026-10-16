# Consumer resource ownership migration

Status: accepted successor contract under ADR-0023 and ADR-0013 for
`WP-400-CONSUMER-RESOURCE-OWNERSHIP`. Implementation authority is limited to
that tranche's recorded scope. Existing `StrictV1` registrations retain their
contract and evidence scope; no existing footprint is implicitly converted.
The active 65/31/121 dispositions and TD typed-input interpretation revision 2
do not change.

This file owns the accepted detailed contract and API delta. The
[schema overlay](../resource-contracts/consumer-ownership-v2.toml) owns its exact
row dispositions. The [readmission record](../work-packages/WP-400-consumer-resource-ownership-admission.md)
owns implementation scope, evidence obligations and impact disposition.

## Scope and rejected expansion

Apply only to one startup Consumer-only registration, one eager generation and
one manual call/result opportunity per root, in the existing three compilation
cells. Two separately constructed roots may use the same real parent accounts.
Repeated reads require terminal acknowledgement. Producer, dual-role, Directory,
subscriptions, automatic executors, dynamic registration and HTTP SDK selection
retain their own contracts and admission boundaries.

Select a versioned ownership interpretation rather than deleting limits or
creating a shared allocator. Preserve the existing conservative engine-call,
cancellation and transfer allowances. Replacing them with incremental engine
growth or a general scratch pool would introduce post-acceptance engine failures
and scheduling machinery without a demonstrated need. Private protocol growth
may fail safely under a binding policy; this is not a promise that all declared
protocol maxima can succeed simultaneously.

## Specification and authority projection

Within the admitted tranche, these clauses refine only the scoped Consumer
operation. Other operations and every `StrictV1` registration keep their old meaning. Linked owner
sections below select this scoped interpretation; they do not create duplicate
requirement owners.

| Existing owner | Admitted scoped delta |
| --- | --- |
| Foundation `API-RESOURCE-001`, `RES-LIMIT-001..003`, `ADMIT-MEM-001` | Engine admission covers controlled backing and promised handoff capacity. Binding policy covers protocol-private state; deployment composes aggregate availability/fit. Apply revision 2 of the ownership schema explicitly. Preserve local/parent provenance, actual Layout/overlap, linear work and recoverable construction. |
| Binding `BIND-REG-001`, `BIND-MEM-001`, `BIND-STORAGE-001` | Capture the resource contract in the complete registration. Declare full concrete engine-held carrier/slot backing separately from protocol-private policy. Constrained inline state is never pointer-sized fiction. |
| Binding `BIND-DELIVERY-001`, `BIND-IO-001`, `BIND-CALL-CANCEL-001`, `BIND-HOST-CANCEL-001` | Check complete retained request/result backing at the engine handoff. Keep selected artifacts, both seals, preacceptance input recovery, accepted-error ownership, first cause and complete-object transfer. Protocol ingress is bounded before callback by its actual binding owner. |
| Planning `PLAN-BOUND-001`, `PLAN-ARTIFACT-001`, `PLAN-COST-003`; architecture 30 | Extend only the closed Consumer compiler's immutable configuration and complete-owner support capture as specified below. Preserve target-copy lowering, input/output types, all-bounds and paid progress. Cursor, artifact and output remain fully charged engine storage, including erasure, rollback and overlap. No TD/Planning algorithm or public API change. Runtime transport caches are not compiler artifacts. |
| Runtime `ADMIT-TXN-001`, `HANDLE-DROP-001`, `HOST-ASYNC-001`; architecture 40/50 | Keep prebacked status/manual destination, original accounts, atomic install, funded complete calls, source-free reclamation and child-before-parent release. Split the SDK disposal failure policy below. No asynchronous registration retirement is introduced. |
| ADR-0006/0011/0015/0022 | Scoped supersession for the admitted Consumer operation of transitive SDK maxima and universally nonallocating SDK disposal only. Keep pending/late values, whole-object cleanup, borrowed profiles, nonduplicated work and allocation-free engine fallback. Their rationale remains historical evidence. |

## Schema, applicability and identity

The numeric CSV remains byte-for-byte unchanged: 196 rows, ordering, units, zero
rules, defaults, requirement links and V1 profile identities. Its complete SHA-256
is pinned by the overlay. Schema revision 1 remains strict. Ownership schema
revision 2 is the pair of that numeric table and this operation overlay; it
reinterprets exactly 20 rows. The other 176 retain their entire prior meaning.
An unchanged row is not a claim that its deferred behavior is implemented.
No row is retired, renamed or added, and none becomes `NA` through this change.

Applicability is Consumer-only/manual-poll in `std`, `no-default` and
`async-no-std`; these are execution cells, not hardware or allocator guarantees.
The existing finite TD/Planning operation projections still check their required
fields. The Consumer policy must reject a missing applicable value, unsupported
cell, contract mismatch or illegal disposal policy before clock/input
observation, parent mutation, allocation or callback. Producer/dual-role bundles
must reject the owned interpretation. Unused legacy fields remain explicit
values, not newly inferred non-applicability. This does not freeze broad external
resource authoring or resolve the deferred schema-authoring work in topic 0047.

Configuration identity is `(ownership schema revision, typed-input revision,
operation, role, execution model, compilation cell, disposal policy, profile
origin, value digest, registration, call, input, result, work, timeouts,
scratch_bytes)`. Checked policy captures all these constructor settings
immutably. A binding's configuration and artifact compatibility capture its
resource-contract variant, concrete private policy, result bounds and supported
cells. An owned policy cannot reuse a strict
registration/artifact even if numeric limits are equal; checked-policy caches
also distinguish the complete policy identity. `APPLICATION_DEFINED` alone is
insufficient identity. Changing an engine quota need not invalidate a protocol
artifact whose complete admitted bounds still fit the new transaction.

The value digest is SHA-256 of ASCII `clinkz-resource-values-v1\0` followed by
the 196 entries in numeric order: one byte `0` for `None`, or byte `1` followed
by the value as eight little-endian bytes for `Some`. Revision/cell/disposal and
the other identity coordinates remain explicit, not inferred from that digest.
Named V1 tables may supply numeric input to an explicitly constructed owned
policy; their old names never silently select it. No implicit legacy conversion
or external cache migration is allowed.

The fixed resource-identity key uses complete-policy encoding revision 1:
SHA-256 of ASCII `clinkz-consumer-policy-identity-v1\0`, followed by:

1. The first eight coordinates above, through profile origin, each as UTF-8
   prefixed by its two-byte little-endian length. Revisions/profile origin use
   decimal integers without leading zeros. Role is `consumer-only`, operation
   is `consumer-property-read` and model is `manual-poll`.
2. The 32 raw value-digest bytes.
3. `registration`, then `call`: for each footprint, `retained_items` as four
   little-endian bytes, then `retained_bytes` as eight little-endian bytes.
4. Every element of `input[4]`, `result[2]`, `work[7]` and `timeouts[2]`, in that
   operand and element order, as eight little-endian bytes per element.
5. `scratch_bytes` as eight little-endian bytes.

Every independently supplied constructor operand participates. Derived TD/
Planning scratch projections and computed backing totals are not encoded again;
their original operands are. Arrays use the exact meanings/order documented by
`ConsumerPropertyReadPolicy::try_new`. This is a byte encoding, not Rust struct
memory or host-endian integers. The schema overlay fixes complete fixtures and
one-operand negative vectors, including scratch 65,536 versus 131,072 under the
same ResourceLimits. Its validator also perturbs every scalar operand separately.

Only this complete key may identify checked policies in caches, configuration,
diagnostics and evidence. The value digest alone addresses the numeric table;
the earlier candidate's table/coordinate-only policy keys are withdrawn. This
corrected the schema-2 candidate before independent acceptance without
reinterpreting any active profile or accepting a legacy conversion. Checked policy retains
all coordinates, values and constructor settings and exposes the complete key
through the existing `BindingConfigurationDigest` representation. This key
identifies engine policy, not protocol configuration or held parent capacity.
The complete registration's separate binding digest
incorporates its contract variant, private policy and result bounds; compiler/
artifact compatibility uses that configuration. Provisioning checks the variant
and bounds against policy without requiring identical unrelated engine quotas.

Resource diagnostics retain the identity above, actual field, scope, phase,
configured ceiling and safely known requested/observed amount, plus value origin
and the effective limiting local/parent owner. Fixed fallback status must retain
the domain (`engine-capacity`, `binding-policy`, or `deployment-failure`) and first
cause without allocating. Binding-private rejection names its private policy
key and configuration; it must not masquerade as a schema byte-row refusal.
Termination has no guaranteed in-memory diagnostic.

## Closed Consumer binding configuration

This is the narrow correction to workspace finding 0086. Extend the existing
source-reviewed `ResolvedTargetCompiler`, rather than projecting two different
configuration identities through every Planning candidate and compiler call.
The latter would change compiler input, continuation, envelope and selection
contracts without changing the target-copy algorithm. Accepting an arbitrary
replacement digest or relaxing support capture alone cannot establish the
required complete-owner correspondence. A shared-program layer is unnecessary
to resolve this configuration conflict.

Introduce one fixed immutable value, `ConsumerBindingConfiguration`, with private
fields and these checked operands: resource contract, target capacity, concrete
private-policy key, maximum result body bytes and maximum result media String
capacity. It captures the **current compilation cell**, with precedence `std`,
then `async-no-std`, then `no-default`. Its supported cell is that singleton;
this tranche adds no multi-cell mask or runtime cell override. Static and Host
representations in the same cell use the same semantic configuration. The
contract distinguishes strict-local from Host process-failure disposal.

`try_new` accepts only `ConsumerOwnedV2` or `ConsumerOwnedHostV2`, capacity
`1..=64`, positive result bounds representable by the supported full Payload/
String backing with checked arithmetic, and a private-policy key. The Host
contract is illegal outside `std`; `std` alone does not qualify its owner.
Construction is fixed, allocation-free metadata work with no client callback,
clock, parent-account access or protocol effect. It derives and retains the key;
there is no constructor accepting a final binding key or mutable configuration.

Binding-key encoding `closed-consumer-binding-v1` is SHA-256 of ASCII
`clinkz-consumer-binding-configuration-v1\0`, followed, in order, by:

1. One byte resource-contract tag: `1` for `ConsumerOwnedV2`, `2` for
   `ConsumerOwnedHostV2` (including their disposal interpretation).
2. One byte cell tag: `0` for `no-default`, `1` for `async-no-std`, `2` for `std`.
3. One byte target capacity.
4. The 16 raw bytes of `clinkz-target-v1`, the unchanged artifact format.
5. The 32 raw private-policy-key bytes.
6. Result body bytes, then media capacity, each as eight little-endian bytes.

Reject a derived key equal to **any** legacy strict capacity key. This explicit
namespace exclusion preserves strict/owned rejection independently of a hash
collision with that finite namespace. The schema overlay owns fixed vectors,
one-operand mutations and this exclusion's controls. Rust memory, enum
discriminants, host endianness and display strings are not the encoding.

`ResolvedTargetCompiler::try_new(capacity)` remains strict and returns exactly
`[0x63, 0x74, 1, capacity]` followed by zeros for all 64 capacities. Add
`new_owned(configuration: ConsumerBindingConfiguration) -> Self`; it consumes
the checked value without allocation. `configuration()` returns the actual
immutable strict or owned key. Native bounds/start/paid continuation checks,
the private supported Host adapter and its support descriptor must all use
that same key. Support must retain the actual configuration, never reconstruct
an owned key from capacity alone. Keep compatibility `clinkz-target-v1`: the
payload remains the same owned target, and compatibility never waives the
separate exact configuration/registration/generation checks.

Add the passive `consumer_configuration` getter to `PollClientBinding` and
Host `ClientBinding`, defaulting to `None` for existing strict clients. An owned
client returns `Some` of its actual immutable configuration. Its private-policy
key must derive from **all behavior-affecting settings of the captured client**,
including input/concurrency/queue/growth/retirement and disposal policy. The
binding owns that encoding and its source/configuration-specific evidence;
an application-supplied label cannot substitute. Shared infrastructure may
have a separate owner; its policy/compatibility dependencies participate, its
address and live counters do not. The getter is a bounded, allocation-free,
side-effect-free metadata read; it grants no callback/destructor attestation.
Interior policy mutation after capture is prohibited. Runtime occupancy may
change under the captured policy without changing its identity.

Complete Consumer-only registration construction checks the actual client's
metadata against the declared resource contract and explicit result pair, the
current cell, and `identity.configuration()`. Strict registration requires
`None`; owned registration requires matching `Some`. Check passive metadata
before operational callbacks, input/clock observation, allocation or parent
mutation. Consuming support capture additionally checks the actual closed
compiler's configuration/metadata against that same client and declarations;
the existing concrete static type and private Host adapter checks remain.
Dual-role and Producer bundles reject owned contracts. Every rejection returns
the original complete input/owner, including the original Host client allocation;
no request callback or replacement owner is introduced. Eligibility requires
source-specific evidence of actual policy correspondence and bounded behavior,
not equality of caller digests alone.

Planning forwards the complete registration identity unchanged into every
candidate and immutable generation-qualified artifact envelope. There is no
configuration translation around bounds/start/step. Core request acceptance
retains full binding id/generation/configuration/format, plan id and plan-set
generation matching; normal and late result sealing and exact recovery remain.

This key identifies **registration eligibility**, not reusable program content.
It includes no engine-policy key/quota, parent account, endpoint, Thing/plan
identity, generation, registration ordinal or process nonce. One compatible
registration may compile/execute multiple plans and target values. Equal keys
do not imply equal payloads, semantic equivalence, shared storage or authority.
Different eligibility keys do not require physically different immutable bytes.
No per-Thing compiler, policy copy or new owner is required by this correction.
Payloads may later reference separately owned immutable data under a separately
reviewed ownership contract; current envelopes, leases and physical funding
remain binding. Topic 0081's content/operand/persistence investigation remains
outside this tranche and non-blocking under these constraints.

## Compilation-cell projection amendment

This bounded amendment addresses the independently confirmed mixed-cell finding
in [PR #187](https://github.com/yushun1990/clinkz-wot/pull/187#discussion_r4290942708).
The exact contract/API/source-scope delta below was independently accepted in
[review 5544009375](https://github.com/yushun1990/clinkz-wot/pull/187#pullrequestreview-5544009375)
at `1afa7d335afee37d635f7269e31106174fb1c797` against base
`d4bde22f9276ad2d94bbd9d455277a6d99223e94`, with no blocking design finding.
Tranche readmission, changed implementation, predecessor evidence reaffirmation
and successor completion retain their separate ADR-0013 obligations. The
successor remains `candidate`; this scope acceptance does not itself admit
production implementation or accept a gate. Previously accepted encodings,
cells, owner checks and exclusions remain fixed.

Add `ConsumerBindingConfiguration::compilation_cell(&self) -> &'static str`
as a public `const` getter. It returns exactly `"std"`, `"async-no-std"` or
`"no-default"`, using the existing complete-policy coordinate spellings. Capture
the cell once in private checked configuration metadata at construction, with
the existing Core precedence: std, then async, then no-default. Read the captured
coordinate without deriving it from the digest or constructing another value.
No caller-supplied cell, new public cell type, mutable configuration, default/
unchecked constructor, override or multi-cell support mask is introduced.

Make `ConsumerCompilerSupport::consumer_configuration(&self) ->
Option<ConsumerBindingConfiguration>` a public `const` getter. It returns `None`
for strict support and the actual immutable `Some` retained after existing
complete-owner/client/closed-compiler capture checks for owned support. The
descriptor remains privately constructible and is borrowed through the existing
`ConsumerCompilerRegistration::support`. Reading/copying metadata grants no
client, callback, compiler, lease or destructor attestation authority. Both
getters perform bounded, allocation-free metadata work without clock, parent
access or operational callbacks.

At the start of owned `ConsumerPropertyReadPolicy::qualify`, obtain the captured
configuration through that support projection and compare its cell with
Servient's **own** feature-selected cell. Use the same private Servient cell
projection for this comparison and complete-policy encoding. Missing or
different owned metadata rejects with the original complete input and fixed
policy context before clock/input observation, allocation, parent mutation or
operational callbacks. Reconstructing a configuration would run Core's feature
selection again and cannot establish Servient's cell. Strict eligibility is
unchanged; no policy-key or binding-key translation is permitted.

Keep the existing mixed-cell production negatives. Completion requires zero
clock/allocation/callback observations, unchanged parent totals/peaks and exact
input/client recovery on rejection. Core native and supported Host capture must
project the actual configuration; strict support must return `None`. Matching
portable positives use the existing normal-library downstream runner's matching
dependency graph, while Servient's mixed dev graph remains the negative control.
Relocate affected positives within the admitted fixtures if necessary; do not
ignore the negative or add a substitute runtime/compiler. Run all three Consumer
cells and both existing ARM compile cells, and renew the affected layouts and
failure measurements required by the admission impact table. ARM remains
compilation/layout evidence only. Independent readmission and completion review
are separate; neither the accepted scope nor passing tests reaffirms predecessors
or changes gate verdicts.

## Backing and ownership rules

Use three distinct declarations; summing them is not an engine measurement:

- **Engine-held**: actual concrete raw call/associated slot, Core envelope/seal,
  registration wrapper/client/compiler representation, request storage, promised
  result backing, output/artifacts/cursors, records and cleanup destinations.
  Include capacities, alignment and simultaneous copies/checkout/return overlap.
- **Binding-private**: socket/session buffers, parser/TLS state, protocol queues,
  SDK tasks/callbacks, pools and scratch controlled by that binding. A concrete
  policy specifies supported input, concurrency, retained queue items/bytes,
  growth refusal and retirement. The engine captures its configuration identity;
  it neither provides an allocator nor certifies its implementation.
- **Deployment**: shared runtime, stacks, allocator overhead/fragmentation, OS
  buffers and process limits. State hard enforced limits, measurements/estimates
  and assumptions separately. An OS kill limit does not bound remote-driven
  retained growth or qualify a binding.

An engine-held value's inline representation and engine output are charged even
when a binding defined the Rust type. Merely putting a `Vec`, `Arc` or future
behind an engine-owned pointer does not classify its backing: name the actual
physical owner and allocation controller. Protocol-private backing may stay with
the complete raw call while engine admission covers that call's concrete box.
Once backing crosses into an engine request, retained result, artifact or queue,
the complete backing must fit engine admission or carry an already funded,
identity-checked owner lease. This tranche introduces no new leased payload API.

Current `Payload::body` is a complete `Arc<[u8]>`; account the full allocation
including its control block/alignment, not only useful payload bytes. Media
Strings use capacity, not length. URI maps include nodes and String capacities.
Do not invent a protocol-memory census from content length. A binding must
produce a supported complete result representation within the promised backing;
normal and late handoff checks reject an overrun as an accepted binding error,
retain the call/lease/cleanup destination and never allocate a replacement seal.
Inspection/disposal of a rejected result remains within the binding's supported
finite output/destructor envelope. No unbounded value is accepted temporarily
by claiming it will be rejected later.

Shared private backing has one real owner. Registrations/calls retain bounded
leases; cancelling one lease cannot close another user's infrastructure. Last
user retirement must cover original tasks/callbacks and disposal. A completed
semantic result, closed flag or residual is not that retirement barrier. Charges
follow every still-live physical engine owner and each applicable private policy
until physical release or acknowledged transfer; a residual never authorizes
freeing surviving local capacity. A separately retained external infrastructure
owner has its own explicit lifetime and shutdown proof. The current runtime has
no asynchronous retirement hook: captures needing one remain unsupported.

## Reservation and disposal mechanics

Preserve current reservation sequencing. Let `C` be the conservative complete
engine call allowance: concrete raw state plus Core envelope, supported URI-map
backing and complete maximum result backing. Registration has its own engine
carrier/backing allowance. Compiler/output storage remains under PlanFootprint.
Constructor overlap is distinct temporary capacity. Reserve actual allocation
Layouts before allocation; an aggregate allowance is not a contiguous request
or an observed physical peak. Conservative preflight may refuse an aggregate
without reporting it as an actual allocation.

Keep the complete `C` cancellation cap and `C + destination/context/status`
transfer cap before the first side effect, with one manual fallback destination.
These overlap the call's policy caps; physically live storage is counted once in
engine live accounting. A move does not allocate a second call or release the
original parent. Refused transfer retains the identical complete object, first
cause, result, account and destination. Cleanup/status capacity cannot be spent
on ordinary work. Do not reduce these caps to pointer sizes.

`BindingTransientFootprint` in the owned contract covers controlled mediation
scratch. Keep that part conservatively startup-reserved through registration
disposal; remove only protocol-private scratch from mandatory engine admission.
No callback-time engine scratch acquisition, new pool or changed concurrency is
needed. Binding-owned scratch acquires its capacity before private growth and
retains existing funded owners if refused. Strict deployments may additionally
reserve protocol maxima in their own real account, without charging one shared
physical pool once per Thing or call.

`strict-local` keeps the existing nonallocating finite disposal contract.
`host-process-failure` explicitly permits finite local SDK allocations during
progress and disposal while the declared Host process/runtime lives. It is legal
only for a qualified `std` Host cell and never selected merely by `std`.
Engine residual/status commitment, failure fallback, complete handoff and Core
postacceptance envelope/seal installation remain allocation-free. Commit required
residual status before unresolved destruction; retain original engine funding
through the destructor. Count/audit SDK disposal work and allocations separately.
Neither policy permits network progress, blocking joins, replacement work,
unbounded destruction, detached local jobs or successful shutdown before actual
required local retirement. Supported returned allocation errors preserve owners;
abort/OOM termination supplies no cleanup/report guarantee. Strict embedded fit
and continued-operation claims require separate complete-target evidence.

## Admitted API projection

These are the exact registered changes for the successor; implementation
follows its indexed admission disposition. Existing
frozen API rows continue to describe `StrictV1`; the matching implementation
must update only the necessary registered rows under this projection.
The API matrix also records the successor additions as `absent/add` contract
targets; those rows do not assert that the APIs exist before readmission.
No generic binding-policy trait or central manager is introduced.
The two cell-amendment rows below record the accepted scoped delta above;
tranche readmission remains separate.

| Package / API | Registered change |
| --- | --- |
| Foundation `ResourceLimits`, `ResourceKind`, `ResourceProfileId`, account/reservation APIs | No signature, generated ordering or value changes. Raw assembly remains unqualified. The checked Consumer policy binds schema identity and retains its full immutable values; exported cache/config records carry the explicit identity above. |
| Core `BindingResourceContract` | Add `StrictV1`, `ConsumerOwnedV2`, `ConsumerOwnedHostV2`. The latter two select schema 2 with strict-local and host-process-failure disposal respectively. |
| Core `BindingResourceDeclarations` | Add captured contract and `with_resource_contract(self, BindingResourceContract) -> Self`, plus `resource_contract(&self) -> BindingResourceContract`. Existing constructors select `StrictV1`. Add `with_consumer_result_limits(self, body_bytes: u64, media_capacity: u64) -> Self` and `consumer_result_limits(&self) -> Option<(u64, u64)>`. An owned complete registration requires explicit supported result limits and rejects unsupported role/cell or inconsistent metadata. Concrete private policy is part of its existing binding configuration, not an engine allocator interface. |
| Core `ConsumerBindingConfiguration` | Add the checked immutable `Clone + Copy + Debug + Eq + PartialEq` value above. `try_new(contract: BindingResourceContract, target_capacity: usize, private_policy: BindingConfigurationDigest, body_bytes: u64, media_capacity: u64) -> CoreResult<Self>`; getters `resource_contract(&self)`, `target_capacity(&self)`, `private_policy(&self)`, `result_limits(&self)` return those operands, and `configuration(&self) -> BindingConfigurationDigest` returns the derived key. No public fields or unchecked/default constructor. |
| Core `ConsumerBindingConfiguration::compilation_cell` — cell amendment | Public `const fn compilation_cell(&self) -> &'static str`; exactly the three captured singleton-cell spellings specified above. Accepted contract; production implementation awaits tranche readmission. |
| Core `ConsumerCompilerSupport::consumer_configuration` — cell amendment | Public `const fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration>`; passive projection of actual closed support, `None` for strict. Accepted contract; the same readmission boundary applies. |
| Core `ResolvedTargetCompiler`, `ConsumerCompilerSupport`, supported Host adapter | Add `ResolvedTargetCompiler::new_owned(ConsumerBindingConfiguration) -> Self`. Preserve strict constructor/key, compatibility, cursor/artifact types, target-copy algorithm and paid work. Capture and forward the actual immutable configuration through support. Remeasure the compiler/support/adapter, cursor/output/temporary and aggregate owners; strict source compatibility does not promise unchanged Layout. |
| Core `PollClientBinding`, Host `ClientBinding`, complete Consumer-only constructors/capture | Add `consumer_configuration(&self) -> Option<ConsumerBindingConfiguration>` with default `None`. Validate the actual owner's immutable metadata as specified above, without new execution or mutable-client escape. Existing complete signatures and exact recovery stay. Generic compiler/Host erasure remains ineligible for closed support. |
| Core `BindingLifetimeFootprint`, `BindingTransientFootprint`, `BindingStateLayout` | Keep constructors/getters. Their interpreted scope is fixed by the captured contract, not by an untagged footprint alone. Owned retained items count engine-held owners; retained bytes include full concrete backing above. Static size/alignment/drop checks stay. |
| Core installed Consumer constructor/normal and late seals | Capture the admitted result-backing bound with the existing selected seal; check full backing as well as existing semantic validity before retention. Preserve recoverable raw construction, accepted-overrun owner and allocation-free seal installation. No raw installed-client escape or replacement envelope. |
| Core `ResourceFailureDomain`, `ErrorContext` | Add the fixed enum `EngineCapacity`, `BindingPolicy`, `DeploymentFailure`; extend context with an optional resource-identity key/domain and effective local/parent `(SlotIndex, Generation)` coordinates. Add `with_resource_failure(identity, domain, local, parent) -> Self` and `resource_failure(&self)` returning the same optional fixed tuple. Existing contexts default to absent. Keep error categories/retry/first-cause rules and bounded private-policy cause keys; account the larger complete error/accepted-overrun envelope. |
| Servient `ConsumerPropertyReadPolicy` | Existing `try_new` remains strict. Add `try_new_owned(contract, profile_origin: ResourceProfileId, ...existing try_new operands...) -> CoreResult<Self>`; reject `StrictV1` here. Expose `resource_contract`, `resource_identity`, `profile_origin` and borrowed `resource_limits`. Compute `resource_identity` from all independent constructor operands under the complete-policy encoding above, including controlled registration/call/input/result/work/timeout/scratch settings. Provisioning verifies the matching registration contract/cell and that its result limits fit the policy before effects. Servient stamps fixed resource-failure context with this complete policy key. |
| Servient Host/static roots, reads, calls, cleanup | Keep signatures, manual ownership, real parent bank and complete-object transfer. Interpret only an explicitly owned policy through the new rules. No async retirement API. |
| TD / Planning public API | Unchanged; regenerate affected layout evidence if captured Core metadata changes size. Neither compiler support nor typed proof grants binding/runtime eligibility. |

The implementation must export sufficient fixed diagnostic/configuration identity
from the checked policy; it may choose private helper decomposition. Changing
these public semantics, weakening backing checks or discovering an omitted
storage owner returns to scoped review before changed source proceeds.
