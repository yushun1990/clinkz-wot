# 0067 Bounded Exact-Decimal Number Impact

Status: MIGRATED

Kind: ADR-0013 scoped implementation-impact review / TD semantic authority amendment

Evidence baseline: master `c7bce96` (github-pr:81 and github-pr:82).

Impact and authority amendment review location: github-pr:83.

## Finding and decision

The [Number boundary fixture](../tools/architecture-fixtures/validated-thing-number-boundary/README.md)
exposes two independent failures of the frozen stable-`Number::as_f64` route.
An arbitrarily long exponent is consumed synchronously before the public query
returns, even when the result is `Some(1.0)` or `Some(10.0)`. Decimal-cancellation
witnesses then show that an exact-value-only float replacement also changes the
current Basic result: legal spellings of mathematical one return `None` or
`Some(0.0)` from the observed query. Existing Basic validation can accept
those bounds where an exact comparison rejects them.

The selected direction is **exact finite decimal value** for the TD Basic
predicates that inspect `serde_json::Value::Number` in schema extension fields.
This deliberately revises their current float-projection acceptance; it does
not claim parity with the present `as_f64` implementation. The future public
`Thing::validate_with_level(Basic)` adapter and both `ValidatedThing` entries
must use the same TD-owned rule. The source Number remains lossless and typed
fieldwise equivalence remains separate from arithmetic equality.

This is a planned semantic contract, not current production behavior. The
tranche stays `planned` / `candidate` / `current`. The exact rule and progress
responsibility are owned by the amended [admission record](../docs/work-packages/WP-100-consumer-validated-thing-admission.md),
not repeated here.
No new ADR is needed for this scoped amendment: TD keeps the existing Basic
owner and public signatures, and Foundation, Planning, and Servient retain
their existing resource and lifecycle responsibilities. Independent review of
the semantic change is still required before readmission.

## Alternatives and impact boundary

| Alternative | Disposition |
| --- | --- |
| Keep `as_f64` and charge its containing node or precharge the whole query | Rejected: the stable call has no internal continuation and no bounded cancellation interval. |
| Preserve the observed float result with a lexeme-aware resumable clone of private parsing | Rejected: it would make rustc/serde implementation quirks product semantics across supported versions. |
| Fold to exact decimal, then produce a correctly rounded `f64` | Rejected: exact-value-only float projection fails the cancellation witnesses and still introduces rounding for Basic comparisons. |
| Compare exact decimal values in one resumable TD semantic kernel | Selected: it gives one version-independent meaning to legal finite decimal Numbers without an input-length cap or a second validation owner. Constructibility and resource proof remain required before readmission. |

The change affects the Basic result for numeric `minimum`, `exclusiveMinimum`,
`maximum`, `exclusiveMaximum`, and `multipleOf` extension fields, including
extensions on non-numeric schema variants that current validation inspects.
It does not change Number syntax, parsing acceptance, typed `NumberSchema` /
`IntegerSchema` field types or comparisons, JSON serialization, default
operations, URI/security rules, or the public method signatures and error
categories. Existing `Thing` behavior is the historical oracle for unaffected
rules, but no longer the oracle for these five predicates.

The affected requirement set remains `DOC-RUNTIME-001`, `ADMIT-MEM-001`,
`ADMIT-TXN-001`, `CONSTRAINED-WORK-001`, `CONSTRAINED-PROGRESS-001`, and
`CONSTRAINED-OWN-001`. The TD semantic owner already includes
`td/src/components/data_schema.rs` and `td/src/validate.rs` in the permitted
future paths. No Foundation method, WorkClass, resource row/account, allocation
category, state transition, Planning input, Servient owner, or public API item
is added. The three retained/four temporary arena catalog still owns number
bytes and fixed continuation state; whether that suffices must be falsified by
the source-level prototype and full resource proof, not assumed from this
document.

`docs/spec/runtime-safety.md` and `docs/work-packages/WP-100-core.md` receive
the cross-domain projection. `docs/work-packages/index.toml` registers this
current impact disposition while preserving `planned` / `candidate` /
`current`, the existing APIs, requirements, dependencies, paths, evidence key,
and separate readmission review. `docs/api-ownership.csv`,
`docs/resource-limits.csv`, Foundation's work/ledger contract,
`docs/state-machines.toml`, architecture 10/20/30/50, WP-200/WP-400, and
`PLAN.md` need no shape or roadmap change: TD still owns Basic, Planning still
consumes the same validated view, and Servient still owns admission/publication.
This is an impact finding, not a runtime reaffirmation of those artifacts.

## Evidence and stop condition

The #81/#82 fixture remains evidence against the **current** stable query
and against an exact-value-only float substitute; its Basic parity assertions
are historical counterexamples, not the future acceptance oracle. Its
restricted `UnitFold` proves only a narrow progress and cancellation witness.
The #79 RFC3339 finding and #80 date amendment remain independent.

All eight pre-readmission items remain required and incomplete as a set. In
particular, item 3 must now prove the deliberate Basic semantic delta and
shared-adapter parity, and items 2, 5, 6, and 7 must prove bounded Number
access, supported feature/target behavior, progress, cancellation, and
physical resource sufficiency. Item 8 still requires later reaffirmation of
prior evidence at the exact future source boundary. This review completes
none of them; it creates no completion evidence manifest and readmits no
tranche. The passed Producer Property Read gate and completed WP-200/WP-300
evidence are not changed or rerun by a docs migration.

If a construction requires a new public Number API, a production path outside
the permitted list, a new WorkClass/resource/account/allocation category, a
different Basic owner, or a changed lifecycle/Planning/Servient boundary,
stop and open another impact review before implementation. An inability to
prove exact decimal semantics for every supported graph is a readmission
blocker; silently falling back to `as_f64`, rejecting legal Number syntax, or
changing the supported graph is not authorized.
