# 0087 Consumer Owned Compilation Cell Projection

Status: MIGRATED

The exact passive cell/support contract/API/source-scope amendment was
[independently accepted](https://github.com/yushun1990/clinkz-wot/pull/187#pullrequestreview-5544009375)
at `1afa7d335afee37d635f7269e31106174fb1c797`. Its detailed contract and separate
readmission/implementation boundary live in the linked owners below. This
migration closes the API/scope decision; it grants no tranche readmission,
predecessor reaffirmation or successor completion.

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

The accepted scoped correction lives in the
[specification amendment](../docs/spec/consumer-resource-ownership.md#compilation-cell-projection-amendment),
[two API targets](../docs/api-ownership.csv) and
[admission scope](../docs/work-packages/WP-400-consumer-resource-ownership-admission.md).
Those owners define the passive getters, captured singleton-cell semantics,
comparison before effects, exact source boundary and completion obligations.
Their independent scope acceptance does not grant tranche readmission.

## Rejected alternatives

| Alternative | Reason |
| --- | --- |
| Reconstruct a configuration inside Servient | The constructor still runs Core's feature-selected code, so this reproduces the captured Core cell rather than observing Servient's cell. |
| Probe or decode the digest | A hash is not an inspectable coordinate; probing also couples eligibility to private policy/result operands and cannot establish policy correspondence. |
| Let Core's cell silently redefine Servient's identity | Changes the accepted composition and complete-policy identity contract, including canonical portable keys. |
| Reject all owned portable registrations | Violates admitted matching portable composition; true portable downstream production paths already exist. |
| Expose the installed client or permit custom support | Adds operational authority and weakens the closed complete-owner proof unnecessarily. |
| Introduce a public cell enum or support mask | Adds a type or multi-cell contract where the existing three fixed coordinate spellings suffice. |

## Review boundary

The original PR #184 configuration amendment did not admit either new public
projection. The accepted correction changes neither the canonical encodings nor
which crate determines each captured cell. It selects passive observation of the
existing complete owner, preserving the original rule rather than weakening it.
The exact API/scope decision is closed; source readmission, implementation,
predecessor reaffirmation and successor completion remain separate ADR-0013
obligations. The successor remains `candidate` in its index, the production cell
negatives remain active, and gate verdicts do not change.
