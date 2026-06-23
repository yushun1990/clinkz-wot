//! Two-level locking for the exposed-Thing registry (baseline v3.0 §7).
//!
//! [`InMemoryExposedThingRegistry`] holds an outer [`MapLock`] around a
//! `BTreeMap<ThingId, Arc<ThingSlot>>`. Each [`ThingSlot`] owns a [`DrainFlag`]
//! (settable without the per-Thing lock) and an inner [`MapLock`] around
//! `Option<LocalThing>` — the `Option` lets `destroy` take the thing out
//! cleanly via [`Option::take`] instead of cloning a throwaway placeholder.
//!
//! ## Dispatch discipline (reentrancy-safe)
//!
//! ```text
//! lock map → clone Arc<ThingSlot> → drop map lock
//! lock thing → run handler → drop thing lock
//! ```
//!
//! Locks are never held across `.await` or across a handler that calls back
//! into the Servient. A handler calling `destroy(own_id)` sets the drain flag
//! (through `&self`, no lock needed) and the dispatch epilogue completes the
//! removal after the handler returns.

use alloc::{collections::BTreeMap, format, string::String, sync::Arc, vec::Vec};

use clinkz_wot_core::{AffordanceTarget, CoreError, LocalThing, MapLock, SecurityError};
use clinkz_wot_protocol_bindings::resolve_form_security;
use clinkz_wot_td::{data_type::Operation, security_scheme::SecurityScheme};
use clinkz_wot_td::{td_defaults::FormContext, td_defaults::effective_form_operations};

use crate::ServientError;
use crate::lock::DrainFlag;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct InboundResolutionKey {
    target: AffordanceTarget,
    operation: Operation,
}

