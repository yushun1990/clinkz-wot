# 0020 Governance and Checker Complexity

Status: OPEN

Kind: owner-raised governance-risk investigation

Priority: HIGH

Target: the cumulative interaction among governance rules, registries, audits, manifests, continuation state, and executable checkers

## Scope and authority

This topic records a Project Owner concern about whether the repository's governance and checker system remains proportionate to the implementation risks it is intended to control, or whether the control system has developed independent complexity that can create new blockers and maintenance obligations.

The concern is an investigation input. It does not classify any existing artifact or checker as unnecessary, does not request weaker governance, and does not prescribe simplification. Codex owns the technical and governance judgment from repository evidence.

## Repository observations

The repository records that:

- architecture authority, project governance, planning state, continuation state, work-package records, audits, manifests, registries, and executable checkers each have distinct stated responsibilities;
- recent critical-path work has included substantial non-product changes to those artifacts;
- some checks protect runtime, ownership, lifecycle, authority, or evidence invariants;
- some checks validate consistency among other process artifacts;
- D9 states that continuity, registry, audit, and checker changes should travel with the checkpoint whose truth they record and should not become independent critical-path prerequisites;
- a new checker is intended to protect a stable invariant not already proved by an existing executable check.

## Owner concern

The Project Owner is concerned that individually justified governance artifacts may form a cumulative dependency system whose complexity approaches or exceeds the product change it controls. The concern includes whether process-consistency checks can recursively generate correction work without independently protecting implementation behavior or architecture authority.

## Questions for investigation

1. What stable invariant is protected by each governance, registry, audit, manifest, state, and checker artifact currently on the active critical path?
2. Which invariants can be violated by product implementation, architecture change, or authority migration?
3. Which checks primarily verify consistency among other process artifacts?
4. Are any invariants proved more than once by overlapping executable checks?
5. Are any artifacts authoritative owners of truth that is also independently restated elsewhere?
6. Can a process artifact change force updates across a broad graph even when product semantics, ownership, lifecycle, resources, and public APIs are unchanged?
7. Has any checker or registry rule become an independent source of critical-path failure rather than a detector of an underlying project defect?
8. Does D9's rule for attaching support artifacts to owning checkpoints operate effectively in current commits?
9. Can a fresh Codex session identify which artifacts are mandatory for a specific tranche without reconstructing the full governance graph?
10. Does the current system distinguish traceability value, authority protection, implementation safety, and status bookkeeping?
11. What evidence would show that the cumulative governance burden is proportionate and bounded?
12. If the concern is supported, which authoritative governance owner must record the decision?

## Constraints

- Do not use raw document, line, commit, or checker counts as sufficient evidence.
- Do not classify all non-code work as overhead.
- Do not classify an artifact as necessary merely because another artifact references it.
- Do not weaken architecture authority, independent review, rollback evidence, lifecycle safety, resource bounds, or durable continuation under this topic.
- Do not prescribe consolidation, deletion, quotas, or a new governance model before investigation.
- Preserve the AI-led model.

## Expected decision output

Codex should determine:

1. the invariant and authority map for the active governance/checker system;
2. whether duplicated, circular, recursively generated, or independently blocking obligations exist;
3. whether D9 currently bounds support-artifact work as intended;
4. whether any authoritative responsibility or progress claim requires correction;
5. the conditions for moving this topic through its workspace lifecycle.

## Historical decision

The concern was supported in a bounded form. The repository did not need a new
governance model, but it contained transition coupling capable of creating
critical-path failures without changing product semantics. The stale carried
digest, the pre-source source-presence rule, and the temporary dependence of a
review-pending candidate on moving `HEAD` were concrete examples.

The valid dependency direction was established as:

```text
authoritative contract/work-package state
  -> PLAN and PROJECT_STATE projections
  -> audit/attestation and registry evidence
  -> executable validation
```

The reverse direction is invalid. An audit, registry row, digest, or checker
cannot define technical truth solely because another support artifact points
to it. Exact digests still protect authority and evidence transitions, and
topology checks still protect candidate and implementation scope; those are
distinct invariants rather than duplicate owners.

