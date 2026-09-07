# 0064 serde_json Retained Representation Impact

Status: OPEN

Kind: ADR-0013 implementation-impact review and representation-authority
decision

Finding baseline: `14ececaf847e7eb68446813c5469c486d8cfb41f`

Review location: github-pr:71

## Question

What enforceable representation boundary lets the Consumer retain the exact
original public `Thing` while computing a conservative footprint for every
reachable owned buffer under legal downstream Cargo feature unification?

This question blocks readmission of `WP-100-CONSUMER-VALIDATED-THING`. It does
not reopen the Foundation-owned changes merged by github-pr:69, the passed
Producer Property Read architecture gate, or any WP-200 Step 2 work.

## Falsifying evidence

The rejected candidate models every `serde_json::Map<String, Value>` as a
content-sized BTree map and every `serde_json::Number` as allocation-free.
Neither assumption is stable across the public feature surface of the locked
`serde_json 1.0.149` dependency:

- downstream `preserve_order` feature unification selects a private IndexMap
  backing whose retained capacity can be increased with
  `Map::with_capacity`, but `Map` exposes no capacity accessor;
- downstream `arbitrary_precision` feature unification selects a private
  String-backed Number, including content and spare capacity not exposed by a
  capacity accessor.

An external reproducer using both features against the exact baseline reports
the same 9309-byte footprint for a compact one-entry Map and a one-entry Map
with capacity 16,384. It reports the same 8108-byte footprint for a one-digit
Number and a 16,384-digit Number. These equalities falsify the completion
claim that the census includes every reachable retained buffer.

The review also found bounded-progress defects that do not require a new work
class but must be included in any replacement admission evidence:

- explicit `form.op` membership is scanned before incremental
  `DocumentNodes` charging; and
- security expression roots and combo children are bulk-expanded before
  incremental `SecurityBranches` charging.

## Constraints that remain binding

- `ValidatedThingCursor`, `ValidatedThingStep`, and `ValidatedThing` public
  signatures and terminal ownership remain frozen unless impact review
  deliberately changes them.
- Success retains the exact original Thing; accounting may not clone,
  normalize, rebuild, sort, or shrink it implicitly.
- The census must cover actual retained representation, not serialized length
  or a `size_of` approximation.
- Basic semantic validation remains exactly
  `Thing::validate_with_level(ValidationLevel::Basic)` and runs only after a
  complete conservative census and atomic pre-charge.
- Existing schema, URI, and security work classes remain their sole charging
  owners. No PlanningItems or WP-200 Step 2 work enters this decision.
- `td/src/components/context.rs` has no authority beyond the single read-only
  `pub(crate)` Context storage seam accepted by github-pr:70.

## Directions requiring evaluation

1. Define and enforce a supported serde_json feature/representation surface.
   Merely setting `default-features = false` is insufficient because Cargo
   unifies features selected by downstream users of the same package.
2. Obtain a stable representation-inspection capability for Map and Number,
   whether upstream or through another deliberately governed dependency
   boundary. Any solution must preserve public type compatibility and avoid
   reliance on private layout or unsafe casts.
3. Change the public ingestion/ownership contract so retained allocation
   metadata is captured before it becomes opaque. This is a larger API and
   lifecycle change and cannot be inferred from the current admission.
4. Explicitly reject representations whose footprint cannot be proven. This
   is only viable if the restriction is enforceable, realistic for extension
   data, and deliberately specified as observable behavior.

No direction is selected here. The implementation candidate cannot choose
among them implicitly.

## Readmission boundary

Before implementation resumes, the selected conclusion must be migrated into
the authoritative dependency/API/resource owners and the admission record,
then independently reviewed. Pre-code checks and completion criteria must
include downstream feature-unified fixtures covering:

- compact and heavily over-capacity JSON maps with equal content;
- short, long, and spare-capacity arbitrary-precision Numbers;
- nested occurrences through every public Thing path that can retain Value;
- the already admitted Context entry and backing-Vec capacity cases; and
- step-by-step form-operation and security-expression fixtures proving no
  externally sized scan or enqueue occurs before its existing-class charge.
