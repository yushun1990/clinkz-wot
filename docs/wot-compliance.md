# W3C WoT Compliance Notes

## Baseline

The default compliance target is W3C Web of Things TD 1.1, Architecture 1.1, Discovery, and Profile.

TD 2.0 is tracked as experimental work only. Do not make TD 2.0 behavior part
of the default public API until the specification stabilizes.

## TD 2.0 Feature Gate

TD 2.0 vocabulary that is not part of TD 1.1 is gated behind the
`td2-preview` feature and is absent from default builds. The gated surface is
limited to:

- The `ActionAffordance.synchronous` field and its builder method. When the
  feature is disabled the term round-trips as an opaque extension field.

The full TD 1.1 `op` vocabulary is always available in default builds. This
includes `cancelaction`, `subscribeallevents`, and `unsubscribeallevents`,
which are part of the TD 1.1 REC `op` vocabulary (§5.3.4.2). Their runtime
behavior is likewise always available: the `ActionCancelHandler` trait,
`ExposedThingHandle::set_action_cancel_handler`, the
`ConsumedThingHandle::subscribe_all_events` / `unsubscribe_all_events` methods
(sync and async), and the matching inbound dispatch and zenoh binding planning
arms.

Consequences:

- A default build targets full TD 1.1, including every TD 1.1 `op` value, so a
  conformant TD 1.1 document (e.g. the official WoT parsing fixtures) parses,
  validates, and round-trips without enabling `td2-preview`.
- Enabling `td2-preview` on `clinkz-wot-td` additionally surfaces the
  `ActionAffordance.synchronous` data-model field for experimental TD 2.0
  tracking.

## Thing Description

A Thing Description is the machine-readable contract for a Thing. It describes identity, metadata, interaction affordances, security metadata, protocol forms, links, schemas, and extension vocabulary.

TD parsing should be tolerant enough to preserve unknown terms. Validation should be a separate explicit step.

## Thing Model

Thing Model should be used to reduce authoring repetition and define reusable templates.

The intended workflow is:

1. Authors define reusable TM templates.
2. Platform tooling instantiates concrete TDs from those templates.
3. Consumers receive fully usable TDs with concrete forms.

This keeps TDs precise for consumers while avoiding repetitive hand-authored JSON.

## Forms and `base`

In TD, a form describes how to perform an operation. A form target is provided by `href`.

Using a Thing-level `base` reduces repetition:

```json
{
  "base": "zenoh://clinkz/gateways/gw001/",
  "properties": {
    "temperature": {
      "type": "number",
      "forms": [
        {
          "href": "properties/temperature",
          "op": ["readproperty", "observeproperty"]
        }
      ]
    }
  }
}
```

The runtime or binding layer should resolve the target as:

```text
zenoh://clinkz/gateways/gw001/properties/temperature
```

The TD crate should store both `base` and `href`. Shared resolution logic should be added so bindings do not each implement their own resolution behavior.

## URI Templates

URI templates are useful when one affordance exposes parameterized resources.

For fully enumerated properties, each property should still have an explicit form, but the form can use a short relative `href` and default metadata.

## Defaults

TD authors and generators should avoid redundant fields when defaults are clear.

Examples:

- `contentType` defaults to `application/json`.
- Thing-level `security` can be inherited by forms unless a form overrides it.
- Default operation behavior is resolved according to TD rules and interaction
  type. Per TD 1.1 §5.4 the default `op` for an omitted Event form is
  `subscribeevent` and `unsubscribeevent`; for a Property form it depends on the
  `readOnly`/`writeOnly` flags, and for an Action form it is `invokeaction`. A
  `readOnly` and `writeOnly` combination of both `true` is rejected.

## Context Ordering

When a TD carries multiple `@context` values, the standard WoT TD context URI
must be the first entry so JSON-LD processors resolve the WoT vocabulary before
any extension namespace. Profile/Full validation enforces this; extension-only
first entries are rejected.

## JSON-LD Contexts

The `@context` field defines the semantic vocabulary used by a TD.

The standard WoT TD context should always be present. Extension vocabularies should be explicitly declared with their own namespace prefixes.

## Scripting API Boundary

