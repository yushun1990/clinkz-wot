# 0065 liballoc BTreeMap Retained Representation Impact

Status: OPEN

Kind: ADR-0013 implementation-impact review and retained-representation
architecture question

Finding baseline: `339314686fb3dede6249f615bb99765dafe57548`

Blocked candidate: github-pr:74 (closed without merge)

Review location: github-pr:75

## Question

What stable, explicit, and verifiable representation authority can support the
`WP-100-CONSUMER-VALIDATED-THING` contract that
`retained_source_bytes()` conservatively covers every reachable allocation of
the exact retained `Thing`, when its public representation contains
`alloc::collections::BTreeMap` values whose node allocations are private to
liballoc?

This topic records an architecture question and impact analysis only. It does
not select a correction, authorize production Rust, readmit the tranche,
produce completion evidence, or make any WP-100, Consumer architecture-gate,
or successor-tranche claim.

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

## Constraints that remain binding during investigation

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

## Directions requiring evaluation

No direction below is selected or authoritative while this topic is OPEN.

### 1. Exact rustc/liballoc and supported-target representation freeze

Evaluate whether the project can define and enforce an exact compiler/liballoc
source identity plus an explicit supported-target set, then prove the node
capacity, minimum occupancy, leaf/internal layouts, empty-root retention, and
all relevant mutation histories for each target. A CI toolchain pin alone is
insufficient: the restriction would need to be part of supported build
behavior, prevent an unsupported downstream build from entering the retained
boundary, and carry executable allocator-observed evidence. The compatibility,
maintenance, custom-toolchain, cross-compilation, and target-evolution costs
must be compared before this can be selected.

### 2. Project-owned observable retained-allocation map representation

Evaluate replacing or wrapping retained map ownership with a project-owned
representation whose allocated capacity and layout are observable through a
stable API. The investigation must cover every typed and nested serde_json Map
path, public type/package compatibility, ordering, deserialization and builder
entry, `no_std + alloc`, bounded conversion work, allocation peaks, and whether
the exact original Thing can still be retained. A private wrapper that cannot
receive all existing public values does not close the counterexample.

### 3. Change the exact-original or retained-source contract

Evaluate deliberately allowing normalization, rebuilding, or a different
retained source artifact so that opaque caller-created allocation history is
discarded before publication. This may make accounting observable, but changes
the current promise that `ValidatedThing::thing()` exposes the exact consumed
Thing and preserves its owned buffers. Required public API, ownership,
temporary-memory, pointer-identity, losslessness, and lifecycle changes must be
made explicit rather than inferred as implementation mechanics.

### 4. Private-layout-independent conservative accounting

Investigate whether a real upper bound can be proved using only stable public
contracts and observable state. A candidate must cover empty maps with distinct
allocation histories, arbitrary insert/remove histories, every reachable map,
all supported targets, and future supported liballoc implementations. Merely
raising the current constants, counting one node for every empty map, charging
by current length, or using `size_of::<BTreeMap<_, _>>()` does not by itself
prove such a bound. If no finite useful bound follows from public state, this
direction must be rejected explicitly rather than represented by a heuristic.

## Non-solutions

- Treating the Rust 1.95.0 workflow install as downstream representation
  authority.
- Reusing the serde_json outer-size guard as a BTree node-layout guard.
- Fixing only the current empty-root case while leaving future/private layout
  assumptions unowned.
- Declaring a larger per-node allowance without a falsifiable derivation.
- Treating passing content, serde feature, or non-allocator-observed tests as
  proof of retained allocation conservatism.
- Weakening evidence text while continuing to use the census as a physical
  retained-source admission ceiling.

## Required output before readmission

The investigation must select one coherent direction, record its trade-offs and
rejected alternatives, and migrate the stable conclusion into the registered
runtime-safety, WP-100, admission, compatibility, target, and evidence owners
that it affects. Completion criteria must include a reproducible version of the
empty retained-root counterexample and evidence capable of falsifying the
selected representation/accounting boundary. Only an independently accepted
docs-only correction may be followed by a separate admission-only transition.
