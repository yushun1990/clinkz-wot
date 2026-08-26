# 0061 Consumer One-Shot Domain Entry

Status: DISCUSSING
Kind: v5 domain-entry authority review
Baseline: `16812e40431714365a410d51f33ac0846e272b52`

## Scope and authority

This topic reviews the minimum inactive v5 requirement identities needed to
enter the Consumer one-shot domain and make the roadmap-required Consumer
Property Read architecture proof legally specifiable.

It is not implementation authority. It does not reactivate any requirement,
change `docs/spec/v5-authority-reset.toml`, create a gate manifest, admit a
WP-100/WP-200/WP-300/WP-400 tranche, or modify Rust source.

The target is deliberately narrower than broad Consumer completion. The first
Consumer proof is one Property Read call through the target v5 ownership path:

```text
consumed plan-set publication
 -> selected immutable Consumer plan
 -> owned OutboundRequest
 -> selected ClientBinding call/slot
 -> protocol result
 -> shared Consumer response validation
 -> InteractionOutput
 -> caller / cancellation / late completion / cleanup
```

Fallback, lazy artifacts, subscriptions, collections, write/action breadth,
advanced validation/codec work, status/observability breadth, and broad
multi-binding fairness remain outside this entry unless an exact dependency is
proved necessary.

## Trigger

Workspace topic 0060 is decided and its roadmap conclusions have been migrated
through PR #43. The authoritative roadmap now requires Consumer one-shot domain
entry before a formal Consumer Property Read cross-package architecture gate.

The active v5 authority reset still classifies 34 identities as
`inactive-domain-entry-review-required`. The roadmap names three likely inputs
for this domain:

- `PLAN-REQUEST-001`;
- `BIND-OUT-001`; and
- `API-OPTIONS-001`.

This topic determines whether those identities should be re-adopted, replaced,
split, or remain inactive, and whether any additional inactive identity is
actually required by the minimal proof.

## Controlling active authority

The following active contracts already cover most of the Consumer proof and do
not need reactivation:

- `PLAN-SET-001` owns the consumed plan-set lifecycle and generation leases;
- `PLAN-ARTIFACT-001` owns immutable binding artifacts and compatibility;
- `PLAN-BOUND-001` owns bounded candidate examination;
- `BIND-REG-001` owns the complete startup binding registration;
- `BIND-STORAGE-001` owns constrained associated-state storage;
- `BIND-MEM-001` owns retained binding-call footprint admission;
- `BIND-DELIVERY-001` owns pre-acceptance input preservation and terminal
  delivery parity;
- `BIND-IO-001` owns binding I/O identity and transport metadata ownership;
- `BIND-CALL-CANCEL-001` and `BIND-HOST-CANCEL-001` own host call construction,
  cancellation, late completion, and cleanup settlement;
- `API-PAYLOAD-001` owns `InteractionOutput` payload/status/metadata shape;
- `API-SECURITY-001` owns protocol-neutral security semantics;
- `CLEANUP-RECORD-001`, `ERR-TAXONOMY-001`, `ERR-RETRY-001`, and
  `HANDLE-DROP-001` already own cleanup/error/drop behavior.

The active primary data flow already requires Consumer selection to occur
inside the admitted plan set, followed by construction of one
`OutboundRequest`, selected binding execution, and shared response validation.
No stage may rescan the TD to rediscover a decision already represented by the
plan.

## Historical inputs under review

### PLAN-REQUEST-001

The retained v4.9 clause says per-call requests reference static target, form,
URI-template, schema, security, response, extension, and artifact data through
generation-bearing plan slots while owning only call-varying payload,
URI-variable, cancellation, deadline, correlation, committed-security, and
protocol-status data.

This clause matches the current v5 data-flow direction and directly prevents a
Consumer call envelope from copying or reinterpreting the TD/Form tree.

