# 0050 Tranche Completion and Successor-Entry Evidence

Status: MIGRATED

Kind: owner review question

## Observation

The Property Read progression recorded package-local completion for the narrow
WP-200 planning slice and the narrow WP-300 Producer binding slice. Preparation
of the WP-400 Servient slice later found that the implemented planner produced
only `BindingArtifactRole::ConsumerCall`, while the completed Producer binding
path required a reachable `ProducerRoute` artifact reference and a real
`PrepareInput` handoff.

## Questions

- What exact claim does package-local tranche completion make about the
  constructibility and reachability of its output at the next package boundary?
- What evidence is currently required before a completed tranche is allowed to
  release a dependent tranche?
- Which package or work package owns proof that a real upstream output reaches
  the first legal entry point of its declared successor?
- Did the WP-200 and WP-300 completion criteria intentionally exclude the
  Producer-route handoff, or did they assume that the handoff was already
  reachable?
- At which transition in the existing Property Read dependency chain was the
  missing `ProducerRoute` path expected to be detected?
- Can the current evidence model allow two adjacent package-local completion
  claims to remain valid while their intended production handoff is
  unreachable?
- Which authoritative artifacts distinguish package-local completion from
  eligibility to release downstream source work?
- Do any other completed or approved tranches rely on a successor handoff that
  has not yet been exercised with real upstream output and a real downstream
  entry point?

## Decision

Tranche completion is an exact package-local claim. It proves only the
requirements, paths, feature cells, exclusions, and evidence registered by the
tranche; it does not imply constructibility for every later role. A predecessor
regression check proves that the predecessor still works, not that a successor
can consume its output.

There are two release events. Package-local completion may release successor
candidate construction, fixtures, independent review, and exact pre-source
simulation. Successor source admission remains owned by the successor's
approved checkpoint. When the successor declares a cross-package handoff, its
entry evidence must carry a real upstream output into the first legal
downstream entry and may not synthesize the plan, artifact, identity, owned
object, or downstream input that the production boundary is meant to supply.

The first legal detection point for the current defect was therefore WP-400
candidate preparation, before source admission. That preparation correctly
failed: the implemented WP-200 algorithm emits only `ConsumerCall`, while the
completed WP-300 Producer registration consumes a `ProducerRoute` reference
through `PrepareInput`. The WP-200 and WP-300 local completion records both
explicitly exclude cross-package Property Read composition, so neither local
claim is revoked. WP-400 source remains blocked until Planning-owned correction
evidence carries the real output through the real WP-300 entry.

Inspection of the other completed exact tranches found no second released
source-admission boundary with a declared but synthetic successor handoff.
Their successor checks either exercise the reused public predecessor contract
or the dependency is sequencing rather than a declared runtime data handoff;
all broader package and architecture entries remain blocked. Future declared
handoffs are nevertheless subject to the same successor-entry rule.

## Rejected alternatives

- Treating local completion as end-to-end reachability is rejected because it
  would retroactively broaden explicit exclusions and make package-local
  evidence claim work it never executed.
- Allowing predecessor completion alone to grant successor source authority is
  rejected because it cannot detect role, identity, ownership, or construction
  gaps at the consumer boundary.
- Moving the proof into a fixture that creates its own plan, artifact, or
  `PrepareInput` is rejected because it hides the production handoff.
- Revoking both completed slices is rejected because the finding does not
  falsify either registered local contract.

## Migration

The completion/preparation/source-admission distinction and successor-entry
owner rule are projected into `AGENTS.md` and `PROJECT_GOVERNANCE.md`. `PLAN.md`
now projects the durable Property Read blocker without overstating release,
and `PROJECT_STATE.md` records the current correction boundary. This topic is
`MIGRATED`.
