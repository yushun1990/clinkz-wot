# 0086 Consumer Owned Artifact Configuration

Status: MIGRATED

## Resolution and independent-review boundary

The narrow correction is projected into the
[configuration/API owner](../docs/spec/consumer-resource-ownership.md#closed-consumer-binding-configuration),
[canonical vectors](../docs/resource-contracts/consumer-ownership-v2.toml),
[API ownership matrix](../docs/api-ownership.csv), and
[revised admission/impact record](../docs/work-packages/WP-400-consumer-resource-ownership-admission.md).
The existing resource validator checks the encoding; the production discriminator
retains the strict rejection witness and adds continuation/content counterexamples.
No production implementation changes in this amendment. Migration closes the
missing-contract/scope finding, not the production obligation. The linked
admission record and index own independent acceptance and implementation permission.

The [0081 evolution constraints](0081-reusable-plan-and-thing-identity.md#constraints-on-the-narrow-repair)
are explicitly reconciled in the admission record. That investigation remains
DISCUSSING; shared programs, instance operands and persistence are not selected
here. Binding eligibility is distinct from content and engine-policy identity,
without requiring per-Thing compiler ownership or changing generation authority.

## Original finding and boundary

The following reconstructs why the original admission could not proceed. Its
production evidence still describes the unchanged strict implementation; the
previous exclusion of compiler configuration is corrected by the linked revised
scope. The linked admission record owns the separate readmission transition.

The reconstruction baseline is
`959f27b93a9a880740fe6851563471096b24c10b`. The
[accepted successor contract](../docs/spec/consumer-resource-ownership.md#schema-applicability-and-identity)
requires the complete registration's binding digest to incorporate its resource
contract variant, concrete private policy and result bounds; compiler/artifact
compatibility uses that configuration. A strict artifact cannot become an owned
artifact merely because its numerical limits fit. The complete checked-policy
key is separately specified and cannot substitute for this binding digest.

The same specification says there is no compiler algorithm/API change, and the
[admission record](../docs/work-packages/WP-400-consumer-resource-ownership-admission.md#exact-sourcereadmission-boundary)
excludes TD/Planning algorithms and permits only the listed Core/Servient
production paths. `core/src/binding_compiler.rs` is outside those paths.
Changing the compiler configuration contract through a different file would
still change that excluded semantic boundary.

## Production evidence

The discriminator is
[`consumer_configuration.rs`](../planning/tests/consumer_configuration.rs).
It calls the public production Core types directly. It does not use the fixture's
included Planning source or experimental adapters. Its nondefault key is an
arbitrary distinguishable witness, not a proposed canonical binding encoding.

- `ResolvedTargetCompiler::configuration` in
  [`planning/src/binding_compiler.rs`](../planning/src/binding_compiler.rs) encodes only
  `[0x63, 0x74, 1, capacity]` followed by zeros. All 64 supported capacities are
  checked. There is no private-policy, resource-contract, result-bound or cell
  constructor operand.
- The native `check` requires the candidate configuration to equal that exact
  key. Both bounds and start succeed with the original key and reject a distinct
  key. A check bypass in complete-registration capture would therefore leave
  actual compilation blocked.
- Core's `static_consumer_support` in
  [`core/src/binding.rs`](../core/src/binding.rs) downcasts to that exact closed
  type and requires its configuration. A complete, compatibility-consistent
  Consumer-only registration with the distinguishing key is constructible, but
  its consuming capture rejects with cause 323 and returns the original owner.
- Host's private supported adapter stores the same closed compiler.
  `ConsumerCompilerSupport::configuration` reconstructs its capacity-only key;
  `host_consumer_support` requires it. The public Host discriminator observes
  the same capture rejection and bounds/start rejection. The original boxed
  client address survives capture rejection; no request callback runs, and
  exactly one client destructor runs when the recovered input is dropped.
- Positive complete static and public Host controls capture the existing strict
  compiler successfully. Existing rejection and ownership assertions are kept.
- [`planning/src/consumer_property_read.rs`](../planning/src/consumer_property_read.rs)
  seals `ConsumerPropertyReadRegistration`. Its production implementations read
  identity from the complete registration and directly invoke its compiler;
  materialization copies that configuration into the candidate and artifact
  identity. Servient cannot supply an independent adapter or relabel the
  immutable output through the existing admitted public boundary.

Thus using the required distinguishable binding key fails the only admitted
compiler path. Reusing the compiler key makes resource/private-policy/result
variants indistinguishable in the binding/artifact configuration. Runtime
contract checks alone cannot make the latter meet the accepted contract.

## Alternatives examined

| Alternative | Disposition |
| --- | --- |
| Add captured metadata and the complete engine policy key only in Core/Servient | Insufficient: engine policy identity is separate from the required binding/artifact identity. The frozen artifact still carries only the capacity key. |
| Change only the support check in `binding.rs` | Insufficient: native and supported Host bounds/start reject the new candidate configuration. |
| Choose another target capacity or binding ID/generation | Does not encode the required configuration. Target capacity is an actual compiler bound; binding ID/generation identify another owner, not the missing configuration operands. |
| Use a custom compiler or ordinary Host erasure | Outside the closed supported compiler contract. Static capture requires the exact source-owned type; generic Host erasure deliberately supplies no Consumer support. See the existing compiler-support rejection cases. |
| Translate configuration around each compiler call | Changes the compiler/artifact identity contract. Planning invokes the typed compiler directly, the cursor retains its candidate, and the public compiled output is immutable. This needs a reviewed projection, not a hidden Core/Servient workaround. |
| Extend the closed compiler to capture the required immutable binding configuration | A plausible bounded repair in the same successor tranche, but it changes excluded source/API semantics. Define the exact constructor/configuration/support/artifact projection and revise admission before implementing it. The linked configuration contract now selects this extension for independent readmission review. |

The falsifiable resolution boundary is one reviewed configuration projection
that preserves StrictV1, distinguishes the required owned binding operands,
captures the actual complete owner in both representations, compiles through the
real all-bounds path, and rejects strict/owned artifact substitution before
effects. It must also specify the private-policy input and the affected compiler
support/cursor/Host layouts. A new generic allocator, manager or task framework
is unnecessary.

## Transitive impact disposition

No production API, layout, numeric profile, accounting implementation or existing
test is changed by this finding. The 196-row table, schema overlay, six evidence
keys, workload manifests and budgets remain unchanged. Strict quantitative
evidence is not relabelled as an owned measurement.

| Owner / dependency | Disposition of this finding |
| --- | --- |
| WP-100 call values and borrowed TD | Existing strict semantic/input evidence remains valid; no source/API or layout change. |
| WP-200 planning-selection | Strict selected identities remain valid. A future binding-key repair must prove that the new configuration reaches candidate/artifact selection without weakening exact matching. |
| WP-200 compiler-support | Existing strict closed-type/configuration rejection is corroborated, not falsified. An extended compiler/configuration/support representation needs new native/Host and failure/layout evidence. |
| WP-200 aggregate and output-ownership | Existing strict output, all-bounds, source-free lifetime and rollback evidence remains valid. A compiler repair transitively affects aggregate identity and potentially cursor/output/bridge layout measurements; remeasure and review those changed records before treating them as current for the new revision. |
| WP-300 Consumer binding and Consumer-only registration | Existing strict complete capture and both seals remain valid. Owned registration/artifact configuration compatibility is unresolved; the successor cannot claim it. |
| WP-300 Host call construction | Existing strict recovery, Core-first allocation and postacceptance sealing remain valid. Future enlarged seal/error/call representations need their own measurements and fault injection. |
| WP-400 Consumer runtime | Existing strict parent provenance, funded calls/cleanup and residual-first disposal remain valid. New schema-2 accounting/disposal remains unimplemented. |
| Consumer/Producer gates and Foundation/portable dependents | Original verdicts remain scoped to strict source; there is no changed production layout requiring new measurements in this PR. A future shared layout change must put the affected completed records into impact review and reproduce transitive Consumer and applicable Producer evidence. ARM checks establish surface/layout only. |

At the finding revision, only the successor's implementation admission returned
to `candidate`; its implementation remains `planned`. No completed predecessor's original claim is
invalidated, so none is reopened or self-reaffirmed for a hypothetical changed
layout. Neither architecture gate verdict nor milestone status changes.

## Completion evidence boundary

| Registered key | Outstanding production obligation |
| --- | --- |
| `consumer-owned-identity` | The revised binding/artifact configuration is specified but unimplemented. This reproduction establishes the strict baseline; it does not pass the policy-key, contract/cell/role, limit, parent or exact-input matrix. |
| `consumer-owned-handoff` | Owned normal/late complete backing checks and accepted-overrun retention are unimplemented; no owned result is measured. |
| `consumer-owned-accounting` | No owned production provisioning/build/call/disposal exists to measure or fault-inject. Existing strict measurements remain strict. |
| `consumer-owned-pressure` | Existing admission prototype is not production evidence for independent roots, private pressure, reuse or last-user retirement. |
| `consumer-owned-disposal` | Existing controlled destructor prototype is not production Host disposal qualification or strict-policy exclusion. |
| `consumer-owned-regression` | Strict regression commands can pass without resolving the other five obligations. Successor composition still requires exact-revision production evidence and independent acceptance. |

No successor completion TOML is manufactured from the prechecks or this
discriminator. Verification results and exact tested revisions are reported at
the GitHub review boundary; the linked authorities own the correction and its independent readmission
boundary.