The stable correction was to bind candidate identity independently of unrelated
later `HEAD` movement and to exercise the declared next state before
attestation. A support failure blocks only when it falsifies contract,
dependency, admission, completion, authority, or evidence truth. Otherwise it
travels with its owning checkpoint and cannot start another refinement cycle.
Deleting current authority, review, rollback, or resource controls would not
address the proven defect and was rejected.

## Historical migration

The directed responsibility model and transition rule were migrated into
`PROJECT_GOVERNANCE.md`; D13 and `PROJECT_STATE.md` recorded their execution
consequence. The then-current WP-200 transition had been exercised end to end,
so no additional checker or governance checkpoint was admitted on that
critical path.

## Reopened 2026-08-11: post-migration checker-growth counterexample

The Project Owner is reopening this topic because new evidence appeared after
the original migration. This does not revoke the historical directed
responsibility model. It asks whether that model bounded dependency direction
without sufficiently bounding the implementation structure and cumulative cost
of executable validation.

The relevant post-migration evidence is not the current total line count by
itself. Two consecutive, recent critical-path tranches continued to add large
amounts of tranche-specific checker machinery:

- between `30485b1a51470f328e79453ba0e82e3358c14f79` and
  `fcce9e69036459506a163ac73ef5542f92e5eb7f`, the route-reservation correction
  changed `tools/design-check/src/main.rs` by +924/-112 lines, added a dedicated
  144-line schema test, and added dedicated 195-line entry and 63-line
  completion shell checks while the principal product-source correction was
  comparatively narrow;
- between `fcce9e69036459506a163ac73ef5542f92e5eb7f` and
  `f72e494d6e6a229545f54fd00df3562b0067afcb`, the WP-400 Property Read
  Servient progression changed `tools/design-check/src/main.rs` by +975/-112
  lines, added a dedicated 369-line schema test, and added dedicated 392-line
  entry and 99-line completion shell checks alongside the substantial Servient
  implementation;
- the implementation commit `a993555f3cbd2bc7026423f34ed5620f3a2e058f`
  itself did not need to modify `tools/design-check/src/main.rs`, while the
  immediate completion checkpoint `031001d584689294ed7520dd3bc62cfe040227fd`
  subsequently added 274 and removed 6 lines there to record and validate the
  completed tranche topology.

These observations are evidence of continued per-tranche checker growth, not a
conclusion that the checks are unnecessary. They challenge the possibility
that the present checker size is merely historical residue whose growth has
already stopped.

### Reopened questions

13. Did the historical migration solve only support-artifact dependency
direction while leaving checker implementation complexity proportional to the
number of tranches?
14. Which of the recent route-reservation and WP-400 checker additions protect
new stable architecture/runtime invariants, and which encode instance-specific
candidate refs, path sets, transition topology, or completion bookkeeping?
15. For instance-specific transition facts that remain worth validating, can a
generic validation engine consume declarative work-package/evidence data rather
than requiring new tranche-specific Rust control flow?
16. Should adding a new tranche normally require new `design-check` Rust code,
or should Rust growth occur primarily when a genuinely new invariant category
is introduced?
17. Are the dedicated shell entry/completion checks and schema tests proving
independent defect classes, or are parts of them repeated projections of one
transition specification?
18. Can completed historical tranche-specific executable machinery be retired,
reduced to declarative evidence, or otherwise removed without weakening durable
architecture, rollback, admission, or completion proof?
19. What measurable evidence, other than raw line counts, would demonstrate
that checker complexity is now bounded as the number of future work packages
grows?

### Reopened constraints

- Do not infer unnecessary complexity from size alone; map recent additions to
  the stable invariants or transition truths they protect.
- Preserve strict ownership, lifecycle, resource, protocol-boundary,
  cross-package handoff, independent-review, and remote-integration evidence.
- Do not delete historical validation merely to reduce code volume.
- Prefer an architectural explanation of checker scaling behavior over an
  arbitrary line-count target.
- Treat the latest Property Read aggregate architecture gate as useful evidence
  for whether newer work reuses stable validation machinery or continues to
  generate tranche-specific executable control logic.