> Aligned to v4.0 (`docs/baseline/engine-architecture-baseline.md` §0). The
> earlier "Native WoT Runtime, Scripting API as design reference only"
> positioning is **reversed**.

`clinkz-wot` targets **WoT Scripting API Consumer/Producer/Discovery User Agent
conformance** as a first-class goal (not merely a design reference). The
`WoT` facade, `ExposedThing`, `ConsumedThing`, and `ThingDiscovery` surfaces
follow the Scripting API method catalogue (see the conformance map in v4.0
§10). Rust idiom (`Result` instead of throw, `impl Future` instead of Promise,
owned buffers) is the *syntax*; the *method set, parameter semantics, and error
model* follow the Scripting API.

Consequences for this engine:

- The compliance bar for a Thing is a conformant TD, the protocol behavior
  declared by its forms, **and** faithful Scripting API interaction semantics.
- **Engineering-priorities posture (v4.0 §0):** Scripting API alignment is a
  *target*, not a constraint overriding engineering judgment. Performance,
  stability, extensibility, and code reasonableness are the primary criteria;
  where strict Scripting API adherence conflicts with these, the
  engineering-best choice wins and the divergence is recorded as a §9
  deviation. The §10 map is the default surface, amended by §9.
- **Naming posture:** conformance is method-catalogue + parameter-semantics +
  error-model alignment in Rust idiom, **not** verbatim JS type/method naming.
  The Servient UA surface (the `WoT`/`Servient` facade, the
  `ExposedThingHandle`/`ConsumedThingHandle`/`ThingDiscoveryProcess` handles,
  and the `snake_case` method catalogue — `read_property`, `set_*_handler`,
  `emit_event`, etc.) is the Scripting-API-aligned layer (v4.0 §10 map). The
  engine-internal concrete types (`ExposedThing`, `ConsumedThing`,
  `InteractionInput`/`Output`, `EventBroker`, `PushFn`, …) are Rust-idiomatic
  engine types: `ExposedThing`/`ConsumedThing` denote the concrete thing STATE
  in core, wrapped by the Servient's `ExposedThingHandle`/`ConsumedThingHandle`
  (the app-facing surface that maps 1:1 to the Scripting API's
  `ExposedThing`/`ConsumedThing`). Such naming/idiom choices are governed by
  v4.0 §0 and are not Scripting-API deviations.
- Engine-specific **behavioral** deviations from the Scripting API surface are
  documented explicitly (v4.0 §9), not hidden. The current documented
  deviations are:
  - **Subscription delivery is a pull-queue** (`Subscription` drained by
    `poll_next`/`Stream`), not a push callback — required for `no_std + alloc`
    safety on a bare MCU (reentrancy / super-loop blocking).
  - **Errors are `Result`, not thrown exceptions** (Rust idiom).
  - **`fetchTD` / directory exploration are trait objects (`Discoverer`)**, not
    a built-in `fetch` — the engine is protocol-neutral and the concrete
    transport is injected.
- No other **behavioral** deviations are permitted without an explicit entry in
  v4.0 §9.

## Subscription Delivery Model

`ConsumedThingHandle::observe_property` and `subscribe_event` return a
`Subscription`: a bounded per-subscription queue with drop-oldest backpressure,
drained synchronously via `Subscription::poll_next`. This is the primary
delivery primitive on every build, including `no_std + alloc`.

The design decouples *data arrival* from *data handling*: a client binding (the
producer) pushes remote samples into the queue; the application (the consumer)
drains it when ready. Application handler code never runs in the protocol
stack's execution context, which avoids reentrancy into the stack, unbounded
stack growth, and priority inversion on constrained devices.

- **MCU (`no_std`):** the pull queue is the safe model for bare-metal
  super-loops and cooperative RTOS tasks, where a callback fired from inside the
  protocol poll could self-deadlock or block the whole loop.
- **Host / gateway (`std` + `async`):** with the `async` feature, `Subscription`
  implements `futures_core::Stream`, so a host consumer drains it as
  `while let Some(payload) = sub.next().await`. The `Stream` impl layers a
  `core::task::Waker` notification on top of the same queue, keeping the queue
  the single source of truth and giving gateway consumers native push
  ergonomics without baking a runtime dependency into the core.
