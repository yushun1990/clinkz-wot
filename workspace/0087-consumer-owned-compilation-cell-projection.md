# 0087 Consumer Owned Compilation Cell Projection

Status: DISCUSSING

This is the bounded API amendment proposed for
[PR #187 P1](https://github.com/yushun1990/clinkz-wot/pull/187#discussion_r4290942708).
The [independent review](https://github.com/yushun1990/clinkz-wot/pull/187#pullrequestreview-5543094684)
accepts the finding at `230477221f0489db8867304745408df74fbf73fd`, but expressly
leaves the exact API and successor implementation acceptance open. This proposal
does not amend accepted authority or grant source admission. ADR-0013 requires
migration and independent scope review before implementing the new public API.

## Evidence and selected repair

The accepted [eligibility rule](../docs/spec/consumer-resource-ownership.md#schema-applicability-and-identity)
requires the registration's captured singleton cell to match the checked
Servient policy before provisioning effects. Cargo can unify Core/std while
Servient remains no-default or async-no-std. The existing production test
`owned_mixed_core_and_servient_cells_must_reject_before_clock_and_parent_mutation`
constructs that graph, captures the canonical Core/std configuration and complete
owner, then observes successful portable runtime provisioning and one clock read.
Both registered portable runtime commands fail. The
[successor evidence](../docs/evidence/WP-400-consumer-resource-ownership.toml)
owns the exact keys, commands and observations.

Core currently retains the configuration in its closed support descriptor, but
both the cell coordinate and that descriptor's configuration getter are private.
Select two passive projections of this existing metadata:

```rust
impl ConsumerBindingConfiguration {
    pub const fn compilation_cell(&self) -> &'static str;
}
impl ConsumerCompilerSupport {
    pub const fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration>;
}
```

`compilation_cell` returns exactly `"std"`, `"async-no-std"` or `"no-default"`.
Those are the already specified complete-policy coordinate spellings. Capture
the cell once in private checked configuration metadata, with the existing Core
feature precedence: std, then async, then no-default. The getter reads that
captured coordinate; it does not infer it from the digest or rerun construction.
There is no caller-supplied string, mutable field, default/unchecked constructor,
runtime override or multi-cell support mask. This avoids a new public enum and
any new identity encoding while providing the exact missing observation.

Promote the existing support `consumer_configuration` getter from crate-private
to public. It returns `None` for strict support and the exact captured `Some`
for owned support, after existing complete-owner/client/compiler checks. The
descriptor remains privately constructible and borrowed through
`ConsumerCompilerRegistration::support`. Reading or copying metadata grants no
client, compiler, callback, lease or attestation authority. Both getters perform
bounded, allocation-free work without clock, parent or operational callbacks.

Servient derives its expected cell from **its own** feature flags, using the same
precedence and the same private projection used to encode its complete policy
key. At the start of owned `ConsumerPropertyReadPolicy::qualify`, compare that
cell with the `compilation_cell()` of the `Some` configuration returned by
`r.support().consumer_configuration()`. Missing or
different owned metadata rejects with the original complete input and fixed
policy context, before clock/input observation, allocation, account mutation or
operational callbacks. Existing strict eligibility is unchanged.

## Rejected alternatives

| Alternative | Reason |
| --- | --- |
| Reconstruct a configuration inside Servient | The constructor still runs Core's feature-selected code, so this reproduces the captured Core cell rather than observing Servient's cell. |
| Probe or decode the digest | A hash is not an inspectable coordinate; probing also couples eligibility to private policy/result operands and cannot establish policy correspondence. |
| Let Core's cell silently redefine Servient's identity | Changes the accepted composition and complete-policy identity contract, including canonical portable keys. |
| Reject all owned portable registrations | Violates admitted matching portable composition; true portable downstream production paths already exist. |
| Expose the installed client or permit custom support | Adds operational authority and weakens the closed complete-owner proof unnecessarily. |
| Introduce a public cell enum or support mask | Adds a type or multi-cell contract where the existing three fixed coordinate spellings suffice. |

## Exact authority and implementation delta for review

Migrate the two signatures and their semantics into the existing Closed Consumer
binding configuration and Admitted API projection sections of
`docs/spec/consumer-resource-ownership.md`. Add only these two API-matrix rows,
with the same requirements/cells/operation as the existing configuration rows:

- `consumer_binding_configuration_compilation_cell`, public path
  `clinkz_wot_core::ConsumerBindingConfiguration::compilation_cell`.
- `consumer_compiler_support_consumer_configuration`, public path
  `clinkz_wot_core::ConsumerCompilerSupport::consumer_configuration`.

Amend the existing admission record and `api_changes` list in
`docs/work-packages/index.toml` to cover those projections. All implementation
paths are already in the successor's scope: Core `binding_compiler.rs`, existing
direct exports/tests, Servient's checked-policy path and existing admitted
Consumer fixture directories. No additional production package or API path is
needed. Keep the successor `candidate` until the migrated exact scope receives
independent review; this proposal itself does not restore admission.

The schema overlay, binding-key encoding, eight canonical vectors, complete
engine-policy keys, 64 strict keys, artifact compatibility/format, paid compiler
algorithm and TD/Planning public APIs remain fixed. No Foundation, shared program,
allocator, lifecycle, route, protocol or four future architecture-checkpoint work
is part of this amendment.

## Falsifiable completion and impact

Preserve the two existing mixed-cell negatives and make their actual production
constructors reject with zero clock/allocation/callback observations and unchanged
parent totals/peaks. Preserve exact complete-input/client recovery. Cover the
Host entry where a mismatched owner is constructible; an illegal Host disposal
contract must still reject earlier. Strict support projects `None`; owned native
and private Host support project the exact closed compiler/client configuration.

Reuse the existing Core compiler-support tests, Consumer runtime suite and
normal-library downstream runner. Matching portable positives must use an actual
matching dependency graph, already supplied by the runner; Servient's mixed dev
graph is the negative control, not portable-positive evidence. Relocate affected
owned positive cases within those admitted fixtures if needed; do not ignore the
negative or introduce a test-local runtime/compiler. Run all three registered
Consumer cells and both existing ARM compile cells. ARM is compilation/layout
evidence only.

Remeasure configuration/compiler/support/Host adapter, cursor/output/temporary,
aggregate/rollback/bridge/transfer and affected return/error layouts after the
source change, even when sizes remain equal. Preserve historical measurements.
The nine affected predecessor records already marked `review-required` retain
that disposition until independent evidence reaffirmation. The six successor
evidence keys, authority-check limitation and unchanged gate verdicts retain their
existing owners; neither this amendment nor green tests grants completion.
