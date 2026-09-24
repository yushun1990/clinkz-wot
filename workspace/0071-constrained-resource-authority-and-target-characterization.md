# 0071 Constrained Resource Authority and Target Characterization

Status: MIGRATED — ownership correction projected; no readmission

Kind: architecture/resource-authority investigation

Baseline: `master` at `498cc9d263ed4257645ed65ad692657215b62016`.

Related: workspace topics 0069/0070, `docs/amendments/WP-100-bounded-atomic-number-v1.md`,
`PROFILE-AXIS-001`, `CONSTRAINED-PROGRESS-001`, and github-pr:88.

## Trigger

The WP-100 bounded-atomic Number work introduced a mandatory physical acceptance
boundary for one <=256-byte numeric projection on a declared 168 MHz Cortex-M4:
168,000 cycles and 4,096 additional stack bytes.

Implementing the corresponding STM32F407 evidence runner exposed disproportionate
validation complexity. The repeated failures were mostly in the evidence
machinery itself rather than in the Number projection: cancellation-interval
validation, probe-rs 0.32.0 profiling behavior, and debugger coverage
false-positive paths.

That cost is evidence that the project may be answering the wrong authority
question. `PROFILE-AXIS-001` defines four independent requirement axes:
compilation environment, execution model, resource profile, and capability
role; it separately states that a compilation environment does not imply a
hardware target. It does not itself prohibit a target-specific acceptance
gate. `CONSTRAINED-PROGRESS-001` already permits a non-incremental operation
when its admitted worst-case input is explicitly bounded and its complete
work/lifetime debit succeeds before execution. The narrower question here is
what additional invariant the Number-specific Cortex-M4 cycle/stack gate proves,
and why that extra evidence must be a prerequisite for the relevant Consumer
readmission rather than target/profile characterization.

This topic reopens that boundary. It does not assume that the current STM32
requirement is wrong, and it does not assume that it must be preserved merely
because substantial evidence work has already been invested in it.

## Questions

Determine, from current repository authority and realistic product roles:

1. What must core constrained authority guarantee independent of target hardware?
   In particular, distinguish semantic correctness, hard boundedness, work
   accounting, cancellation/progress guarantees, and resource-failure behavior.

2. Which limits belong to a resource profile rather than core authority?
   Review the role of limits such as Number lexical length, document size,
   schema-node count, forms, bindings, and related retained/temporary capacities.

3. Which claims are target or product characterization rather than generic
   admission requirements?
   Examples include wall-clock/cycle latency, additional stack depth, and
   target-specific performance margins.

4. Does the current project-wide 256-byte Number ceiling need to remain a
   project-wide atomic maximum, or should resource profiles own more of that
   choice? If a project-wide ceiling remains necessary, identify the invariant
   that requires it.

5. What additional property, beyond the already-frozen
   `CONSTRAINED-PROGRESS-001` atomic-operation rule, does the Number-specific
   physical measurement establish? Identify the concrete safety or admission
   invariant that the cycle/stack result proves and the generic bounded,
   completely precharged rule does not.

6. Why, if at all, must the current WP-100 Cortex-M4 168,000-cycle / 4,096-byte
   gate be a prerequisite for every relevant Consumer readmission rather than
   target/product characterization or profile-specific evidence? Do not infer
   either answer from `PROFILE-AXIS-001`; identify the actual dependency that
   would require the gate at that scope.

7. Which capability roles are realistically expected on small MCUs? Do not infer
   that `no_std + alloc` implies a full Consumer/Servient role. Evaluate
   Producer/server, Consumer, gateway, and other supported roles separately.
   Role applicability remains independent of resource-profile values. Preserve
   the existing thumb capability-on compile commitment while deciding which
   roles are realistic deployment choices.

8. What empirical measurements are actually required before correcting
   authority? Do not perform a broad STM32/ESP32/Linux survey by default.
   Identify only measurements that resolve a concrete authority uncertainty or
   calibrate a named resource profile.

9. What should happen to github-pr:88 under the resulting model: mandatory
   readmission evidence, target/profile characterization, a simplified benchmark
   fixture, or another disposition?

## Evaluation principle

Keep these layers separate unless repository evidence proves they must be
coupled:

1. **Core semantic and boundedness authority**
   - legal/invalid semantics;
   - hard finite resource boundaries where required;
   - bounded rejection;
   - complete work accounting;
   - progress and cancellation invariants.

