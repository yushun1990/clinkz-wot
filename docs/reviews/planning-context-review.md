# Review Comment – Clarify the Planning Snapshot Abstraction

## Observation

The current architecture diagram contains the following stage:

```
capture immutable policy and binding-registration snapshot
```

At the same time, the architecture defines **startup-only binding composition**
and explicitly lists dynamic binding composition as a **v1 non-goal**.

Under those assumptions, the Binding Registry appears to be immutable for the
lifetime of a Servient instance. This makes the meaning of
`binding-registration snapshot` somewhat unclear.

Questions that arise include:

- Is this intended to be a deep copy of the registry?
- Is it an immutable view/reference with an associated generation?
- Or is it simply the planner's immutable execution context?

The current wording may unintentionally suggest that runtime binding
registration is expected, even though the rest of the architecture points
toward a statically composed binding model.

## Suggestion

Consider raising the abstraction level and describing this stage as:

```
capture immutable Planning Context
```

The Planning Context could encapsulate everything required by the planner,
including:

- Binding Registry (typically as an immutable reference or generation-bound view)
- Policy snapshot
- Compiler Extension Registry
- Selection Strategy
- Other planning-time configuration

From the planner's perspective, the API becomes:

```
Planner::plan(td, planning_context)
```

rather than exposing individual implementation details such as registry
snapshots or policy snapshots.
