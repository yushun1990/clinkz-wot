# WP-200 Consumer compiler support admission candidate

Status: candidate only under ADR-0013, design revision v5.1. No production source
is admitted or completed by this record. Independent review must examine the
exact candidate head before a separate admission transition and source work.

## Scope and decision

`WP-200-CONSUMER-COMPILER-SUPPORT` prepares the narrow compiler precursor needed
by the paired Consumer aggregate. Its registered owner is WP-200, which already
owns Core's compiler/artifact SPI. The exact behavioral/API owner is
[Binding SPI: closed Consumer support](../spec/binding-spi.md#closed-consumer-compiler-support).
Planning owns its later use and whole-aggregate barriers; this record does not
repeat that specification.

The concrete problem includes trust as well as storage. Existing complete
registrations validate declarations, not arbitrary callbacks or actual compiler
configuration. Current Host erasure also allocates after consuming the native
cursor, when the existing failure shape cannot retain a completed payload.

The selected scope is one closed Core-owned resolved-target copy primitive,
immutable checked configuration, support attached to the complete registration,
and a supported Host adapter holding one fallibly acquired cursor/output slot.
The copy primitive performs no protocol or TD interpretation. It is deliberately
smaller than a compiler certification framework. Open native compilers retain
their existing SPI and narrower evidence, without becoming eligible for the
stronger aggregate by declaration.

The prior [product/runtime review](../reviews/review-07-product-runtime-architecture.md),
topics [0021](../../workspace/0021-host-constrained-authoring-constructibility.md),
[0036](../../workspace/0036-host-constrained-semantic-parity.md),
[0045](../../workspace/0045-host-constrained-parity-proof-and-evolution.md) and
[0076](../../workspace/0076-composable-capabilities-and-runtime-boundaries.md)
support separating semantic ownership from physical representation. They do not
establish a need for Runtime crates, images, a universal allocator or a new
native-extension trust model. A fresh read-only technical challenge also
identified the missing issuer/configuration boundary; it supplied advice, not
admission or acceptance.

Alternatives rejected at this boundary:

| Alternative | Reason |
| --- | --- |
| Generic public support token, unsafe attestation trait or compatibility allowlist | An arbitrary compiler/configuration can still make an unchecked assertion; private token fields do not repair its constructor. |
| Reserve logical bytes around today's Host adapter | Its infallible boxes cannot consume held backing; declaration does not reserve allocator storage. |
| Fallible boxing after Complete | The native cursor is gone and the existing Failed variant cannot retain the completed artifact. |
| Separate cursor and artifact boxes | Constructible, but the closed primitive can use one owner/slot with fewer acquisition and transfer points. Exact layouts, rather than assumed savings, govern completion. |
| Static-only aggregate or broad deployment redesign | Changes the existing paired claim or unrelated product boundaries before a concrete need. |

## Exact implementation envelope

Predecessors are the completed/current exact-coordinate
`WP-200-CONSUMER-PROPERTY-READ-PLANNING` and complete Consumer bundle
`WP-300-CONSUMER-PROPERTY-READ-BINDING`. WP-000 is the complete package dependency.
The TD tranche is disjoint: this precursor takes already resolved Core input and
does not consume or reinterpret a Thing or its validation proof.

Production paths are exactly:

- `core/src/binding_compiler.rs`: closed copy primitive, immutable configuration,
  private support issuance, fixed support descriptor and supported Host slot path;
- `core/src/binding.rs`: consuming support capture from existing complete Host and
  static registrations, with ownership-preserving rejection; and
- `core/src/lib.rs`: their necessary exports.

Focused completion tests may be added at
`core/tests/consumer_compiler_support.rs` and the existing external
[construction fixture](../../tools/architecture-fixtures/consumer-compiler-support/README.md).
No manifest, Foundation, TD, Planning, Servient, concrete protocol, workflow or
legacy API change is admitted. In particular, there is no allocator argument on
`BindingCompilerExtension`, no new operation, WorkClass, resource row, public
proof factory, callback/destructor extension or Producer lifecycle change.

The required API items and cells are registered in `docs/api-ownership.csv` and
`index.toml`. The compiler, artifact, descriptor and typed complete capture are
available in no-default, async-no-std and std. Supported Host construction and
erasure are std-only. The first supported primitive owns a 64-byte target buffer;
a checked capacity below that maximum is actual compiler configuration. This is
a supported subset, not support for every target allowed by a broad profile.
The fixture's forced start/step failure scenarios are test instruments and do
not add production configuration flags or arbitrary callbacks.

A callback receives only a short original resolved-input loan. Native cursor and
artifact hold no input pointer. Static capture checks the actual compiler type
with safe type identity and its actual private configuration. Host capture checks
the private supported adapter kind created by `try_new_consumer`; the ordinary
generic Host constructor confers no support, even for the same compiler type.
The whole registration is retained, and support cannot be paired with a second
registration. Fixed callback costs include compatibility if preparation uses it;
an immutable captured compatibility avoids that callback entirely.

The Core adapter's physical acquisition occurs before native start. Its actual
slot Layout is known from the support descriptor before allocation. Planning
later authorizes that Layout, outer owner and simultaneous temporary costs using
existing local/parent admission, then prepays fixed allocation/callback/release
work. The adapter retains physical backing through Pending/Failed/Complete.
Core does not own a parent/global governor or turn a logical reservation into
allocator backing. Startup registration storage remains a separately owned
baseline, not a per-coordinate hidden delta.

## Admission checks and review boundary

All named prechecks are executable now:

```sh
RUSTUP_TOOLCHAIN=1.95.0 tools/check-design-artifacts.sh
RUSTUP_TOOLCHAIN=1.95.0 cargo run --locked --offline --manifest-path tools/architecture-fixtures/consumer-compiler-support/Cargo.toml --bin witness
RUSTUP_TOOLCHAIN=1.95.0 cargo test --locked --offline --manifest-path tools/architecture-fixtures/consumer-compiler-support/Cargo.toml
RUSTUP_TOOLCHAIN=1.95.0 cargo test --locked --offline --manifest-path tools/architecture-fixtures/consumer-compiler-support/Cargo.toml --no-default-features
RUSTUP_TOOLCHAIN=1.95.0 cargo test --locked --offline --manifest-path tools/architecture-fixtures/consumer-compiler-support/Cargo.toml --no-default-features --features async
RUSTUP_TOOLCHAIN=1.95.0 cargo test --locked --offline -p clinkz-wot-core -p clinkz-wot-planning
```

The fixture library models the proposed Core issuer, and its separate downstream
binary/tests construct real complete Core registrations. Its positive Host result
is a **transport model over a complete typed registration**, not the existing
public Core Host adapter. A real complete Consumer-capable Host registration is
constructed and correctly rejected as unsupported. These are pre-code
constructibility discriminators. They neither complete the precursor nor prove
the proposed public Core names already exist.

Independent admission review must assess whether the closed primitive belongs
in Core, whether its limited usefulness justifies the additive surface, whether
the checked complete-owner construction is sufficient, and whether the retained
slot preserves the existing public projections. Review must distinguish those
direction judgments from the valid current counterexamples. The candidate cannot
be admitted merely because its tests pass. No independent acceptance is recorded.

## Required production completion

The stable key is `consumer-compiler-support`; its future result is
`docs/evidence/WP-200-consumer-compiler-support.toml`. It must use the admitted
public Core surface, not just the model, and cover:

- external successful typed and **public Host-erased** preparation through
  complete Consumer-capable registrations, followed by actual source/registration
  destruction and use of the concrete target and complete artifact identity;
- absent support, a same-compatibility foreign compiler, ordinary Host erasure,
  mismatched actual configuration, role/cell mismatch, every qualified identity
  mismatch and attempted support reuse, all before preparation callbacks;
- pre-bounds/compatibility/start/step/abort and destructor costs, fixed native
  storage, exact slot/owner Layouts and simultaneous native-output/slot overlap;
- failure at every new actual allocation point, including startup adapter and
  per-coordinate slot acquisition, retained other owners and unchanged rejection
  ownership; no native start on slot-acquisition failure;
- repeated zero/short Pending, paid Pending, Failed and fresh-budget retries
  without reboxing or refilling the coordinate remainder; exactly-once abort,
  output destruction and physical release;
- preserved public compatibility/type mismatch recovery and Consumer role
  metadata, including rejection of Producer-only route metadata; legacy Producer
  route metadata/projection behavior remains regression evidence;
- isolated no-default, async-no-std and std Core/Planning feature cells, existing
  compiler/registration/Consumer leaf tests and all registered Producer gate
  commands; and
- passing authority checks and normal locked workspace validation.

Target measurements and the new public Host positive belong to completion after
admission. Requiring them before writing the admitted source would recreate the
evidence recursion rejected by ADR-0013 and topic 0017. Conversely, the model
cannot be carried forward as if it measured the production implementation.

The aggregate stays unadmitted until this precursor is complete/current. It
still owes all-coordinate TD preflight, all bounds before start, variable output,
exact PlanFootprint and bounded rollback. WP-400 still owns parent/global pairing,
publication and terminal retention. Neither gate nor milestone status changes.