2. **Capability-role applicability**
   - which Producer, Consumer, gateway, or other roles are supported or selected;
   - compile/support obligations for each role, including the existing thumb
     capability-on cell;
   - role selection is not inferred from compilation environment or resource
     profile.

3. **Resource-profile policy**
   - concrete capacity limits for an already selected role;
   - permitted zero capacities constrain resource availability within that role
     without redefining the role or feature-support contract;
   - deployment-specific resource contracts.

4. **Target/product characterization**
   - measured cycles or latency;
   - stack and memory margins;
   - target-specific implementation viability;
   - product real-time guarantees.

An empirical benchmark may justify a profile value or product claim without
automatically becoming generic semantic authority.

## Investigation scope

Audit the smallest relevant current authority, including:

- `docs/design.md` profile axes and supported cells;
- `docs/spec/runtime-safety.md`;
- `docs/spec/foundation.md`;
- WP-100 work-package and tranche authority;
- topics 0069/0070 and the bounded-atomic Number amendment;
- resource-schema/profile definitions;
- the intent and current cost of github-pr:88.

Look for similar target/profile coupling outside the Number case, but do not
turn this into a repository-wide redesign unless the same defect is actually
present elsewhere.

## Empirical evidence policy during this topic

Do not front-load broad hardware experiments.

First identify the authority decision and the unknowns that genuinely depend on
measurement. Then request the smallest experiment that can discriminate between
credible alternatives.

A simple manually inspected benchmark/demo is acceptable for characterization
when the question is engineering viability rather than formal admission
evidence. Stronger evidence is required only when the resulting authority makes
a correspondingly strong claim.

## Non-goals

This topic does not:

- implement production `ValidatedThing`;
- change Number semantics;
- readmit WP-100;
- select final resource-profile values in advance;
- declare STM32F407 unsupported as a WoT endpoint;
- declare any MCU incapable of a Servient role;
- continue hardening PR #88 before the authority question is resolved.

## Authority audit (master `bdf856e`, 2026-09-24)

This is an investigation result, not a change to the authorities cited below.
The audited master includes the Number feature-graph alignment from #90. The
reviewed PR #88 diff at `6a7b8df` and its description provide no accepted
physical-board results. Production `ValidatedThing` remains absent and
`WP-100-CONSUMER-VALIDATED-THING` remains `planned` / `candidate`.

The smallest relevant authority set is `docs/design.md` (`PROFILE-AXIS-001` and
`FEATURE-MATRIX-001`), `docs/spec/runtime-safety.md`
(`CONSTRAINED-PROGRESS-001`), `docs/spec/foundation.md` (`RES-LIMIT-001/002` and
`CONSTRAINED-WORK-001`), the WP-100 core and Consumer admission records, the
Number amendment, topics 0069/0070, `docs/resource-limits.csv`, and the
constrained workload README. `foundation/src/resource.rs` and
`td/src/components/data_schema.rs` were inspected to distinguish implemented
behavior from future requirements. The migrated topic 0047 decision already
separates canonical resource fields from executable role applicability and
profile authoring. A narrow scan of `docs/performance/constrained.toml` found
other target workloads; no broader resource redesign follows from this audit.

### What the present evidence proves

- The general constrained rule permits an atomic operation after an explicit
  admitted input bound and complete step/lifetime debit. It requires no
  universal millisecond or stack-byte constant. The Number amendment adds a
  **specific target tolerance**: 168,000 cycles (1 ms) and 4,096 additional
  stack bytes on one 168 MHz Cortex-M4. This tests one build's parser cost and
  stack margin. It does not prove Basic semantics, early rejection, budget
  behavior, a mathematical worst-case execution time, or viability of a whole
  Servient. The workload itself excludes Number parsing, allocation, and the
  caller's live state from the timed/stack interval, and says it is not an
  exhaustive WCET proof.
- The proposed causal chain behind the current gate is: 256 admitted bytes may
  reach a slow float-parser path; an uninterruptible path might exceed an
  application's tolerated cancellation delay or stack reserve; therefore
  measure it on M4. The first two statements justify a finite atomic boundary
  and target-specific characterization. No audited authority shows that **all**
  supported Consumer cells promise 1 ms latency on this M4 or a 4 KiB parser
  stack allowance. `PROFILE-AXIS-001` alone does not decide this; the missing
  dependency is a declared Consumer product/profile guarantee at that scope.