impl InboundResolutionKey {
    fn new(target: &AffordanceTarget, operation: Operation) -> Self {
        Self {
            target: target.clone(),
            operation,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ResolvedInboundSecurity {
    pub(crate) schemes: Vec<(String, SecurityScheme)>,
    pub(crate) scopes: Vec<String>,
}

/// Per-Thing slot inside the registry.
///
/// The [`DrainFlag`] is settable through `&self` without acquiring the
/// per-Thing lock, which lets a handler call `destroy(own_id)` without
/// self-deadlock (baseline §7 edge case). The `Option<LocalThing>` lets
/// `destroy` extract the thing via [`Option::take`] when no handler is in
/// flight.
pub(crate) struct ThingSlot {
    draining: DrainFlag,
    thing: MapLock<Option<LocalThing>>,
    inbound_security: MapLock<BTreeMap<InboundResolutionKey, ResolvedInboundSecurity>>,
}

impl ThingSlot {
    fn new(thing: LocalThing) -> Self {
        Self {
            draining: DrainFlag::new(),
            thing: MapLock::new(Some(thing)),
            inbound_security: MapLock::new(BTreeMap::new()),
        }
    }

    /// Briefly locks the thing slot and runs `f` with `&mut LocalThing`.
    /// Returns `None` if the thing was already taken out by `destroy`.
    #[cfg(feature = "async")]
    pub(crate) fn with_thing<R>(&self, f: impl FnOnce(&mut LocalThing) -> R) -> Option<R> {
        self.thing.with(|opt| opt.as_mut().map(f))
    }

    fn clear_inbound_cache(&self) {
        self.inbound_security.with(BTreeMap::clear);
    }
}

/// Deterministic in-memory registry for locally exposed Things with two-level
/// locking (baseline v3.0 §5 / §7).
///
/// An internal concrete type; owned by the [`Servient`](crate::Servient) inner
/// state. The registry is fully interior-mutable — every method takes `&self`.
pub(crate) struct InMemoryExposedThingRegistry {
    things: MapLock<BTreeMap<String, Arc<ThingSlot>>>,
}

impl InMemoryExposedThingRegistry {
    /// Creates an empty exposed Thing registry.
    pub(crate) fn new() -> Self {
        Self {
            things: MapLock::new(BTreeMap::new()),
        }
    }

    /// Inserts a local Thing dispatcher by id.
    ///
    /// Returns `Err(ServientError::DuplicateExposedThing)` when an entry
    /// already exists. Holds the outer map lock only for the insert
    /// (baseline §7).
    pub(crate) fn insert(&self, id: String, thing: LocalThing) -> Result<(), ServientError> {
        self.things.with(|map| {
            if map.contains_key(&id) {
                return Err(ServientError::DuplicateExposedThing(id));
            }
            #[allow(clippy::arc_with_non_send_sync)]
            map.insert(id, Arc::new(ThingSlot::new(thing)));
            Ok(())
        })
    }

    /// Dispatches a closure against the locally exposed Thing under the
    /// per-Thing lock, following the baseline §7 dispatch discipline:
    ///
    /// 1. Lock map → clone `Arc<ThingSlot>` → drop map lock.
    /// 2. If the entry is draining, reject immediately.
    /// 3. Lock thing → run `f` (if the thing is still present) → drop thing
    ///    lock.
    ///
    /// Returns `None` when no entry exists for `id`, the entry is draining, or
    /// the thing was already taken out by a concurrent `destroy`.
    pub(crate) fn dispatch<R>(&self, id: &str, f: impl FnOnce(&mut LocalThing) -> R) -> Option<R> {
        let slot = self.things.with(|map| map.get(id).cloned())?;

        if slot.draining.get() {
            return None;
        }

        slot.thing.with(|opt| opt.as_mut().map(f))
    }

    /// Dispatches a TD mutation and clears cached inbound metadata for the
    /// Thing so subsequent inbound requests re-resolve forms and security.
    pub(crate) fn mutate<R>(&self, id: &str, f: impl FnOnce(&mut LocalThing) -> R) -> Option<R> {
        let slot = self.things.with(|map| map.get(id).cloned())?;

        if slot.draining.get() {
            return None;
        }

        let result = slot.thing.with(|opt| opt.as_mut().map(f));
        if result.is_some() {
            slot.clear_inbound_cache();
        }
        result
    }

    /// Returns the [`Arc<ThingSlot>`] for `id` without dispatching, for use by
    /// async dispatch paths that need take-out / await / return semantics.
    ///
    /// Returns `None` when no entry exists or the entry is draining.
    #[cfg(feature = "async")]
    pub(crate) fn slot_for(&self, id: &str) -> Option<Arc<ThingSlot>> {
        let slot = self.things.with(|map| map.get(id).cloned())?;
        if slot.draining.get() {
            return None;
        }
        Some(slot)
    }

    /// Marks the entry for `id` as draining and removes it from the map.
    ///
    /// **Immediate case** (no handler in flight): the per-Thing lock is
    /// acquirable. The [`LocalThing`] is taken out via [`Option::take`].
    ///
    /// **Deferred case** (handler in flight — e.g. `destroy(own_id)` called
    /// from within a handler): the [`DrainFlag`] is set so the dispatch
    /// epilogue knows the entry is gone, the entry is removed from the map to
    /// prevent new dispatches. The `LocalThing` will be dropped when the
    /// in-flight handler's dispatch releases the last `Arc<ThingSlot>`.
    ///
    /// Returns `true` when the entry was found (regardless of immediate or
    /// deferred). Returns `false` when no entry exists for `id`. In no case
    /// does the caller self-deadlock, because the map lock and per-Thing lock
    /// are independent primitives.
    pub(crate) fn destroy(&self, id: &str) -> bool {
        let slot = self.things.with(|map| {
            let slot = map.remove(id);
            if let Some(ref slot) = slot {
                slot.draining.set();
            }
            slot
        });

        if let Some(slot) = slot {
            // Best-effort extraction. If the lock is held (handler in flight),
            // try_with returns None and the thing is dropped later with the
            // slot's Arc. The caller does not need the LocalThing back.
            slot.thing.try_with(|opt| opt.take());
            true
        } else {
            false
        }
    }

    /// Returns the Thing Description for an exposed Thing by id, or `None`
    /// when no entry exists or the entry is draining.
    pub(crate) fn thing_description(&self, id: &str) -> Option<clinkz_wot_td::thing::Thing> {
        self.dispatch(id, |thing| thing.thing_description().clone())
    }

    /// Resolves and caches the security metadata for one inbound request.
    ///
    /// This avoids cloning the full TD and rescanning its forms for every
    /// inbound request on the same `(Thing, target, operation)` path.
    pub(crate) fn resolve_inbound_security(
        &self,
        id: &str,
        target: &AffordanceTarget,
        operation: Operation,
    ) -> Option<Result<ResolvedInboundSecurity, CoreError>> {
        let slot = self.things.with(|map| map.get(id).cloned())?;

        if slot.draining.get() {
            return None;
        }

        let key = InboundResolutionKey::new(target, operation);
        if let Some(cached) = slot.inbound_security.with(|cache| cache.get(&key).cloned()) {
            return Some(Ok(cached));
        }

        let resolved = slot.thing.with(|opt| {
            opt.as_ref().map(|thing| {
                resolve_inbound_security_from_thing(thing.thing_description(), target, operation)
            })
        })?;

        if let Ok(metadata) = &resolved {
            slot.inbound_security.with(|cache| {
                cache.insert(key, metadata.clone());
            });
        }

        Some(resolved)
    }
}

impl Default for InMemoryExposedThingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_inbound_security_from_thing(
    thing: &clinkz_wot_td::thing::Thing,
    target: &AffordanceTarget,
    operation: Operation,
) -> Result<ResolvedInboundSecurity, CoreError> {
    let Some(form) = find_form_for_operation(thing, target, operation) else {
        return Ok(ResolvedInboundSecurity::default());
    };

    let effective_security = resolve_form_security(thing, form);
    let mut schemes = Vec::with_capacity(effective_security.security.len());
    for scheme_name in effective_security.security {
        let scheme = thing.security_definitions.get(scheme_name).ok_or_else(|| {
            CoreError::Security(SecurityError::SchemeFailure(format!(
                "Security definition '{}' is not declared",
                scheme_name
            )))
        })?;
        schemes.push((scheme_name.clone(), scheme.clone()));
    }

    Ok(ResolvedInboundSecurity {
        schemes,
        scopes: effective_security.scopes.to_vec(),
    })
}

fn find_form_for_operation<'a>(
    thing: &'a clinkz_wot_td::thing::Thing,
    target: &AffordanceTarget,
    operation: Operation,
) -> Option<&'a clinkz_wot_td::form::Form> {
    let (forms, context) = match target {
        AffordanceTarget::Thing => (thing.forms.as_deref().unwrap_or(&[]), FormContext::Thing),
        AffordanceTarget::Property(name) => {
            let property = thing.properties.as_ref()?.get(name)?;
            (
                property._interaction.forms.as_slice(),
                FormContext::Property(property),
            )
        }
        AffordanceTarget::Action(name) => {
            let action = thing.actions.as_ref()?.get(name)?;
            (
                action._interaction.forms.as_slice(),
                FormContext::Action(action),
            )
        }
        AffordanceTarget::Event(name) => {
            let event = thing.events.as_ref()?.get(name)?;
            (
                event._interaction.forms.as_slice(),
                FormContext::Event(event),
            )
        }
    };

    forms
        .iter()
        .find(|form| effective_form_operations(context, form).contains(&operation))
}
