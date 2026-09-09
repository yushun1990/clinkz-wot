# 0065 liballoc BTreeMap Retained Representation Impact

Status: DECIDED

Kind: ADR-0013 implementation-impact review and retained-representation
architecture decision

Finding baseline: `339314686fb3dede6249f615bb99765dafe57548`

Decision baseline: `3516c701f951e7ec80e916c0f23a56c2651c5052`

Blocked candidate: github-pr:74 (closed without merge)

Impact review location: github-pr:75

Decision review location: github-pr:76

## Question

What stable, explicit, and verifiable retained representation can preserve the
same TD semantics while supporting provable resource admission on ordinary
stable Rust, Host, and `no_std + alloc`, without making caller allocation
history or private liballoc layout part of the product contract?

This topic now selects an architecture direction. It remains a non-authoritative
workspace decision until the conclusion is migrated into every affected
authority and independently accepted. It does not authorize production Rust,
readmit the tranche, produce completion evidence, or make any WP-100, Consumer
architecture-gate, or successor-tranche claim.

## Impact finding

Github-pr:72 corrected the earlier serde_json representation blocker by pinning
serde_json 1.0.149 and rejecting feature-unified IndexMap-backed Map and
String-backed Number representations at compile time. The Map guard establishes
that serde_json uses the outer BTreeMap-backed representation; it does not
establish the allocation layout or retained state of BTreeMap's private nodes.

Github-pr:74 implements `btree_allocation_bytes()` using assumptions that exist
only in the current private liballoc implementation:

- node capacity is eleven key/value slots;
- a non-root node retains at least five keys;
- node count is bounded from current map length using that occupancy rule; and
- key/value slot sizes plus a derived pointer/metadata allowance bound every
  leaf and internal allocation.

The accepted authoritative artifacts do not freeze those constants, node
layouts, supported-target layouts, allocation-retention rules, or a
Rust/liballoc identity. The repository's Rust 1.95.0 CI install makes remote
validation reproducible, but it is not an enforced downstream representation
contract. The serde_json guards compare the outer Map/BTreeMap size and Number
drop behavior only. A liballoc node change can therefore leave TD compiling and
both guards passing while invalidating the byte envelope.

## Exact-head counterexample

The issue already occurs on github-pr:74 exact head
`339314686fb3dede6249f615bb99765dafe57548` with the same Rust 1.95.0 toolchain
used by CI. That liballoc implementation permits an empty BTreeMap to retain an
allocated empty leaf root after its last element is removed, while
`btree_allocation_bytes(length == 0)` returns `(0, 0)`.

The external probe used rustc 1.95.0 commit
`59807616e1fa2540724bfbac14d7976d7e4a3860` on
`x86_64-unknown-linux-gnu`. It built two Basic-valid Things with equal JSON
content. Each retained an extension array of 1,024 empty serde_json Maps. The
first used newly constructed empty maps. For every map in the second, the probe
inserted one `("removed", Value::Null)` entry and removed it before moving the
map into the Thing. A custom global allocator counted requested live allocation
sizes around successful `ValidatedThing` construction and asserted that drop
returned the counter to its original baseline.

Observed results:

| Input | Reported retained bytes | Measured live heap bytes |
|---|---:|---:|
| 1,024 fresh empty maps | 40,436 | 39,372 |
| 1,024 retained empty leaf roots | 40,436 | 686,540 |

The 647,168-byte difference is exactly 1,024 allocations of 632 bytes. With a
configured retained-source ceiling of 65,536 bytes, the second Thing still
reached `Complete` because its reported value was 40,436, although measured
live heap alone was 686,540 bytes. The inline retained value size is not needed
to falsify the bound; allocator-observed heap alone exceeds both the report and
the configured ceiling.

## Why current evidence does not detect the gap

All of the following passed at the rejected exact head:

- the supported default and no-default serde_json representation cells;
- the negative `preserve_order`, `arbitrary_precision`, and combined guard
  cells;
- the focused TD unit and integration tests; and
- github-pr:74 mainline CI.

