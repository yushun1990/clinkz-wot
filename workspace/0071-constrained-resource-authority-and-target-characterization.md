# 0071 Constrained Resource Authority and Target Characterization

Status: OPEN

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
