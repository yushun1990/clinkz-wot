# 0061 Consumer One-Shot Domain Entry

Status: DECIDED
Kind: v5 domain-entry authority review
Baseline: `03e17e64c388370bd4ceb99f0bff42ba694076d7`

## Scope and authority

This topic decides the minimum authority required to enter the Consumer one-shot
domain and make the roadmap-required Consumer Property Read architecture proof
legally specifiable.

It is not itself implementation authority. It does not reactivate requirements,
change the active design revision, create a gate manifest, admit implementation
tranches, or modify Rust source. Those changes require a separately reviewed
authority migration.

The target remains deliberately narrow:

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
broad codec/schema work, observability breadth, and production Zenoh remain
outside this entry.

## Controlling findings

The decision is controlled by these repository facts.

- ADR-0018 intentionally left 34 v1 requirements inactive until their owning
  domains approach implementation; domain entry may re-adopt, replace, split,
  or retire them.
- The active v5 primary flow already requires Consumer selection from an
  immutable consumed plan set, construction of one selected `OutboundRequest`,
  selected binding execution, and shared response validation. No stage may
  rescan the TD to rediscover a decision already present in the plan.
- Current Planning already contains the beginning of a Consumer Property Read
  path: `PropertyReadPlanCompiler` internally defaults to
  `BindingArtifactRole::ConsumerCall`; only its Producer constructor is public.
  The existing algorithm emits an owned logical plan plus generation-bound
  artifact without requiring lazy/cache behavior.
- Current Consumer runtime is still legacy: `ConsumedThingHandle` scans the TD
  at call time, clones a raw Form, converts `InteractionOptions.data` into
  `InteractionInput`, and enters legacy `ConsumedThing`/`BindingRequest`.
- `InteractionOptions` is therefore implementation evidence, not a target
  contract. In particular, payload ownership is currently mixed into the
  options value even though the target interaction model treats payload/input
  and selection/control policy as different roles.
- `BindingResponseMetadata` already exists as fixed-size untrusted metadata.
  `API-PAYLOAD-001`, `BIND-IO-001`, and `BIND-DELIVERY-001` already own the
  output shape, provenance/identity ownership, and exactly-once delivery
  boundary. The historical broad response amendment is explicitly inactive.

## Decision table

| Identity / area | Disposition | v5.1 owner / reason |
| --- | --- | --- |
| `PLAN-REQUEST-001` | RE-ADOPT | `docs/spec/planning.md`; static execution facts stay behind generation-bearing plan slots and per-call state stays owned and bounded. |
| `BIND-OUT-001` | RE-ADOPT | `docs/spec/binding-spi.md`; `OutboundRequest` is the only selected Consumer execution envelope and contains no TD/Form/reselection authority. |
| `API-OPTIONS-001` | RE-ADOPT WITH NARROWED v5.1 DEFINITION | `docs/spec/interaction-core.md`; activate only the owned Consumer selection/control kernel needed by one-shot calls. Historical broad selectors remain non-authoritative. |
| Consumer Property Read response validation | ACTIVE OWNER SUFFICIENT | Narrow refinement of active `API-PAYLOAD-001`, `BIND-IO-001`, and `BIND-DELIVERY-001`, using the newly active request/binding identities. No `VALIDATE-*` identity enters. |
| `BIND-PROGRESS-001` | KEEP INACTIVE | Active one-shot call/cancellation/storage/delivery contracts already cover the first gate; its historical clause is subscription/general-progress broad. |
| advanced planning / codec / status / profile identities | KEEP INACTIVE | Not required to falsify the Consumer one-shot ownership topology. |

No fourth deferred identity is required for the first Consumer Property Read
architecture gate.

## PLAN-REQUEST-001 decision

Re-adopt the stable identity in the Planning owner.

The v5.1 active contract keeps the historical semantic kernel:

- static target/form/URI-template/schema/security/response/extension/artifact
  facts are referenced through immutable generation-bearing plan data;
- a per-call request owns only facts that genuinely vary for that call;
- no request copies a TD or raw Form and no transport execution path may
  reinterpret them; and
- the first Property Read slice uses an eager admitted artifact and does not
  imply `PLAN-LAZY-001`, `PLAN-CACHE-001`, `PLAN-INDEX-001`, or
  `PLAN-COST-002`.

The current private Consumer role in `PropertyReadPlanCompiler` is useful
implementation evidence that this boundary is constructible, but it does not
itself grant authority or freeze the later public constructor shape.

## BIND-OUT-001 decision

Re-adopt the stable identity in the Binding SPI owner.

The v5.1 active contract requires one owned `OutboundRequest` after selection
and security commitment. It carries the selected binding/plan/artifact identity
and call-varying execution facts required by that selected call. It carries no:

- TD or raw Form;
- credential provider;
- mutable application-options object;
- capability to ask another binding for support;
- authority to choose another candidate; or
- implicit fallback/retry-to-another-candidate permission.

A binding input rejection returns the complete request but does not re-enter
selection. The first Property Read slice uses one selected candidate and no
automatic fallback.

## API-OPTIONS-001 decision