Those checks prove the serde_json backing selection and observable String/Vec
capacity cases. They do not observe BTreeMap node allocations, exercise
insert/remove history that leaves an empty root, validate private leaf/internal
layouts, or relate allocator-observed retained bytes to every reported census
result. Consequently the branch-only completion evidence statement that the
census "conservatively accounts every reachable owned allocation" is
falsified. Github-pr:74 never merged, so master contains no completion evidence,
`complete` state, or PLAN completion text to revert.

## Constraints that remain binding before migration

- `WP-100-CONSUMER-VALIDATED-THING` remains `planned` and its admission is
  withdrawn to `candidate` under ADR-0013.
- No production Rust may proceed until a conclusion is migrated into the
  applicable authoritative artifacts, independently accepted, and followed by
  a separate admission-only transition.
- Any retained-source claim must cover the real retained representation and all
  publicly reachable construction histories, not only serialized content,
  current length, or an outer `size_of` equality.
- The prior serde_json feature guards remain useful evidence, but do not answer
  this liballoc question.
- Foundation changes merged by github-pr:69 and the Context inspection seam
  merged through github-pr:70 are not reopened by this finding.
- No WP-200, WP-400, Consumer architecture gate, or successor scope enters this
  investigation.

## Repository conclusions that control this decision

The four relevant pull requests establish one cumulative result rather than
four independent implementation preferences:

1. Github-pr:71 proved that legal downstream Cargo feature unification can
   change opaque `serde_json::Map` and `serde_json::Number` allocations without
   changing the public semantic value observed by the candidate census.
2. Github-pr:72 accepted an exact serde_json pin and compile guards as a narrow
   repair while retaining the exact caller-owned `Thing` contract. That review
   established only the outer serde_json representation, not liballoc's
   retained allocation graph.
3. Github-pr:74 was closed without merge after the retained-empty-root
   counterexample falsified its BTreeMap envelope on the exact CI toolchain.
4. Github-pr:75 correctly withdrew admission and left this architecture choice
   open. Its Foundation changes and Context seam remain useful history but do
   not make the rejected accounting model sound.

The repeated failure is not an isolated missing constant. The contract combines
two independently expensive promises:

- preserve the exact caller allocation graph, including spare capacity, node
  topology, pointer identity, and mutation history; and
- prove a useful conservative byte ceiling for every allocation reachable from
  that graph.

For public `Thing` values containing private standard-library and serde
containers, those promises force private dependency representation to become a
ClinkZ-WoT compatibility surface. That combination is unnecessary for the
Consumer product semantics and is rejected.

## Decision

Select directions 2 and 3 together:

- validation/admission may losslessly normalize the caller's semantic TD; and
- the successful `ValidatedThing` owns a TD-project-controlled, immutable,
  allocation-observable retained source representation.

Direction 3 alone is insufficient: rebuilding into another `Thing` backed by
`BTreeMap` or `serde_json::Value` merely creates a fresh opaque allocation
graph. Direction 2 alone is also insufficient if it attempts to wrap only new
values while retaining an already-constructed caller `Thing`. The proof
boundary is closed only when successful normalization severs every ownership
edge to the caller's opaque allocation graph and the complete retained value is
project-controlled.

### Semantic identity is retained; allocation identity is not

The contract preserves **the same TD semantic value**, including:

- every known TD field and its present/absent distinction;
- string and URI contents;
- Context, Form, and other semantically ordered sequence positions, including
  original Form indices;
- map key/value associations and every extension name/value;
- nested JSON arrays, objects, booleans, nulls, strings, and numbers without
  lossy floating-point coercion; and
- the result of the one TD-owned Basic-validation and effective-operation /
  security interpretation.

The contract does **not** preserve:

- addresses of the input `Thing` or any nested value;
- `String`/`Vec` capacity, BTree node identity or occupancy, serde_json backing
  capacity, allocator bin choice, or insert/remove history; or
- JSON object storage order where the TD/JSON data model assigns no ordering
  semantics. Ordered arrays, Context entries, and Forms remain ordered.

