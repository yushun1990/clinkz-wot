# clinkz-wot v5.1 Authority Manifest

Status: active v5.1 authority, independently reviewed at candidate commit
`3b133ebfe3c870102931982d6c056595f9d44255` and integrated through the ADR-0019 activation checkpoint.

This document is intentionally a small revision and source manifest. It does
not retain the v4.9 residual-domain monolith. The immutable historical basis is
Git commit `6c01e07a446f51d413618474554b5eedcf5de23e`; ADR-0018 established the bounded reset, ADR-0019 activates the
Consumer one-shot entry, and `docs/spec/v5-authority-reset.toml` gives every
one of its 121 requirement identities an exact disposition. Git history, not copied residual prose, is the
archive and rollback source.

## Normative language and identity

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**,
and **MAY** are interpreted as RFC 2119 and RFC 8174 terms only when uppercase.
A stable requirement identifier names a contract rather than a paragraph.
Only identifiers classified `active` by
`docs/spec/v5-authority-reset.toml` may authorize implementation or satisfy a
design or implementation gate.

The 31 `inactive-domain-entry-review-required` identities are mandatory input
when their named domain enters implementation. The 15 historical inputs, four
retired identities, and six redundant identities cannot authorize work. Their
text is recoverable from the reset basis. Mentioning an inactive identifier in
a plan, review, or historical specification does not reactivate it.

## Authority order

`ARTIFACT-AUTH-001`: Authority is resolved in this order:

1. `ARCHITECTURE_GOVERNANCE.md` controls technical convergence and design
   change;
2. accepted records in `docs/ADRs/` control decisions and supersession;
3. the registered architecture and specification sources listed in
   `docs/spec/v5-authority-reset.toml` own the 65 active requirements;
4. exact machine projections such as API ownership, resource limits, state
   models, work-package DAGs, and evidence records constrain their declared
   fields; and
5. source code and tests establish implementation truth, but cannot silently
   amend an active requirement.

`PLAN.md` owns roadmap and milestone state. The active task and pull request
carry the current engineering claim and acceptance boundary; repository,
GitHub, implementation, tests, and CI provide current state. Work packages
describe migration and technical acceptance; workspace topics retain
investigation history. None is an independent behavioral specification. When
active owners disagree, implementation of the affected scope stops until one
reviewed revision corrects every projection.

`IMPL-CONFORM-001`: Every implementation tranche MUST identify its active
requirements, exact files and feature cells, dependency-complete predecessors,
public API and state impact, resource impact, removal obligations, executable
pre-code checks, independent review, and completion evidence. A work-package
status by itself never authorizes source changes. Inactive identities may be
entry-review input but MUST NOT be used as implementation authority or gate
closure evidence.

`API-OWNERSHIP-001`: Every frozen public type, trait, operation slot,
registration, lifecycle record, and named profile MUST have exactly one
defining crate and public path in `docs/api-ownership.csv`. Higher crates may
re-export an item but MUST NOT redefine it. The matrix MUST remain compatible
with the dependency direction, feature cells, migration disposition, and active
requirements, and its checker MUST reject duplicate or placeholder ownership.

## Profiles and supported cells

Requirements use four independent axes: compilation environment, execution
model, resource profile, and capability role.

`PROFILE-AXIS-001`: A compilation environment does not imply a hardware target,
execution model, resource policy, or capability role. Constrained execution
MUST be usable under `no_std + alloc` and MAY be selected under `std`.
Protocol-neutral Directory client values may be portable even when one adapter
requires `std`. A Directory service remains outside the engine boundary.

`FEATURE-MATRIX-001`: The minimum supported cells are:

| Crate responsibility | `--no-default-features` | `async`, no `std` | `std` |
| --- | --- | --- | --- |
| Foundation | resource, work, time, generation values | same values; no executor | host conversions only |
| TD/TM | values, builders, Serde, validation, URI helpers | same surface | host conveniences |
| Core | interaction/plan values, local handlers, poll binding contracts | portable async handler twins plus poll contracts | host erasure and synchronization adapters |
| Planning | plan values and bounded compiler coordination | adapters without executor ownership | host compiler conveniences |
| Discovery | values and poll client contracts | async adapters over the same state | host client adapters |
| Servient | static values and manual progress | caller-driven adapters | builder, host handles, orchestration |
| Codec | bounded value and incremental contracts | same value contract | host I/O conveniences |
| Umbrella | useful constrained re-exports | portable async re-exports | selected host composition |

