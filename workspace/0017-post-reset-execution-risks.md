# 0017 Post-Reset Execution Risks

Status: OPEN

Kind: owner-raised execution-risk investigation

Priority: HIGH

Target: the transition from the current WP-200 admission-evidence correction to the first executable Property Read vertical slice and the remaining ClinkZ-WoT v1 path

## Scope and authority

This topic records Project Owner concerns about whether the current architecture,
governance, review, implementation, and release path will convert the accepted
v5.0 direction into executable product behavior at a credible rate.

The concerns are investigation inputs. They are not accepted findings,
predetermined technical conclusions, implementation instructions, governance
changes, or permission to bypass existing admission and review requirements.
Codex owns the repository-grounded technical judgment, decides whether each
risk is real, determines its priority and impact, and migrates any stable
conclusion to the proper authoritative owner.

## Repository context

The current repository records that:

- v5.0 bounded-core authority is active, while several global architecture gates
  remain open;
- multiple WP-100 narrow slices are implemented and evidenced;
- the WP-200 Property Read semantic candidate has independent v1 review
  evidence, but product source remains blocked behind a second correction of the
  admission-evidence boundary and a future v2 review;
- the intended Property Read vertical path remains ordered through WP-100,
  WP-200, WP-300, and WP-400;
- the planned Planning compiler/artifact boundary is not yet present in product
  Rust source;
- current Servient and Protocol Binding implementations still contain legacy
  execution and ownership boundaries that must be migrated;
- D9 introduces a bounded design-to-implementation conversion rule intended to
  prevent repeated non-implementation refinement from extending the executable
  critical path without an intersecting finding.

## Owner concerns

The Project Owner asks Codex to investigate the following risks without assuming
that any of them is already proven.

### 1. Repeated admission-evidence correction

The WP-200 semantic candidate passed independent review, but its planned
pre-source transition has already exposed two sequential evidence-boundary
problems: a stale carry-forward digest and a contradictory source-presence
projection.

Questions:

1. Do these failures represent two bounded defects in one otherwise sound
   admission model, or evidence that the admission model can recursively create
   new critical-path failures from its own state machinery?
2. What exact observable repository event would prove that the correction cycle
   is closed rather than merely advanced to another correction layer?
3. After the current second correction passes or fails, what existing rule
   determines whether another non-source candidate is justified?
4. Can a new evidence-truth finding be distinguished reliably from a consistency
   defect produced only by the interaction of status, registry, manifest, and
   checker artifacts?
5. Does the current D9 conversion rule actually constrain this sequence in an
   executable way, or can the same semantic boundary still generate another
   serial review cycle under a different artifact name?

### 2. Serial Property Read vertical path

The first executable cross-package proof still depends on the ordered sequence
from the completed WP-100 handler slice through WP-200 Planning, WP-300 Binding,
and WP-400 Servient work.

Questions:

1. Is this serial dependency chain the minimum required by the accepted
   architecture, or does the current work-package and gate structure add serial
   dependencies that are not technically necessary?
2. Which exact completion event currently represents executable vertical
   progress rather than local contract or authority progress?
3. Can unrelated package work consume repository effort while leaving the
   Property Read critical path unchanged, and how would Codex detect that from
   repository evidence?
4. Is the current Property Read slice narrow enough to reach an executable
   Planning-to-Binding-to-Servient proof without importing broad package scope?
5. Do the present milestone states make the distance to the first end-to-end
   interaction visible and unambiguous to the Project Owner and a fresh Codex
   session?

### 3. Target architecture versus legacy implementation

The accepted direction introduces compiler-owned plans and binding artifacts,
Servient-owned orchestration, and Protocol Bindings that do not select handlers
or reinterpret TDs at runtime. The current product source still reflects legacy
form selection, direct binding storage, and direct execution boundaries.

Questions:

1. What is the exact migration gap between the accepted target contracts and the
   current executable source?
2. Which legacy paths can coexist temporarily with the new WP-200, WP-300, and
   WP-400 slices without creating two authoritative execution models?