This is lossless relative to the typed `Thing` semantic input, not a promise to
preserve whitespace, source-token spelling, or other raw-document evidence that
the `Thing` value no longer contains. Exact source-text/evidence retention, when
required, belongs to the separately charged document/source envelope rather
than this Consumer retained representation.

Physical allocation or pointer identity has no identified application-visible
WoT behavior. Planning selects semantic property/Form coordinates; Servient
owns a generation; bindings consume compiled plans. None requires the caller's
original pointer graph. ClinkZ-WoT therefore does not make that identity a v1
product semantic.

### Retained representation authority

`Thing` remains the ordinary public authoring/interchange value. It need not
replace every public `BTreeMap` or fork serde_json merely to support one retained
runtime boundary.

The completed `ValidatedThing` instead owns one private TD retained snapshot
with these mandatory properties:

1. Map/object storage is project-owned ordered storage, such as sorted
   exact-length slices or ranges in a flat arena. Sequence order is explicit.
2. Strings, byte content, extension JSON nodes, and lossless number content are
   stored in project-owned exact-length blocks or in ranges of project-owned
   arenas.
3. A sealed snapshot has an exhaustive allocation-site catalog. Every retained
   allocation exposes its element count/capacity or was requested by a
   project-owned checked `Layout`; mutable build capacity is not silently
   treated as final exact capacity.
4. No completed snapshot retains a `Thing`, `BTreeMap`, opaque
   `serde_json::Map`/`Number` allocation, caller `Vec`/`String` spare capacity,
   or pointer into the normalization input.
5. The physical representation remains private. Its public surface is a
   storage-independent immutable TD semantic view, so the project may improve
   arena packing without changing downstream TD semantics.
6. The same representation and semantic view compile in Host and
   `no_std + alloc`; they require neither a global counting allocator nor
   pointer-width atomics.

A single canonical byte blob without a TD-owned typed view is not sufficient.
It would force WP-200 to parse and semantically reinterpret the TD, or to create
another opaque temporary `Thing`. The retained snapshot must provide borrowed,
allocation-free typed access and TD-owned effective-operation/security queries
needed by Planning.

### Accounting authority

The accounting authority is the project's exhaustive retained allocation-site
catalog plus checked public length/capacity/`Layout` facts, not rustc, liballoc,
serde_json private layout, or allocator introspection.

The retained footprint records at least:

- total requested bytes for every live allocation owned by the sealed snapshot;
- inline retained owner bytes where the account includes them;
- allocation count; and
- largest requested contiguous allocation.

“Requested bytes” means the `Layout::size` requested for the live project-owned
payload. Allocator-private headers, bins, and platform rounding are not
reachable TD storage and cannot be portably observed by a Rust library. A
profile that needs allocator overhead must add a separately owned
allocator-specific surcharge; it may not silently reinterpret the portable TD
footprint.

The existing `retained_source_bytes()` name may survive only as a derived
scalar for the normalized snapshot. It is no longer a census of the caller's
exact `Thing` and cannot remain the sole authority when largest-contiguous and
conversion-peak facts are required. Authority migration must freeze a
structured source-footprint handoff even if a scalar convenience accessor is
kept.

### Normalization work and peak memory

Normalization is an explicit unpublished admission phase, not an uncharged
clone or an implementation detail. It must:

1. walk semantic nodes under the existing specific work classes and charge
   emitted/copied source bytes before bulk work;
2. compute checked final sizes before retaining final storage, or reserve a
   conservative project-owned build capacity whose observed capacity is
   charged immediately;
3. reserve final-source bytes, temporary cursor/arena bytes, the largest
   contiguous request, and any grow/seal reallocation overlap before those
   allocations can occur;
4. drop partial normalized storage and release every reservation on invalid,
   limit, cancellation, or conversion failure; and
5. destroy or return the caller input before `Complete` is exposed, so only the
   project-owned snapshot crosses the retained/publication boundary.

