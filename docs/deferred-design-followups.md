# Deferred Design Follow-ups

This document records design and performance improvements identified during the
performance-and-elegance review that were intentionally deferred. Each entry
explains the issue, the value, and why it was not taken in the same pass as the
[in-place performance hardening](../PLAN.md) so a future change can pick it up
with full context.

The discovery/directory redesign is no longer tracked here. It has moved to the
dedicated plan document
[`docs/plan/discovery-directory-refactor-plan.md`](plan/discovery-directory-refactor-plan.md).

Entries are ordered by recommended priority.

## 1. `Payload` media-metadata sharing (medium value, API churn)

`Payload.body` is already stored as `Arc<[u8]>` (`core/src/payload.rs`), so the
largest event fan-out clone cost is gone. The remaining always-cloned media
metadata still uses owned strings: `content_type: String` and, if present,
`content_coding: Option<String>`.

If clone pressure on payload metadata becomes measurable, changing
`content_type` to `Arc<str>` (and possibly `content_coding` to
`Option<Arc<str>>`) would make the immutable metadata path mirror the already
shared payload bytes.

Why deferred: this is now a smaller optimization with public-type churn. The
change would still require updating payload assertions and APIs that currently
assume `String`-backed fields, so it is best done only if profiling shows the
remaining string clones matter.

## 2. Handler-trait consolidation (high value, large refactor)

`core/src/thing.rs` now exposes narrowly scoped handler traits for property
read/write/observe/unobserve, action invoke/query/cancel, and event
subscribe/unsubscribe, plus async read/write/action twins. `LocalThing` groups
them into per-affordance handler sets, but the public registration and
dispatcher surface is still broad and mechanically repetitive.

W3C Scripting API treats these operations as facets of the same affordance. A
future redesign could evaluate composite per-affordance handler traits with
default `MissingHandler` or empty-ack behavior instead of continuing to expose a
large collection of single-method public traits.

Why deferred: this touches the handler-registration API, servient dispatch
paths, fallback semantics, and a wide test surface. It remains architecture work
best handled in a standalone refactor.

## 3. `ExposedThing` / `ConsumedThing` trait removal (medium value)

`core/src/thing.rs` defines `ExposedThing` and `ConsumedThing` traits that
each have exactly one implementor in the whole workspace (`LocalThing` and
`BoundConsumedThing`). No fakes, no alternative implementations. Premature
abstraction that bloats the public API.

Why deferred: removing the traits re-exposes `LocalThing`/`BoundConsumedThing`
on the public surface. Decide whether a future `RemoteExposedThing` proxy is
planned before deleting; otherwise just use the concrete types.

## 4. `apply_security` outbound metadata extraction (medium value, trait change)

`servient/src/interaction.rs` already hoists `TransportRequest` allocation out
of the per-scheme loop and reuses the metadata buffer with `clear()` +
`extend(...)`, so the earlier per-scheme map-clone cost has been removed. The
remaining inefficiency is that `apply_security` still performs a post-`apply`
diff to discover which metadata entries the security provider added.

Change `SecurityProvider::apply` to return the metadata it added explicitly.
That would remove the post-`apply` diff while preserving the current
scheme-isolation behavior.

Why deferred: it changes the `SecurityProvider` trait contract and all
implementors; the multi-scheme isolation guarantee must be re-validated.

## 5. `data_type.rs` split (medium value, mechanical)

`td/src/core/data_type.rs` is still a catch-all mixing URI types,
`ExtensionMap`, `MultiLanguage`, `VersionInfo`, `Operation`, `ExpectedResponse`,
and `Metadata`. This violates the AGENTS.md "avoid large single-file" guidance.

Split into `core/uri.rs`, `core/metadata.rs`, `core/version.rs`,
`core/response.rs`, `core/operation.rs`. Mechanical but touches every
`use crate::data_type::…` in the TD crate.

## 6. `ThingModelForm` / `Form` deduplication (medium value)

`td/src/thing_model.rs` `ThingModelForm` still duplicates `Form` fields and
(de)serde logic from `td/src/components/form.rs`, differing mainly in
`href: Option<FormHref>` vs `href: FormHref`. Extract a shared `FormData` core.

## 7. Shared `Thing` / `ThingModel` validation helpers (medium value)

~150 lines of properties/actions/events validation logic are copy-pasted
between `td/src/thing.rs` and `td/src/thing_model.rs`. `Thing` preserves the
error variant via `prepend_context`; `ThingModel` collapses non-schema errors
into `InvalidSchema`, losing semantic information. Extract a shared helper
parameterized by an error-context strategy.

## 8. Minor items (low value)

- Some `BindingError` variants still carry free-form `String` messages
  (`protocol-bindings/core/src/error.rs`): convert to structured variants so
  callers can match programmatically and defer allocation to `Display`. Keep the
  existing structured `UnknownAffordance { kind, name }` shape.
- `Operation::as_str()` duplicates the serde `rename_all = "lowercase"` mapping
  (`td/src/core/data_type.rs`): two sources of truth that can silently drift.
- `LocalThing` handler maps keyed by `String` while `AffordanceTarget` carries
  `Arc<str>`: forces conversion on every dispatch.
- `register_*_handler` methods have inconsistent semantics (only
  `register_action_handler` returns the displaced handler).
- Traits (`ClientBinding`, `ServerBinding`, `PayloadCodec`, handler traits)
  are unsealed: document which are stable extension points.

## Status convention

When an entry above is started, move it to PLAN.md's "Performance Hardening"
section (or a dedicated milestone) and delete it here.