3. Could a temporary compatibility path preserve a hidden binding-to-handler
   dispatch shortcut or runtime TD interpretation after the new contracts are
   introduced?
4. Which ownership, generation, identity, cleanup, and rollback properties are
   most likely to be lost at the migration boundary?
5. What repository evidence would prove that the first vertical slice exercises
   the target architecture rather than a fixture-only or legacy-assisted path?
6. Does the current work-package decomposition identify removal and replacement
   ownership precisely enough to avoid an indefinite dual architecture?

### 4. Governance and checker complexity

Recent progress includes substantial work in specifications, work packages,
audits, registries, carried evidence, state projections, and executable design
checkers. Some of that machinery protects architecture and evidence truth, while
some of it may exist primarily to keep other process artifacts mutually
consistent.

Questions:

1. Which current checkers protect invariants that future product source or
   authority changes could actually violate?
2. Which current checks primarily validate consistency among process artifacts,
   and are those checks themselves on the executable critical path?
3. Has the cumulative governance and checker system become complex enough to
   create defects at a rate comparable to the product risks it is intended to
   control?
4. Can Codex identify duplicated validation truth across scripts, manifests,
   registries, audits, and design-check code?
5. Does every current critical-path checker have a stable owner, failure meaning,
   and removal or migration condition?
6. What repository-grounded measure distinguishes necessary risk control from a
   self-sustaining governance workload?
7. Has D9 changed actual commit sequencing and artifact composition, or only the
   language used to describe the intended process?

### 5. Host and constrained compiler/artifact model

D8 selects an associated-type portable compiler contract, host-side safe
 erasure, application-closed static compiler/cursor/artifact enums, owned
outputs without TD lifetime, and bounded artifact identity and footprint.

Questions:

1. Is the selected public contract realistically authorable by an independent
   third-party Protocol Binding crate in both host and `no_std + alloc`
   profiles?
2. How much application-owned enum and dispatch boilerplate is required when a
   constrained application composes more than one compiler?
3. Does safe host erasure preserve useful ownership and mismatch recovery
   without exposing Core internals or creating an impractical API?
4. Are cursor ownership, pending/failure recovery, and artifact footprint rules
   implementable without excessive copying, boxing, or public type complexity?
5. Do the existing authoring fixtures test realistic external use, or can they
   pass while depending on assumptions that a production binding author would
   not have?
6. What implementation or usage evidence would falsify the current D8
   constructibility claims?
7. Could host and static paths compile successfully while producing materially
   different semantics, diagnostics, resource behavior, or extension
   ergonomics?

### 6. Independence and effectiveness of technical review

The project uses independent root sessions and exact candidate attestations to
separate authoring from review.

Questions:

1. How much cognitive independence does a new root session provide when the
   author and reviewer use the same repository authority, similar model behavior,
   and the same formal candidate framing?
2. Are current reviews better at detecting path, parent, digest, schema, and
   fixture inconsistencies than API usability, runtime ownership, performance,
   or real binding integration problems?
3. Which classes of defect have the existing independent reviews actually found,
   and which important classes remain weakly tested?
4. Can a candidate satisfy every exact review check while still producing an
   awkward or unrealistic Protocol Binding or application-facing API?
5. What evidence should Codex use to judge whether review depth remains
   proportional to the risk and reversibility of each candidate?
6. Does the current review structure create correlated blind spots that are not
   visible through another root session alone?

### 7. Remote verification and mainline integrity

The repository records extensive local validation commands and exact review
attestations, while current GitHub commit status does not expose an equivalent
remote verification result for the latest mainline commit.

Questions:

1. Which verification guarantees currently depend on a Codex session executing
   local commands correctly rather than an independently enforced repository
   mechanism?
2. Can a direct mainline change bypass tests, feature-matrix checks, design
   checks, candidate checks, or diff hygiene without creating an immediately
   visible repository failure?
3. Is the absence of a reported GitHub commit status intentional, incomplete, or
   a risk to the evidence model?