An arbitrary caller `Thing` may already contain unobservable excess allocation
before ClinkZ-WoT is called. No library can retroactively bound that pre-entry
process-memory baseline through stable collection APIs. The compatibility
normalizer therefore gives a precise **additional peak over entry baseline**:
all bytes newly allocated or simultaneously retained by ClinkZ-WoT conversion
are reserved and charged, while no absolute claim is made about memory the
caller had already allocated.

For application-static users that require an absolute input-through-retention
bound, the authoritative API must also permit construction/decoding directly
into the project-owned source builder under the same work and byte reservations.
That path accounts storage from its first allocation and feeds the same
normalizer/validator; it is not a second semantic implementation. Host users
may use the ordinary `Thing` compatibility conversion without an exact rustc or
target restriction.

### Public API and ownership effect

`ValidatedThing` remains a non-`Clone`, move-only proof owner constructed only
by TD. Successful construction still proves complete Basic validation and the
checked structural counts needed by Planning.

The current `ValidatedThing::thing() -> &Thing` contract must change. It exposes
the physical representation that this decision deliberately stops retaining.
The migrated API instead exposes a borrowed storage-independent validated TD
view and a structured retained-source footprint. An optional reconstruction of
an owned `Thing`, if product use demonstrates a need, is an explicit charged
conversion that promises semantic equivalence, not pointer/capacity identity;
it is not part of the retained Planning/Servient path.

Invalid, limit, and cancellation ownership must remain linear and unpublished,
but their migrated terminal type must not require reconstructing or preserving
the original pointer graph after normalization work has begun. The exact
terminal spelling is frozen during authority migration together with its
rollback and conversion-peak proof.

The serde_json exact pin and the `preserve_order` / `arbitrary_precision`
compile failures are no longer part of the selected product boundary. Legal
feature unification is accepted as input and normalized through stable semantic
APIs. The crate follows an ordinary declared stable-Rust/MSRV and semver
dependency policy; it does not compile-guard an exact rustc/liballoc source or
supported-target layout.

### One TD semantic authority

Normalization owns storage, not TD meaning. `Thing` Basic validation remains a
single TD-owned rule implementation. It may be refactored to operate on one
internal semantic view shared by the authoring value and retained snapshot, but
the normalizer must not copy validation/defaulting rules into a second tree
walker merely to construct storage.

Planning receives only the validated TD view and TD-owned effective-operation /
security queries. It must not deserialize a blob, validate extension JSON,
reinterpret default operations, or reconstruct a raw `Thing`. Semantic
round-trip/equivalence evidence is mandatory at the TD boundary.

### WP-200 and Servient handoff

The stable downstream source contract is one owned `ValidatedThing` plus a
borrowed immutable semantic view:

```text
caller Thing or project-owned source builder
  -> bounded TD normalization + the single Basic semantic validator
  -> move-only ValidatedThing { private normalized snapshot, footprint, counts }
  -> Planning borrows the typed semantic view
  -> Planning emits a TD-free sealed draft
  -> Servient retains the ValidatedThing owner and reclassifies its exact bytes
```

Original Form ordering/indices and deterministic map-key ordering remain
available to Planning. Servient retains one source owner and one exact footprint
and never needs caller allocation metadata. The normalized snapshot is not a
second plan, effective TD, or semantic authority.

## Direction disposition

### 1. Exact rustc/liballoc plus supported-target freeze — rejected

This could be made enforceable only by turning exact compiler, liballoc source,
target layout, serde version/features, and allocator evidence into supported
build behavior. Every supported target and toolchain update would require a
private-node proof over allocation and deletion histories. Custom toolchains
and normal downstream stable updates would fail before using otherwise portable
TD semantics. Even an exact liballoc freeze would not define allocator-private
overhead. The compatibility and recurring proof cost are disproportionate to a
pointer-identity promise with no product consumer.

### 2. Project-owned observable retained representation — selected with 3