**Candidate disposition: RE-ADOPT**, subject to confirming that its wording does
not accidentally require inactive lazy/cache/index behavior. The first
Consumer Property Read tranche would use eager admitted artifacts only.

### BIND-OUT-001

The retained v4.9 clause says `OutboundRequest` owns only selected binding/plan
identity plus per-call varying data; it contains no TD, raw Form, credential
provider, mutable application options, or authority to select another
candidate. A binding may not rescan the TD, weaken security, or perform implicit
fallback.

This is exactly the missing target boundary between Consumer selection and
client binding execution and is directly opposed to the current legacy
`BindingRequest`/`supports_with_thing` topology.

**Candidate disposition: RE-ADOPT**, with the first tranche restricted to one
selected Property Read candidate and no automatic fallback.

### API-OPTIONS-001

The retained v4.9 clause defines a broad owned `InteractionOptions` value with
explicit selection for form index, binding id, media type, subprotocol,
security branch, deadline, cancellation, URI variables, and validation profile,
plus deterministic handle-default/per-call merging.

The current implementation is substantially narrower: it exposes URI variables,
form index, request payload, and timeout. The Consumer Property Read proof needs
an explicit application selection input, but it does not by itself justify
freezing every historical broad option field or the complete merge contract.

**Candidate disposition: MODIFY / SPLIT — unresolved.**

The review must choose the smallest authority that proves Consumer selection
without forcing unrelated security-branch, validation-profile, media,
subprotocol, or broad handle-default API work into the first gate.

Acceptable outcomes include:

1. re-adopt `API-OPTIONS-001` unchanged if every field is already a necessary
   stable semantic input and implementing the complete value is not an
   accidental broadening;
2. replace the historical identity with a narrower Consumer one-shot options
   requirement and leave advanced options for a later domain entry; or
3. split the identity so the stable selection/URI/deadline/cancellation kernel
   enters now while advanced policy-selection fields remain inactive.

The review must not choose based on preserving the current Rust struct. Current
code is migration evidence, not authority.

## Additional inactive identities considered

### PLAN-INDEX-001 / PLAN-LAZY-001 / PLAN-CACHE-001 / PLAN-COST-002

Keep inactive for the first Consumer Property Read gate. The proof may use one
eager, already-admitted selected candidate and does not need lazy compilation,
single-flight cache behavior, advanced capability indexing, or compilation
policy variants.

### BIND-PROGRESS-001

Keep inactive. Its retained clause spans subscription and general pending
progress semantics. One-shot Host call ownership is already covered by active
`BIND-CALL-CANCEL-001`/`BIND-HOST-CANCEL-001`; constrained one-shot slot
behavior should first be checked against active `BIND-STORAGE-001`,
`BIND-DELIVERY-001`, `CONSTRAINED-PROGRESS-001`, and `CONSTRAINED-OWN-001`.
If that active set cannot specify the required Property Read slot terminal
retention, this topic must identify the exact missing clause rather than
reactivating the whole broad progress requirement by convenience.

### VALIDATE-COMPILE-001 / VALIDATE-REUSE-001 / API-CODEC-001

Keep inactive for the first proof unless the selected Consumer response path
cannot be validated using the already-active payload/response metadata and
binding I/O contracts. The first gate should validate identity, response branch,
status/payload role, and exactly-once result ownership without opening broad
schema compiler or codec reuse work.

### SEC-PERF-001

Keep inactive. The first proof may use the existing active security contract and
a deterministic NoSec or already-supported committed-security path. Security
performance is not required to prove Consumer call ownership.

### CAP-STATUS-001 / OBS-PROFILE-001 / RES-PROFILE-001 / RES-LIMIT-004

Keep inactive unless the exact call admission or cleanup proof demonstrates a
missing active resource/status owner. The gate should not become a broad runtime
observability/profile exercise.

## Consumer response boundary question