- `number_lexeme_bytes_max` is an append-only Foundation schema row, currently
  generated as Consumer-only metadata with 256/`NA`/64 named values. The raw
  `ResourceLimits` constructor validates only a separate response-index rule;
  the future role-bound builder and future `ValidatedThing` owner must enforce
  the Number maximum, early limit, and diagnostics. Existing
  `td/src/components/data_schema.rs` still treats a failed `as_f64()` as absent
  in the five extension predicates. The amendment specifies the deliberate
  future change, not existing production behavior.
- The M4 workload deliberately tests 256 bytes, while the named benchmark
  static reference profile allows 64. An application-defined Consumer profile
  could choose 256 under current authority, but no such complete M4 product
  profile is declared. The reference profile's configured
  `peak_live_bytes_per_admission_max` and `engine_live_bytes_global_max` are
  each 4 MiB; the workload board declares 128 KiB SRAM plus 64 KiB CCM. These
  ceilings are permissions, not an assertion that every admission uses 4 MiB,
  but the profile maxima cannot by themselves certify this board. Parser-only
  measurement therefore cannot establish complete resource feasibility.
- The reviewed PR #88 diff adds roughly 2,876 lines across a board runner,
  validator, tests, fixture, and CI. Its corrections to debugger and
  cancellation validation are useful evidence of measurement difficulty; host
  tests and CI cannot turn absent board measurements into a readmission result.
  Its README still frames the board result as mandatory architecture acceptance.

### Proposed ownership matrix

Each row gives the **current mandatory constraint**, the invariant it is meant
to prove, and its proposed primary owner. A row's current location in an
amendment does not determine its proper ownership. `L` means the selected,
validated per-Number profile limit.

| Current mandatory Number constraint | Invariant proved or intended | Proposed owner and disposition |
| --- | --- | --- |
| Only the five `Value::Number` Basic extension predicates use finite public `as_f64()` with binary64 rounding; failed/non-finite projection is `InvalidSchema`, non-Number is absent, `multipleOf` checks positivity only | One shared Basic result for applicable values, without silently losing an invalid bound | Core semantic/boundedness authority; retain |
| Typed `NumberSchema` remains current `f64`, typed `IntegerSchema` remains `i64`; unrelated Basic rules stay unchanged | No unintended semantic expansion from the extension-predicate change | Core semantic/boundedness authority; retain |
| Within-limit opaque `const`, `default`, and nested Numbers retain exact lexical content; typed equivalence compares that content | Storage/equivalence is lossless even when arithmetic projection is impossible | Core semantic/boundedness authority; retain |
| Public Basic, strict admission, and compatibility admission agree where the same typed Number reaches a predicate; bounded admission alone has `Pending`/`Limit` | Shared semantics with explicit admission resource terminals | Core semantic/boundedness authority; retain |
| Explicit `td/validated-thing -> serde_json/arbitrary_precision` for borrowed `Number::as_str()`; capability-off public Basic remains usable | Stable lexical access for the admitted Consumer surface, without making AP an arithmetic promise | Capability-role applicability; retain the capability and Host/thumb/downstream feature-cell commitment |
| Number admission row applies to Consumer `+validated-thing`; Directory client is `NA` | An inactive role does not acquire a fictitious validation owner | Capability-role applicability; retain, with executable role binding still owed under Foundation/topic 0047 |
| One finite, validated per-Number `L` exists before bounded admission; no missing policy means unbounded | Every atomic projection has a known maximum input and a complete possible debit | Core semantic/boundedness authority for the *requirement to bind a finite limit*; concrete `L` belongs to resource-profile policy |
| Project-wide `L <= 256`, including rejection of configurations above 256 | Currently caps the largest atomic input everywhere | Resource-profile policy for the numeric value; no independent core invariant found that singles out 256. A future authority change should require every selected profile/implementation to establish its admitted atomic bound and representable work/temporary capacity, or use a resumable path. It must not silently accept arbitrarily large `L` |
| Gateway 256, benchmark static reference 64, Directory `NA`; zero disables Number admission | Declared capacity or non-applicability for a selected role/profile | Resource-profile policy for 256/64/zero; capability-role applicability for Directory `NA`. Keep values provisional until owner enforcement and product claims are evidenced |
| Strict input stops at byte `L + 1` before copy or token-finish scan; typed AP path checks borrowed length before projection/copy | Over-limit input cannot cause unbounded scan, copy, or projection first | Core semantic/boundedness authority; retain parametrically in `L` |
| Over-limit Number gives structured `Limit`, not JSON or Basic invalidity; `L=0` rejects first Number byte but permits Number-free documents | Resource failure is early, exact, and distinct from semantic invalidity | Core semantic/boundedness authority; retain; zero is the profile choice |
| JSON lexing, lossless capture/copy, and other input-sized work remain charged and resumable | No hidden input-sized work before or around the atomic projection | Core semantic/boundedness authority; retain |
| Each projection predebits `n` `CodecInputBytes` from step and non-resettable lifetime work; step shortage is `Pending`, lifetime shortage `Limit`, zero budget makes no progress, repeats pay again | One complete atomic operation is accounted once per execution and cannot be funded by budget reset | Core semantic/boundedness authority; retain. `n` is an accounting quantum, not a cycle or stack proof |
| Cancellation checkpoints immediately before/after one projection; comparison afterward is scalar under a schema-node charge | No unchecked sequence of atomic projections or uncharged comparison work | Core semantic/boundedness authority; retain, with the atomic input bound established for the selected profile |
| 63/64/65, 255/256/257, `L-1/L/L+1`, overflow, rounding, long #81/#82 witnesses, and strict/typed parity tests | Falsify boundary and Basic implementation errors at chosen limits | Core semantic/boundedness authority for test *properties*; exact 64/256 examples follow resource-profile policy |
| M4 corpus over lengths 1..256 including slow fallback, 100 warmups, 1,000 samples, cold-first, no interval allocation, complementary stack paints, IRQ/cancellation trace, same-ELF fingerprints | Characterize the chosen build's parser path, observed cycles, stack, and cancellation under a declared target workload | Target/product characterization; retain only to support a named target/product claim or profile calibration, not generic admission |
| Every M4 projection <=168,000 cycles and <=4,096 extra stack bytes including callees, across supported compiler/dependency graphs; failure reopens the 256/projection choice | Intended 1 ms parser checkpoint and 4 KiB parser stack allowance on a declared board | Target/product characterization; remove as a universal WP-100 Consumer readmission prerequisite in a later authority migration. A failed target claim should revise that target's profile, algorithm, or promise rather than silently relax its limit |
| Readmission item 6 currently requires accepted M4 maxima and slow-fallback coverage; host run or thumb compile cannot substitute | Currently couples Consumer admission to that target tolerance | Target/product characterization for the physical part; core admission still needs semantic, budget, limit, cancellation, and supported-cell evidence. Move the physical dependency only after independent authority review; current item 6 remains in force until then |