A private normalized snapshot gives the project an exhaustive allocation
catalog without changing `Thing` into a storage-policy type. Replacing every
public TD map and every serde_json value with a new public wrapper was rejected:
it would impose a broad source/API migration, leak retained-storage concerns
into authoring, and still need normalization for existing `Thing` inputs.

### 3. Lossless normalization/rebuild — selected with 2

Normalization is the necessary boundary that discards caller capacity and
mutation history. It is accepted only when its output is the project-owned
snapshot, its work and additional peak are admitted, and semantic equivalence
is proved. Cloning, serializing, or rebuilding into another opaque standard
collection is not a correction.

### 4. Stable-public-API-only conservative accounting — rejected as unprovable

`BTreeMap` exposes semantic length and iteration but no node count, capacity,
retained-root state, or allocation-history bound. The exact counterexample has
equal public state and different live allocations. Future conforming liballoc
implementations may change node layout and retention while preserving every
stable observable. Consequently no finite useful function of stable public
state proves the requested physical bound for every supported implementation.
An address-space-sized charge is not useful admission, and a guessed constant
is not conservative proof.

## Other rejected non-solutions

- fixing only empty retained roots or raising the current per-node constants;
- treating CI's Rust 1.95.0 install or serde wrapper-size guard as downstream
  representation authority;
- using serialized length, `size_of::<Thing>()`, logical node count, or the
  configured ceiling as if it bounded caller allocation history;
- relying on a global allocator or unsafe private-layout casts in production;
- retaining both the exact caller `Thing` and a normalized snapshot after
  publication;
- retaining only bytes and making Planning parse or semantically validate them;
  and
- weakening the evidence wording while continuing to enforce a physical byte
  ceiling from content-only census.

## Required authority migration

This `DECIDED` topic is not implementation authority. A later docs-only
migration must update, at minimum:

- `docs/spec/runtime-safety.md`: replace exact-Thing/serde-guard retention with
  semantic-equivalence, normalized snapshot, and no-raw-Thing publication
  rules;
- `docs/spec/foundation.md`: define requested-layout accounting, source versus
  temporary normalization accounts, additional-peak baseline, strict
  project-owned input construction, reconciliation, and reclassification;
- `docs/work-packages/WP-100-core.md`: replace the successor API/storage/work
  boundary and enumerate all legitimate TD implementation paths;
- `docs/work-packages/WP-100-consumer-validated-thing-admission.md`: supersede
  the github-pr:72 representation correction, freeze the exact public terminal
  and view/footprint APIs, supported cells, conversion phases, permitted paths,
  and evidence identities;
- `docs/api-ownership.csv`: remove the exact `validated_thing_thing` projection
  and register the storage-independent view, structured footprint, and any
  strict source-builder/conversion surface actually selected;
- `docs/architecture/10-primary-data-flows.md` and
  `docs/architecture/20-module-boundaries.md`: replace the exact retained Thing
  in the Consumer flow with TD normalization, the semantic view, and one
  Servient-owned snapshot;
- `docs/work-packages/index.toml`: update authoritative artifacts, API items,
  implementation paths, checks, evidence key/path, and the independently
  reviewed readmission revision while leaving the tranche `candidate` until a
  separate admission-only transition;
- `docs/requirements.csv` only if the registered evidence projection or owner
  path changes; this decision refines existing `DOC-RUNTIME-001`,
  `ADMIT-MEM-001`, `ADMIT-TXN-001`, and constrained-work/ownership identities
  rather than activating a new requirement.

This decision PR corrects `PLAN.md` only to the coarse current fact that PR #75
withdrew admission and that a workspace decision is not implementation
authority. A later migration changes the roadmap again only if its durable
frontier changes. ADR-0013's impact/readmission process and ADR-0019's Consumer
one-shot sequence remain applicable and need no semantic amendment for this
storage decision.

The migration must explicitly review the ordinary stable-Rust/MSRV policy,
serde_json semver/features, Host target, and at least one real `no_std + alloc`
target. It must not encode exact liballoc layout as compatibility authority. The
existing resource rows appear sufficient; adding a field or work class requires
a concrete falsifiable gap rather than being assumed by this decision.