The Producer Property Read proof already established one Core-owned sealing
boundary for handler-origin success. Consumer response validation is a different
direction: the binding reports untrusted protocol/native response facts and the
engine validates them against the live selected request and compiled response
plan before application delivery.

The active architecture requires that shared validation step, while
`API-PAYLOAD-001` already owns `InteractionOutput` and fixed-size binding-response
metadata and `BIND-IO-001` owns binding I/O identity/provenance.

This review must decide whether the minimal Consumer Property Read validator can
be specified as a narrow refinement of those active requirements or whether one
inactive validation identity must enter. It must not activate broad codec/schema
validation merely because the word "validation" appears in the flow.

## Minimum gate authority target

A successful domain-entry decision must be sufficient to specify all of these
facts before any Consumer source tranche is admitted:

1. `consume` publishes one immutable consumed plan-set generation before calls;
2. one application operation selects only within that plan set;
3. selection never scans the TD or asks a binding whether it supports a raw
   Form at call time;
4. one generation-bearing plan/binding choice becomes one owned
   `OutboundRequest`;
5. the request contains no TD/Form/provider/mutable-options authority;
6. one client call or constrained slot is admitted before protocol side effects;
7. caller drop, timeout, cancellation, late completion, and cleanup retain one
   owner and one terminal classification;
8. untrusted binding response facts are checked against the live request and
   compiled plan before `InteractionOutput` reaches the application;
9. terminal completion releases the call/slot and plan lease exactly once; and
10. Host-erased and constrained/static representations preserve the same
    semantic outcome while exposing profile-appropriate storage/progress.

## Explicit first-gate exclusions

The authority packet produced from this topic must explicitly exclude:

- automatic candidate fallback;
- lazy artifact compilation and cache single-flight;
- subscriptions and `ObserveProperty`;
- collection operations;
- write/action calls;
- retries that select another candidate;
- broad media negotiation or transcoding;
- broad schema/codec compiler reuse;
- runtime binding mutation;
- multi-binding fairness/performance closure; and
- production Zenoh implementation itself.

Real Host Zenoh Consumer Property Read follows the architecture proof as WP-600
production evidence; it is not used to define the shared Consumer authority.

## Required decision output

Before this topic becomes `DECIDED`, record an exact disposition table:

| Identity / area | Disposition | New active owner or reason to remain inactive |
| --- | --- | --- |
| `PLAN-REQUEST-001` | RE-ADOPT / REPLACE / SPLIT / KEEP INACTIVE | ... |
| `BIND-OUT-001` | RE-ADOPT / REPLACE / SPLIT / KEEP INACTIVE | ... |
| `API-OPTIONS-001` | RE-ADOPT / REPLACE / SPLIT / KEEP INACTIVE | ... |
| Consumer response validation | ACTIVE OWNER SUFFICIENT / ADD EXACT ID | ... |
| Any additional deferred identity | ... | ... |

The decision must also name:

- exact normative files to update;
- exact `v5-authority-reset.toml` disposition/count changes;
- work-package and API-ownership projections affected;
- the Consumer Property Read gate manifest fields that become legal only after
  authority migration; and
- the minimal implementation tranche sequence across WP-100/WP-200/WP-300/
  WP-400.

## Non-goals

This topic does not:

- design the final broad `InteractionOptions` API unless the entry review proves
  it is necessary;
- implement `OutboundRequest` or `ClientBinding`;
- delete legacy `BindingRequest` or `ConsumedThing` paths;
- create the Consumer Property Read gate manifest;
- modify active requirement counts before a disposition is decided;
- implement or claim real Zenoh Consumer progress; or
- start the later long-lived subscription/emission domain.

## Migration condition

This topic may become `DECIDED` after the inactive identities above have exact,
non-conflicting technical dispositions. It becomes `MIGRATED` only after those
dispositions are integrated into their registered normative owners and machine
projections through independent review. Only then may the Consumer Property Read
gate and source tranches be admitted.