### Role and profile consequences

A small MCU is a realistic **Producer/server** deployment when its binding,
Thing, handler, and Servient state fit an explicit resource profile. It may also
be a **Consumer** when it actually reads remote Things and can afford bounded
TD admission, planning, and live state. A **gateway** commonly combines roles
and may need substantially larger memory; a Directory **client** is separate
and has no `ValidatedThing` Number owner, while a Directory **service** is out
of scope. No role follows merely from `no_std + alloc`, the static reference
profile name, or the target CPU. The existing real-thumb capability-on compile
cell remains mandatory as portable API/feature evidence even if a particular
MCU product selects Producer only. Compile success makes no memory, latency,
binding, or deployment-feasibility claim.
Zero Number capacity constrains admission within a selected Consumer role; it
does not switch that role or its feature contract off.

Document bytes, schema nodes, forms, bindings, retained bytes, temporary bytes,
and work/lifetime limits remain explicit finite **resource-profile values** for
the roles to which they apply. Their core invariant is early admission,
accounting, bounded failure/cleanup, and no implicit unbounded omission; one
gateway or static-reference constant is not a universal MCU maximum. The
Number row follows the same pattern, with special attention to its atomic
projection. Topic 0047's existing executable-applicability and default-maturity
work is the appropriate follow-up for the broader schema; this audit found no
reason to reopen every resource value here.

### Proposed next disposition and remaining uncertainty

No new hardware measurement is prerequisite to deciding this ownership
boundary. A measurement becomes necessary only if a named MCU deployment
promises a cancellation-time or stack margin, or if a candidate profile value
needs calibration against that target. Then measure the smallest workload at
that profile's `L`, including the selected parser path and whole-product
resource context appropriate to the claim. The present 256-byte M4 workload
can remain an optional characterization reference, but it is not a proxy for
the 64-byte static profile or all Consumer deployments.

