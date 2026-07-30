# 0039 Candidate Fallback Availability Semantics

Status: OPEN

Kind: owner-raised runtime behavior and user-expectation investigation

Priority: MEDIUM

Target: the relationship between deterministic pre-execution fallback, multiple TD forms, runtime availability, explicit retry, and application-facing semantics

## Scope and authority

This topic records a Project Owner concern that the deliberately narrow fallback policy may differ from user expectations that multiple forms provide automatic runtime failover. It does not request fallback after side effects, security commit, acceptance, cancellation, deadlines, or transient execution failure. Codex owns the behavioral and API decision.

## Repository observations

- `PreExecution` fallback skips only side-effect-free security inapplicability or deterministic cacheable lazy-artifact failure.
- Backpressure, stale or draining generations, provider errors, deadlines, resource failure, request-construction failure, and binding input rejection terminate the interaction.
- Runtime health is diagnostic only and cannot reorder or remove candidates.
- Explicit retry remains governed by `RetryClass` and application policy.
- Producer route selection does not use the Consumer fallback policy.

## Questions for investigation

1. What behavior should users expect when a preferred HTTP form is temporarily unavailable but a Zenoh form is admitted?
2. Which failures are selection failures, execution failures, lifecycle failures, and caller-retry inputs?
3. Is the current application API sufficient to implement explicit multi-form retry without violating security or generation rules?
4. What diagnostics explain why another admitted form was not attempted?
5. Should a future immutable health-generation policy be planned, explicitly deferred, or excluded from v1?
6. How do Scripting-compatible behavior and Gateway defaults present this distinction?
7. Which tests prove that fallback never becomes implicit retry while still using valid ordered candidates where allowed?

## Constraints

- Do not fallback after protocol acceptance or committed security side effects.
- Do not let mutable health become an unversioned second planner.
- Preserve one work/probe allowance and bounded diagnostics.
- Distinguish documentation or facade ergonomics from changes to the frozen selection contract.

## Expected decision output

Codex should define the application-visible failover and retry semantics, diagnostics, examples, any facade support needed for explicit retry, and whether a future versioned health-aware policy requires a deferred design owner.