4. Which checks require candidate-specific context and which are valid for every
   mainline change?
5. Does the repository have sufficient evidence that its supported feature
   matrix and default workspace baseline remain reproducible outside the author
   session?
6. How would Codex detect drift caused by toolchain, dependency, script, or
   environment changes when no active implementation tranche is being reviewed?

### 8. Remaining v1 scope and release-path credibility

The roadmap still contains Planning, Protocol Binding SPI, Servient runtime,
Directory/Discovery client runtime, Zenoh and zenoh-pico migration, umbrella
integration, final conformance, and release review work. Several global gates
also remain open.

Questions:

1. What is the exact remaining dependency graph from the current repository state
   to the v1 release target?
2. Which remaining milestones are required for the minimum declared v1 product,
   and which contain scope that could be independent of the first usable
   protocol-neutral runtime?
3. Are the current milestone boundaries small enough for Codex to produce
   executable progress regularly, or can each milestone reopen broad design and
   evidence work?
4. Does the current release target combine architecture closure, package
   completion, production binding migration, Directory client behavior, and
   final conformance in a way that obscures the minimum usable product boundary?
5. Which current risks threaten correctness, and which threaten only delivery
   throughput or visibility?
6. Can the repository state support a credible forecast of the remaining work
   without relying on percentage-complete claims?
7. What evidence would show that v1 scope is converging rather than continuing to
   reveal new required domains at each package entry?

## Cross-risk questions

1. Are these eight concerns independent, or do they arise from one smaller set of
   shared causes?
2. Which concerns intersect the current WP-200 Property Read admission boundary,
   and which are important but disjoint from its next source-changing event?
3. Does any concern reveal a project-goal, product, deployment, or usability
   question that requires Project Owner clarification rather than purely
   technical Codex judgment?
4. Which existing accepted decisions, specifications, work packages, audits,
   checks, or state claims would be invalidated if a concern is confirmed?
5. Which concerns are already controlled by existing repository rules and
   evidence, and what proves that control is effective?
6. Which concerns are not currently represented in the authoritative risk,
   work-package, milestone, or verification structure?
7. Does the current project state overstate architecture closure, package
   completion, executable integration, or release proximity in any way?
8. What exact repository evidence would allow Codex to close this topic as
   unsupported, partially supported, or supported?

## Required investigation discipline

- Do not treat the Project Owner concerns as proof.
- Do not assume that more implementation speed is preferable to correctness,
  portability, lifecycle safety, resource bounds, protocol neutrality, or
  evidence truth.
- Do not assume that existing governance, work-package, checker, or review
  structures are necessary merely because they already exist.
- Do not assume that non-source work is overhead merely because it does not
  change runtime behavior.
- Do not weaken, replace, preserve, or extend any process before determining its
  actual effect from repository evidence.
- Do not preselect a technical representation, migration strategy, review model,
  CI model, milestone decomposition, or release-scope change.
- Distinguish architecture/authority closure, package-local completion, and
  executable vertical integration.
- Distinguish risks that intersect the active WP-200 source boundary from
  important but disjoint project risks.
- Keep the investigation finite by identifying the evidence needed to decide
  each concern and the authoritative owner of any resulting conclusion.

## Expected decision output

Codex should determine from repository evidence:

1. whether each concern is unsupported, partially supported, or supported;
2. the exact mechanism and affected boundary for every supported concern;
3. whether the concern intersects the active executable critical path;
4. whether existing governance and evidence already control it adequately;
5. whether an authoritative architecture, governance, work-package,
   implementation, verification, milestone, or state change is required;
6. which conclusions can be decided together and which have distinct ownership,
   rollback, or validation truth;
7. the conditions under which this topic can move from `OPEN` to `DECIDED`, and
   from `DECIDED` to `MIGRATED`.

This topic deliberately supplies no preferred answer, remedy, implementation
order, artifact shape, review depth, process change, CI design, milestone split,
or release-scope decision. Codex must investigate and decide.