Re-adopt the stable identity, but do **not** reactivate the historical broad
field set unchanged.

The active v5.1 definition is the small Consumer selection/control kernel:

1. `InteractionOptions` is an owned, non-exhaustive application input;
2. omission and explicit selection are distinguishable;
3. the first active one-shot baseline covers URI-template variables, optional
   explicit form selection, and call timeout/deadline intent needed by the
   admitted Consumer call;
4. every explicit selector may only narrow facts already represented in the
   immutable consumed plan set; it cannot trigger runtime TD/Form scanning or a
   binding capability probe;
5. operation payload is **not** an option. Application payload/input ownership
   remains the `InteractionInput`/operation-input role and is copied into the
   selected `OutboundRequest` only as call-varying data; and
6. handle-default/per-call merge breadth and explicit binding-id, media-type,
   subprotocol, security-branch, validation-profile, and other advanced
   selectors are not part of the v5.1 active definition. Adding any of those as
   required public semantics needs a later reviewed domain entry/revision.

The current public `InteractionOptions.data` field is therefore legacy migration
state, not v5.1 target semantics. The first Consumer target path must not depend
on it. Its physical removal is staged with the last legitimate legacy
write/action caller rather than forcing unrelated capability migration into
this gate.

This is a semantic narrowing of an inactive historical requirement, which is
permitted only because this domain-entry review produces a new design revision.
It must not be represented as an ordinary v5.0 wording edit.

## Consumer Property Read response-validation decision

No additional deferred validation identity enters.

The v5.1 authority migration should add one narrow Consumer Property Read
refinement to the existing Core/Binding owners:

- the binding returns a success as untrusted `InteractionOutput` plus
  `BindingResponseMetadata`, or a structured binding error;
- Core validates that the metadata binding id, binding generation, and plan id
  equal the live selected request;
- the first slice accepts only the compiled primary response branch;
- a successful Property Read has `InteractionStatus::Ok`, application payload
  role, no action invocation reference, and exactly one response payload;
- protocol-native status remains opaque provenance; Core does not reinterpret a
  numeric status as HTTP or another protocol;
- broad schema compilation, transcoding, additional-response tables, and
  validation-profile policy remain outside this gate; and
- only after this shared check may the `InteractionOutput` reach the
  application.

This refinement belongs to active `API-PAYLOAD-001`, `BIND-IO-001`, and
`BIND-DELIVERY-001` and consumes the newly active `PLAN-REQUEST-001` /
`BIND-OUT-001` identity boundary. `VALIDATE-COMPILE-001`,
`VALIDATE-REUSE-001`, and `API-CODEC-001` remain inactive.

The historical `validate_untrusted_binding_output` proposal is useful design
input but does not freeze its old public path or put Consumer semantic
validation in the Planning crate. Core remains the semantic owner.

## New design revision requirement

This domain entry changes the active requirement set and narrows a public API
contract. Under ADR-0018 and `CHANGE-CONTROL-001`, it therefore requires a new
design revision.

The authority migration target is **v5.1 Consumer one-shot authority**.

It must not silently mutate active v5.0 from 62 requirements to a larger set.
The reviewed v5.1 candidate and its activation checkpoint remain separate:

```text
active v5.0 (62)
 -> docs-only v5.1 authority candidate
 -> independent immutable-head acceptance
 -> separate v5.1 activation checkpoint
 -> Consumer gate/tranche admission
```

A concise new ADR should record the durable requirement dispositions and the
reason the historical options contract is narrowed; detailed behavior remains
in the registered specification owners rather than the ADR.

## Exact authority migration

After v5.1 activation, the classified set is:

- active requirements: **65**;
- `inactive-domain-entry-review-required`: **31**;
- classified total: **121**, unchanged.

The three identities moving from deferred to active are exactly:

- `PLAN-REQUEST-001`;
- `BIND-OUT-001`; and
- `API-OPTIONS-001`.

The authority candidate must update at least:

- `docs/ADRs/` with the Consumer one-shot domain-entry decision record;
- `docs/spec/interaction-core.md` to own active `API-OPTIONS-001` and the narrow
  Consumer Property Read output-validation refinement;
- `docs/spec/planning.md` to make `PLAN-REQUEST-001` active and remove its
  inactive label;
- `docs/spec/binding-spi.md` to make `BIND-OUT-001` active and remove its
  inactive label;
- `docs/spec/v5-authority-reset.toml` so the active/deferred counts and active
  source ownership project the v5.1 set;
- `docs/spec/README.md` navigation counts;
- `docs/requirements.csv`, moving `API-OPTIONS-001` source ownership from the
  residual design file to `docs/spec/interaction-core.md` while preserving the
  existing Planning/Binding owners for the other two identities;
- `docs/api-ownership.csv` only where the v5.1 semantic projection or staged
  legacy-removal disposition requires clarification; no ownership path change
  is implied merely by activation;
- WP-100/WP-200/WP-300/WP-400 Consumer Property Read entry/proof wording; and
- revision selectors/projections that must become v5.1 only at the activation
  checkpoint.

`docs/spec/v5-authority-reset.toml` should project the final active-source
counts as:

- `interaction-core.md`: 11;
- `planning.md`: 9;
- `binding-spi.md`: 12;
- all other active-source counts unchanged;
- total: 65.

No streaming, Directory, codec, observability, scheduling, lazy/cache/index, or
advanced-options identity changes classification in this revision.

## Gate authority that becomes legal after migration

Only after v5.1 authority activation may the repository register
`CONSUMER-PROPERTY-READ-ARCHITECTURE` and admit source tranches.

The gate must bind to the activated v5.1 revision and require at least:

- one consumed Property Read plan set using an eager Consumer artifact;
- one selected plan/binding generation;
- one owned `OutboundRequest` with no TD/Form/provider/options back-reference;
- one host call and one constrained/static slot representation;
- no call-time TD scan, raw Form selection, `supports_with_thing`, or
  `BindingRequest` target backflow;
- caller-drop, timeout, cancellation, late-result, and cleanup cases;
- narrow shared Consumer response validation;
- exact plan-lease/call/slot terminal release; and
- equal semantic outcome across host-erased and application-static cells.

The gate explicitly excludes fallback, lazy compilation, subscriptions,
collections, write/action semantics, broad validation/codecs, and production
Zenoh.

The gate-registration PR may extend the current singular integration-gate
registry only after this authority exists; this topic does not pre-register an
empty future manifest.

## Minimal implementation tranche sequence

After v5.1 activation, the implementation sequence is:

1. **WP-100 Consumer call-value slice** — target `InteractionOptions` semantic
   subset plus the narrow Core-owned Property Read Consumer output validator;
2. **WP-200 Consumer Property Read planning slice** — expose the bounded eager
   Consumer Property Read plan/selection projection and produce the selected
   generation-bearing request facts without lazy/cache/index breadth;
3. **WP-300 Consumer Property Read binding slice** — target `OutboundRequest`,
   selected `ClientBinding` host call/static slot, input rejection, settlement,
   and negative legacy-backflow checks;
4. **WP-400 Consumer Property Read Servient slice** — consumed plan-set
   publication, one application read call, call admission/lease ownership,
   caller drop/cancel/timeout/late-result handling, validation handoff, drain,
   and cleanup; and
5. **cross-package Consumer Property Read gate** — deterministic mock evidence
   in Host/static representations, followed only then by real Host Zenoh
   Consumer Property Read as WP-600 production evidence.

Each source tranche still requires independent ADR-0013 admission. Package
status alone does not authorize it.

## Explicitly deferred identities

The following remain inactive for this first gate:

- `PLAN-COST-002`, `PLAN-INDEX-001`, `PLAN-LAZY-001`, `PLAN-CACHE-001`;
- `BIND-PROGRESS-001`;
- `HANDLER-SUB-001`, `SUB-STORAGE-001`, `SUB-DATA-001`, `STATE-SUB-001`,
  `PRODUCER-EMIT-001`;
- `VALIDATE-COMPILE-001`, `VALIDATE-REUSE-001`, `API-CODEC-001`;
- `SEC-PERF-001`;
- `CAP-STATUS-001`, `OBS-PROFILE-001`, `RES-PROFILE-001`, `RES-LIMIT-004`;
- Directory-client execution identities; and
- the historical advanced `API-OPTIONS-001` selectors not included in the
  narrowed v5.1 definition.

## Rejected alternatives

### Re-adopt historical API-OPTIONS-001 unchanged

Rejected because it would drag binding-id/media/subprotocol/security/validation
selection and broad merge semantics into a gate whose purpose is to prove
Consumer call ownership.

### Keep API-OPTIONS-001 entirely inactive

Rejected because the target application-facing Consumer call still needs an
owned selection/control contract. Using the current legacy struct without
active semantics would make the gate depend on an implementation accident.

### Split API-OPTIONS-001 into multiple new requirement identities now

Rejected as unnecessary governance and API surface growth. The stable identity
can express the small v5.1 kernel; future advanced selectors can enter in a
later revision if actual capability work needs them.

### Activate broad validation requirements

Rejected because the first gate only needs live request/response identity and
Property Read success-shape validation. Codec/schema reuse is a different
problem.

### Remain on design revision v5.0

Rejected because the active set and public options semantics change. That would
violate the bounded reset and change-control rules.

## Non-goals

This decision does not:

- activate v5.1;
- implement or finalize every Rust field of `InteractionOptions`;
- require immediate removal of legacy payload/options fields used by unmigrated
  capabilities;
- implement `OutboundRequest` or `ClientBinding`;
- delete legacy `BindingRequest` / `ConsumedThing` paths before their target
  replacements exist;
- create or pass the Consumer gate;
- implement real Zenoh Consumer behavior; or
- enter the later long-lived subscription/emission domain.

## Migration condition

This topic remains `DECIDED` until one docs-only v5.1 authority candidate
migrates the three dispositions above into their registered owners and machine
projections, receives independent architecture-level acceptance at an immutable
head, and is activated through a separate mainline checkpoint.

Only after that activation may this topic become `MIGRATED` and the Consumer
Property Read gate/tranches be admitted.