## Readmission evidence required before production work

Before a separate admission-only transition, the migrated docs revision must
have independent acceptance and executable pre-code evidence for:

- one exact public ownership/terminal/view API with no successful
  `&Thing`/mutable/raw-storage escape and no pointer-identity promise;
- a complete retained allocation-site catalog and checked formulas for total
  requested bytes, allocation count, largest contiguous allocation, temporary
  bytes, seal/reallocation overlap, and additional conversion peak;
- one allocation-free TD semantic view sufficient for the registered
  Property/Form/effective-operation/security Planning inputs, without WP-200
  implementation;
- a fixed semantic-equivalence corpus covering known fields, optional
  distinctions, Context ordering, Form ordering/indices, nested extensions,
  long strings, and lossless numbers;
- supported Host and real `no_std + alloc` compilation plans using ordinary
  stable toolchains, plus downstream `preserve_order`, `arbitrary_precision`,
  and combined feature cells that are expected to succeed rather than fail;
- zero-budget/no-progress, non-resettable lifetime-work, cancellation, and
  rollback contracts for normalization as well as Basic validation;
- exact source/temporary/peak/contiguous reservation order using current
  resource rows, including the explicit pre-entry-baseline limitation of the
  arbitrary-`Thing` compatibility adapter; and
- the unchanged disposition of github-pr:69 Foundation work, github-pr:70's
  Context seam, and the passed Producer Property Read gate.

Only after those artifacts and checks are reviewed may a separate change move
`candidate -> admitted`. This decision PR does not perform either step.

## Completion evidence required after a future implementation

The replacement completion evidence must record the exact implementation and
passing results that can falsify the new boundary:

1. Re-run the 1,024 fresh-empty versus retained-empty-root BTreeMap reproducer.
   After normalization and destruction of the input Things, both semantic
   outputs must have equal snapshot footprints, and allocator-observed live
   requested bytes for each snapshot must not exceed its report.
2. Re-run compact versus capacity-16,384 serde Maps and short versus long /
   spare-capacity arbitrary-precision Numbers under `preserve_order`,
   `arbitrary_precision`, and combined downstream unification. All supported
   cells must compile; output footprints must follow normalized semantic
   content, not caller spare capacity, and allocator-observed retained bytes
   must be covered.
3. Cover every typed map and every public nested extension-`Value` reachability
   path, with source inspection proving that a completed snapshot owns no
   opaque standard/serde allocation graph.
4. Prove semantic equivalence before/after normalization, Basic-validation
   positive/negative parity through one rule implementation, exact ordered
   sequence/Form-index retention, deterministic key iteration, and lossless
   extension-number handling.
5. Measure maximum additional live bytes during count/build/seal, including
   input-plus-output overlap, capacity growth, exact-length sealing, failure,
   cancellation, and rollback. Observed project-owned peak and largest request
   must stay within the reservations and return to baseline on drop.
6. Exercise every relevant source, temporary, retained, peak, contiguous,
   structural, checked-arithmetic, and lifetime-work boundary at below/equal/
   above-limit values. No rejected allocation or work unit may occur before its
   charge.
7. Prove zero-budget no-progress and equivalent Host/application-static results
   across varied step sizes on the same cursor algorithm.
8. Compile and execute the frozen supported stable-toolchain/target matrix,
   including a real `no_std + alloc` allocator-observed or project-allocation-
   traced fixture rather than a compile-only parity claim.
9. Compile an external Planning-view consumer that enumerates the required
   semantic coordinates without `&Thing`, raw retained storage, reparsing, or
   duplicate Basic/default/security logic. This is a contract fixture, not
   WP-200 implementation or admission.
10. Re-run the exact Producer Property Read gate impact set and normal locked
    workspace/authority validation without changing that passed gate.

The evidence must claim only `WP-100-CONSUMER-VALIDATED-THING`. It must not
readmit or enter WP-200, WP-400, the Consumer architecture gate, or any
successor tranche.