**Proposed PR #88 disposition:** do not merge it as mandatory WP-100 admission
evidence. Close it after the ownership decision is accepted, preserving the
branch/artifacts for a later, smaller target-specific benchmark if a named
product claim needs them. Do not continue runner hardening to satisfy a generic
gate. This topic does not alter or close the PR.

The remaining technical uncertainty is the profile-dependent atomic envelope:
what finite `L` each supported Consumer deployment can admit with its chosen
parser, work quantum, temporary memory, and tolerable cancellation interval.
That is an implementation/profile review question, not evidence that 256 or
the M4 1 ms/4 KiB margins are universal. A later authority migration must
resolve the project-wide 256 clause, the conflicting readmission wording, and
the named-profile maturity claim together; until it is accepted, the current
normative gate and `planned` / `candidate` state remain unchanged.

## Desired outcome

Converge on the smallest coherent authority correction, if any, that makes
resource limits, capability roles, hardware targets, and performance claims
explicitly owned by the correct layer.

The decision must state:

- which current WP-100 Number requirements remain core authority;
- which statements belong to capability-role applicability independently of
  profile values;
- which move to resource-profile policy;
- which become target/product characterization;
- whether any additional empirical evidence is prerequisite to migration;
- the disposition of PR #88;
- whether related resource limits need follow-up topics.

## Stop condition

Do not migrate authority until the investigation can explain why each retained
mandatory constraint belongs at its chosen layer.

If the only justification for a generic requirement is performance observed or
feared on one target class, keep the topic open and separate the target-specific
claim from the generic runtime invariant.

## Migration result

The accepted investigation was projected into the Number amendment, Foundation
and runtime-safety specifications, WP-100 core and Consumer admission records,
the registered tranche resource-change description, and the two Number fixture
READMEs. The historical 0070 record now points to this supersession. The
resource CSV and generated code retain the named 256/NA/64 values and stable
field identity; no profile value, production behavior, or feature cell changed.

- **Core authority:** the Consumer owner must bind a finite, implementation-
  supported per-Number `L` before bounded admission. It must prove a complete
  representable atomic step/lifetime debit and bounded temporary-resource
  envelope, reject an unsupported configuration, stop strict input at the
  first excess byte, distinguish resource `Limit` from Basic invalidity, and
  preserve lossless storage, shared binary64 predicate semantics, charged
  byte work, cancellation checkpoints, and bounded diagnostics/cleanup. The
  admission record retains its semantic, resource, progress, and Host/thumb/
  downstream feature evidence obligations.
- **Direct-entry boundary:** TD's sole opaque
  `ValidatedThingAdmissionConfig::try_from_limits` projection performs the
  implementation-dependent validation. Both direct admission constructors
  require its successful result and no longer accept raw `ResourceLimits`.
  The projection covers only fields consumed by `ValidatedThing` admission;
  complete Consumer role and execution-cell applicability remains with the
  resource-profile owner. Missing or unsupported admission values return
  `ValidatedThingConfigError` before allowance, ledger, input, or normalization
  state; readmission evidence must include benchmark success despite unrelated
  Consumer `NA` and a finite unsupported `L` whose rejection cannot become
  per-input `Limit`.
- **Profile policy:** 256 for gateway and 64 for the benchmark static reference
  remain provisional named Consumer capacities; Directory-client is `NA`
  because it has no validation owner. There is no project-wide 256 maximum.
  An application-defined value requires validation against its chosen
  implementation and complete resource policy; the schema values alone do
  not certify an MCU deployment. Zero constrains Number admission within a
  selected role without changing role or feature support.
- **Target/product characterization:** the M4 168,000-cycle / 4,096-byte
  tolerance and its physical workload are optional evidence for a named claim
  at a declared build and profile. They are no longer a generic WP-100
  readmission prerequisite. The workload README retains a reusable corpus and
  measurement procedure without asserting an accepted physical result.

No hardware measurement was needed to resolve ownership. The independent
resource-schema applicability/default-maturity work identified in topic 0047
remains follow-up work; this migration did not reopen other capacity values.
Readmission still requires separate independent acceptance of the eight
evidence items, including an implementation-supported atomic envelope and the
real thumb capability cell. PR #88 contains no accepted physical result and
its board-runner evidence is no longer a required WP-100 gate; it may be
closed separately without orphaning a required readmission item. Its branch
can remain a reference for a later target-specific claim.
