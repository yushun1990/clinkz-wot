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
`ConsumedThing`). No fakes, no alternative implementations. Premature
abstraction that bloats the public API.

Why deferred: removing the traits re-exposes `LocalThing`/`ConsumedThing`
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

## 9. `AsyncSecurityProvider` (audit round-2 O2/AD43)

`SecurityProvider::verify` runs synchronously on the inbound dispatch hot path
before the handler (baseline §7.5). The v1 contract is that it must be
non-blocking/short, like a sync handler. Deployments whose verification is
genuinely I/O-bound or CPU-heavy (JWT/signature validation, OCSP, remote
auth) need an async twin so they do not block the executor the same way a
blocking sync handler would.

Add `AsyncSecurityProvider` (`async verify`/`async apply` twins behind the
`async` feature, mirroring the handler sync-primary/opt-in-async policy of
§4.2). The dispatcher selects the async twin when registered and the build has
`async`; otherwise the sync `verify` runs on the hot-path budget.

Why deferred: it widens the security trait surface and the dispatcher's
security path; v1 documents the non-blocking constraint on the sync path
instead. Picked up when a real binding ships a verification flow that cannot
meet the sync budget.

## 10. Per-slot `ArcSwapOption` handler slots (audit round-2 P-2/AD51)

The consolidated `HandlerSet` is a multi-field struct published as one
`Arc<HandlerSet>` via `ArcSwap` (baseline §4.7). Swapping one slot rebuilds the
whole struct + republishes one `Arc` — one allocation per handler swap, off the
per-request hot path.

If profiling shows **runtime** handler swapping (post-`expose`, per AD14) to be
a hot allocation source, move each slot to its own
`arc_swap::ArcSwapOption<Arc<dyn …>>` so one slot swaps without rebuilding the
struct.

Why deferred: the expected handler-swap rate is setup-phase wiring plus
occasional runtime re-attachment — one alloc per swap is acceptable, and
per-slot `ArcSwapOption` complicates the `HandlerSet` shape and dispatch read
(one load per slot instead of one load per set). Re-evaluate once a workload
exists.

## 11. Configurable bulk fan-out concurrency bound (audit round-2 P-3/AD52)

Bulk operations (`readAll`/`readMultiple`/…) fan out per-property through a
bounded `buffer_unordered(bound)` on std (baseline §7.4). The default bound is
the property count. A configurable bound (e.g. a per-Servient or per-call
concurrency limit) lets a caller cap concurrent network fan-out.

Why deferred: the default (bound = property count) is correct for the common
case; a knob adds API surface and tuning burden. Add when a deployment shows
N-way fan-out storms that the default does not bound.

## 12. Wire the outbound `SecurityProvider::apply` + `CredentialStore` path (high value, cross-cutting)

The repo defines a complete outbound credential mechanism that is **designed
but unwired**:

- `CredentialStore` trait (`core/src/security.rs:55–59`), keyed by
  `(thing_id, scheme_name)` → `Credentials` (`BearerToken`, `Basic`, `ApiKey`,
  `Psk`, `Other`). Only `InMemoryCredentialStore` implements it.
- `SecurityContext<'a>` (`security.rs:10–22`) carries
  `credentials: Option<&'a dyn CredentialStore>` alongside thing/form/scheme,
  and is the parameter to `SecurityProvider::apply`.
- `SecurityProvider::apply` (`security.rs:149`) — intended to attach security
  material (headers, tokens, attachments) to a `TransportRequest` for a
  consumed interaction.

None of this reaches the outbound dispatch path today:

- `SecurityProvider::apply` has **zero production callers**. The built-in
  `BearerSecurityProvider` / `BasicSecurityProvider` `apply` impls are no-ops
  with comments noting "the binding handles it" (`security.rs:455, 540`).
- `SecurityContext` is **never constructed** by production code.
- `BindingRequest` (`core/src/binding.rs:29–40`) carries `thing`, `target`,
  `operation`, `form`, `input` — **no credential store, no security context**.
- `ConsumedThing` (`core/src/thing.rs:1089–1092`) holds only `thing` and
  `bindings` — no reference to `Servient::security_providers` or any store.
- `Servient::consume` (`servient/src/servient.rs:104–118`) registers client
  bindings but never passes credentials or providers to the `ConsumedThing`.

**Impact**: per-form request-level security on consumed interactions (a
`bearer` token in a zenoh attachment, an `apikey` header on HTTP) cannot be
honored by the engine. The only wired credential injection point today is
binding-local session-level config (e.g. zenoh's `ZenohSessionPolicy`, which
covers TLS/PSK at connect time but not per-request tokens). This affects
**every** binding, not just zenoh — see
[`docs/zenoh-binding-template.md`](zenoh-binding-template.md) §6.3.4.

**Spec grounding**: the WoT Scripting API deliberately leaves credential
acquisition implementation-defined (§11.1.4: "the runtime should not expose
any API for scripts to query the provisioned security credentials"). This
wiring is how `clinkz-wot` provides that implementation-defined channel: the
`CredentialStore` is injected at the runtime layer (not script/TD-visible),
and `apply` is the protocol-neutral point where a `SecurityProvider` attaches
material to an outbound request. The TD carries only security *configuration*
(`securityDefinitions`); secrets come from the store.

**What wiring involves**:

- Thread an `Arc<dyn CredentialStore>` (or `SecurityContext` factory) from
  `Servient` → `ConsumedThing` → `BindingRequest`.
- Invoke `SecurityProvider::apply` in `ConsumedThing::request` /
  `subscribe` (`core/src/thing.rs:1127–1203`) after form selection, before
  `binding.invoke`, for the form's effective security scheme
  (`effective_form_security`, `td/src/td_defaults.rs:56–60`).
- Implement non-no-op `apply` in the built-in providers (bearer → attachment
  / header; basic → header; apikey → header/query per `in`).
- Decide the relationship to the inbound path: inbound `verify` IS wired
  (`Servient::dispatch`, `servient.rs:231–275`), but it currently uses
  Thing-level `td.security`, not per-form security — a separate divergence to
  reconcile in the same pass (the repo's own `effective_form_security` exists
  for exactly this).

Why deferred: it is a cross-cutting engine change (`core`, `servient`, every
binding's outbound path) with security-correctness implications, orthogonal to
the zenoh URI-convention work. It is a prerequisite for honoring per-form
`bearer`/`apikey` schemes on consumed interactions across all bindings, and
should be picked up as a dedicated security-wiring milestone.

## Status convention

When an entry above is started, move it to PLAN.md's "Performance Hardening"
section (or a dedicated milestone) and delete it here.