Features MUST be additive. `async` MUST NOT select an executor. Host-only
filesystem, socket, thread, process, dynamic-registration, and erased-object
conveniences remain behind their explicit features. A claimed no-default cell
MUST expose useful protocol-neutral values and progress contracts rather than
an empty crate.

## Standards baseline

The conformance baseline is:

- W3C WoT Architecture 1.1, Recommendation 05 December 2023;
- W3C WoT Thing Description 1.1, Recommendation 05 December 2023;
- W3C WoT Scripting API, Group Note 03 October 2023; and
- W3C WoT Discovery, Recommendation 05 December 2023.

`STD-BASELINE-001`: These dated publications, not moving latest or editor-draft
URLs, are the baseline. Published errata are adopted only through an explicit
impact review. Later publications and TD 2.0 features do not alter stable
behavior implicitly and remain behind an explicit feature and design revision.
Rust APIs preserve the Scripting API semantics while using owned values,
`Result`, traits, and Rust naming rather than copying WebIDL shapes.

## Frozen direction and execution order

The engine remains protocol-neutral. TD/TM owns documents; Foundation owns
lower-layer bounded primitives; Core owns protocol-neutral interaction and SPI
values; Planning owns plan construction; Servient owns application handles,
orchestration, lifecycle, scheduling, and cleanup; concrete bindings own
protocol adaptation and I/O; Discovery is a Directory client rather than a
service.

The coarse package-completion DAG is:

```text
WP-000 -> WP-100 -> WP-200 -> WP-300 -> {WP-400, WP-600} -> WP-700
                 \-> WP-500 ------------------------------/
```

`WP-500` depends on `WP-100`, not `WP-300`, because the Directory client owns a
Foundation/TD/Core state-machine boundary and does not consume Planning or
Protocol Binding execution semantics. This package DAG is a completion and
dependency projection, not a broad implementation start barrier.

ADR-0013 permits only an exact independently reviewed tranche to proceed while
broader packages or gates remain open. Cross-package executable feedback is
sequenced by materially different ownership topology rather than by broad
package completion. The passed Producer Property Read proof is the first such
proof. Before broad Consumer expansion, the Consumer one-shot domain must enter
authority and a narrow Consumer Property Read architecture proof must pass.
Before broad subscription/emission expansion, the streaming/emission domain must
enter authority and one minimal `ObserveProperty` long-lived architecture proof
must pass. The existing WP-400 multi-owner/multi-route checkpoint remains
WP-400 package evidence and may proceed in parallel with Consumer domain-entry
preparation; it is not another global gate.

Real concrete-protocol pressure follows a stable matching capability boundary.
Host Zenoh product evidence may proceed after the required WP-200/WP-300 and
WP-400 tranches plus applicable architecture proof are stable, without waiting
for unrelated broad WP-300 work. Real zenoh-pico runtime evidence remains
required before constrained parity or corresponding WP-600 completion claims.
None of this activates an inactive requirement: each Consumer, runtime,
streaming/emission, Directory, codec/validation/security, or advanced-planning
identity still requires its own domain-entry disposition before it can authorize
implementation.

## Revision and rollback

v5.0 replaces the v4.9 residual-decomposition strategy with 62 active
requirements and explicit dispositions for the other 59 identities. ADR-0014
and the D3 decomposition target are superseded for new authority construction.
Existing implementation and completion evidence are not changed by this
revision switch; their exact dispositions were checked before activation.

Candidate `b1916250a28ee133e8d0b12225c5b6311c975247` changed no Rust source,
Cargo manifest, public API, or runtime behavior. Independent attestation
`6d483a598e654f5c7043efb887074aba3a605f7a` reviewed its immutable 27-path
boundary; integration checkpoint
`30b845a4b17dd3eb56670da48c939b72daea7d59` has that candidate as its exact
second parent. Rollback returns mainline atomically to the pre-activation
checkpoint `6d483a598e654f5c7043efb887074aba3a605f7a` and invalidates any tranche
admitted solely by v5.0. Reset basis
`6c01e07a446f51d413618474554b5eedcf5de23e` remains the historical source for
the 121 inherited identities.
