use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
};

use clinkz_wot_core::{
    AffordanceTarget, CoreError, CoreResult, InteractionInput, InteractionOutput, Payload,
    Subscription, SubscriptionGuard, WotLock,
};
#[cfg(feature = "async")]
use clinkz_wot_core::{BindingRequest, ClientBinding};
use clinkz_wot_protocol_bindings::{AffordanceRef, BindingError, validate_form_operation};
use clinkz_wot_td::{data_type::Operation, form::Form};

use crate::ZenohOperationPlan;

/// Request passed from the zenoh binding planner to a zenoh transport adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenohTransportRequest {
    /// Concrete zenoh execution plan, shared by reference so repeated
    /// interactions against a cached plan avoid cloning the plan's strings on
    /// every request.
    pub plan: Arc<ZenohOperationPlan>,
    /// Optional encoded payload from the WoT interaction input.
    pub payload: Option<Payload>,
    /// Runtime parameters supplied by the caller.
    pub parameters: BTreeMap<String, String>,
}

/// Transport adapter contract for concrete zenoh runtime integrations.
///
/// This trait deliberately avoids depending on a concrete zenoh session type so
/// std, constrained, and test runtimes can provide their own integration layer.
/// The receiver is `&self` (baseline addendum §2.4 / §7): each concrete backend
/// owns its own interior mutability for I/O state.
pub trait ZenohTransport {
    /// Executes a planned zenoh operation.
    fn execute(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput>;

    /// Opens a long-lived zenoh subscription for streaming observe/subscribe
    /// operations.
    ///
    /// Returns a consumer-side [`Subscription`] for draining samples and a
    /// [`SubscriptionGuard`] that owns the underlying zenoh subscriber.
    /// The default implementation returns `UnsupportedOperation`.
    fn open_subscription(
        &self,
        _request: ZenohTransportRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        Err(CoreError::UnsupportedOperation(
            "Zenoh transport does not support streaming subscriptions".into(),
        ))
    }
}

/// Zenoh binding implementation.
///
/// This type implements protocol selection and target extraction while keeping
/// concrete zenoh session execution behind an injected transport adapter.
///
/// # Per-form plan cache
///
/// Each `(Arc<Form>, Operation)` pair produces a deterministic
/// [`ZenohOperationPlan`] (target resolution, operation-kind mapping, and
/// metadata extraction). The binding retains computed plans in an internal
/// cache keyed by form pointer identity so that repeated interactions against
/// the same selected form — the common case for the Servient consumed-Thing
/// registry, which stores a live `Arc<Form>` per `(affordance, operation)`
/// — skip the target resolution, key-expr allocation, and extension-metadata
/// clones on every request.
///
/// Cache entries hold a [`Weak<Form>`] so short-lived caller forms do not pin
/// stale plans forever; dead entries are pruned opportunistically on cache-miss
/// inserts.
pub struct ZenohBindingTransport<T> {
    /// Bitset of supported [`Operation`]s (bit position = discriminant).
    supported_operations: u32,
    transport: T,
    plan_cache: WotLock<BTreeMap<PlanCacheKey, PlanCacheEntry>>,
}

/// Identity key for the per-form plan cache. Uses the `Arc<Form>` allocation
/// address, which is stable for the lifetime of the `Arc` and unique
/// process-wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PlanCacheKey {
    form_ptr: usize,
    operation: Operation,
}

#[derive(Clone)]
struct PlanCacheEntry {
    /// Weak reference so the entry is evicted when the caller drops the form.
    form_weak: Weak<Form>,
    plan: Arc<ZenohOperationPlan>,
}

impl<T> core::fmt::Debug for ZenohBindingTransport<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ZenohBindingTransport")
            .field("supported_operations", &self.supported_operations)
            .finish_non_exhaustive()
    }
}

fn ops_to_bitset(ops: impl IntoIterator<Item = Operation>) -> u32 {
    ops.into_iter()
        .map(|op| 1u32 << (op as u32))
        .fold(0, |acc, bit| acc | bit)
}

impl<T> ZenohBindingTransport<T> {
    /// Creates a zenoh binding with an explicit supported operation set and an attached transport.
    pub fn with_transport_and_supported_operations(
        transport: T,
        operations: impl IntoIterator<Item = Operation>,
    ) -> Self {
        Self {
            supported_operations: ops_to_bitset(operations),
            transport,
            plan_cache: WotLock::new(BTreeMap::new()),
        }
    }

    /// Creates a zenoh binding with the default WoT operation set and an attached transport.
    pub fn with_transport(transport: T) -> Self {
        Self {
            supported_operations: default_supported_operations(),
            transport,
            plan_cache: WotLock::new(BTreeMap::new()),
        }
    }

    /// Returns a shared reference to the underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Returns a mutable reference to the underlying transport.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl<T> ClientBinding for ZenohBindingTransport<T>
where
    T: ZenohTransport + Send + Sync,
{
    fn supports(&self, form: &Form, operation: Operation) -> bool {
        (self.supported_operations & (1u32 << (operation as u32))) != 0
            && crate::form::is_zenoh_form(form)
    }

    fn supports_with_thing(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        form: &Form,
        operation: Operation,
    ) -> bool {
        (self.supported_operations & (1u32 << (operation as u32))) != 0
            && crate::form::is_zenoh_form_target(thing, form)
    }

    async fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        validate_form_operation(
            &request.thing,
            affordance_ref_from_target(&request.target),
            &request.form,
            request.operation,
        )
        .map_err(core_error_from_binding_error)?;

        let plan = self.plan_for(&request.thing, &request.form, request.operation)?;
        let transport_request = build_zenoh_transport_request(Arc::clone(&plan), request.input);

        self.transport.execute(transport_request)
    }

    async fn subscribe(
        &self,
        request: BindingRequest,
    ) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)> {
        validate_form_operation(
            &request.thing,
            affordance_ref_from_target(&request.target),
            &request.form,
            request.operation,
        )
        .map_err(core_error_from_binding_error)?;

        let plan = self.plan_for(&request.thing, &request.form, request.operation)?;
        let transport_request = build_zenoh_transport_request(Arc::clone(&plan), request.input);

        self.transport.open_subscription(transport_request)
    }
}

impl<T> ZenohBindingTransport<T> {
    /// Returns the cached [`ZenohOperationPlan`] for the given form and
    /// operation, computing and inserting it on miss.
    ///
    /// Plans are keyed by the form's `Arc` allocation address. The cached
    /// entry holds a [`Weak<Form>`]; if the caller has since dropped the form,
    /// the stale entry is pruned before the next miss is inserted.
    /// `thing` is only consulted on the cache-miss path so a cache hit avoids
    /// not just target resolution but also the form/affordance walk.
    fn plan_for(
        &self,
        thing: &clinkz_wot_td::thing::Thing,
        form: &Arc<Form>,
        operation: Operation,
    ) -> CoreResult<Arc<ZenohOperationPlan>> {
        let key = PlanCacheKey {
            form_ptr: Arc::as_ptr(form) as usize,
            operation,
        };

        // Fast path: cache hit with a live form. Uses a *read* lock — the
        // closure only does `get` + `Arc::clone`, so concurrent consumers of
        // the same `ZenohBindingTransport` do not serialize on cache lookups.
        // The lock is released before the plan is rebuilt on miss, so the
        // slow path does not hold the cache lock during target resolution /
        // metadata extraction.
        if let Some(plan) = self.plan_cache.with_read_recover(|cache| {
            cache.get(&key).and_then(|entry| {
                // `Weak::upgrade` is the canonical liveness probe; `strong_count`
                // is a non-atomic snapshot.
                entry
                    .form_weak
                    .upgrade()
                    .is_some()
                    .then(|| Arc::clone(&entry.plan))
            })
        }) {
            return Ok(plan);
        }

        // Slow path: compute, cache, and return. The plan is a pure function
        // of (form, operation), and the form is immutable for its `Arc`'s
        // lifetime, so caching is safe.
        let plan = Arc::new(
            crate::form::plan_zenoh_operation(thing, form, operation)
                .map_err(core_error_from_zenoh_error)?,
        );
        self.plan_cache.with(|cache| {
            prune_dead_plan_cache_entries(cache);
            cache.insert(
                key,
                PlanCacheEntry {
                    form_weak: Arc::downgrade(form),
                    plan: Arc::clone(&plan),
                },
            );
        });
        Ok(plan)
    }
}

fn prune_dead_plan_cache_entries(cache: &mut BTreeMap<PlanCacheKey, PlanCacheEntry>) {
    cache.retain(|_, entry| entry.form_weak.upgrade().is_some());
}

/// Builds a transport request from a zenoh execution plan and WoT interaction input.
pub fn build_zenoh_transport_request(
    plan: Arc<ZenohOperationPlan>,
    input: InteractionInput,
) -> ZenohTransportRequest {
    ZenohTransportRequest {
        plan,
        payload: input.data,
        parameters: input.uri_variables,
    }
}

fn core_error_from_binding_error(err: BindingError) -> CoreError {
    match err {
        BindingError::UnknownAffordance { kind, name } => {
            CoreError::UnknownAffordance { kind, name }
        }
        BindingError::UnsupportedOperation(message) => CoreError::UnsupportedOperation(message),
        BindingError::MetadataMismatch(message) => CoreError::InvalidInteraction(message),
        BindingError::CallerFilterMismatch(message) => CoreError::InvalidInteraction(message),
        BindingError::FormNotInAffordance => CoreError::InvalidInteraction(err.to_string()),
        BindingError::TargetResolution(message) => {
            CoreError::InvalidInteraction(message.to_string())
        }
    }
}

fn core_error_from_zenoh_error(err: crate::ZenohBindingError) -> CoreError {
    match err {
        // Preserve structured shared-binding errors (e.g. `UnknownAffordance`)
        // by routing them through the same mapper used by the direct
        // `validate_form_operation` path, instead of collapsing them to
        // `InvalidInteraction(String)`.
        crate::ZenohBindingError::Shared(binding_error) => {
            core_error_from_binding_error(binding_error)
        }
        crate::ZenohBindingError::Selection(message)
        | crate::ZenohBindingError::UnsupportedForm(message)
        | crate::ZenohBindingError::InvalidExtension { message, .. } => {
            CoreError::InvalidInteraction(message)
        }
        crate::ZenohBindingError::Target(message) => {
            CoreError::InvalidInteraction(message.to_string())
        }
    }
}

fn affordance_ref_from_target(target: &AffordanceTarget) -> AffordanceRef<'_> {
    match target {
        AffordanceTarget::Thing => AffordanceRef::Thing,
        AffordanceTarget::Property(name) => AffordanceRef::Property(name),
        AffordanceTarget::Action(name) => AffordanceRef::Action(name),
        AffordanceTarget::Event(name) => AffordanceRef::Event(name),
    }
}

fn default_supported_operations() -> u32 {
    use Operation::*;

    ops_to_bitset([
        ReadProperty,
        WriteProperty,
        ObserveProperty,
        UnobserveProperty,
        InvokeAction,
        QueryAction,
        CancelAction,
        SubscribeEvent,
        UnsubscribeEvent,
        ReadAllProperties,
        WriteAllProperties,
        ReadMultipleProperties,
        WriteMultipleProperties,
        ObserveAllProperties,
        UnobserveAllProperties,
        QueryAllActions,
        SubscribeAllEvents,
        UnsubscribeAllEvents,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    use clinkz_wot_core::InteractionOutput;
    use clinkz_wot_td::{
        affordance::{InteractionHelper, PropertyAffordance},
        data_schema::DataSchema,
        thing::Thing,
    };

    struct NoopZenohTransport;

    impl ZenohTransport for NoopZenohTransport {
        fn execute(&self, _request: ZenohTransportRequest) -> CoreResult<InteractionOutput> {
            Ok(InteractionOutput::empty())
        }
    }

    #[test]
    fn plan_cache_prunes_dead_form_entries_before_inserting_new_plan() {
        let thing = Thing::builder("Lamp")
            .nosec()
            .property(
                "status",
                PropertyAffordance::builder(DataSchema::String(DataSchema::string().build()))
                    .form(
                        Form::read_property("zenoh://clinkz/things/lamp/properties/status")
                            .build()
                            .expect("build form"),
                    )
                    .build()
                    .expect("build property"),
            )
            .build()
            .expect("build thing");
        let binding = ZenohBindingTransport::with_transport(NoopZenohTransport);

        let first = Arc::new(
            Form::read_property("zenoh://clinkz/things/lamp/properties/status")
                .build()
                .expect("build first form"),
        );
        binding
            .plan_for(&thing, &first, Operation::ReadProperty)
            .expect("cache first plan");
        assert_eq!(binding.plan_cache.with_read_recover(|cache| cache.len()), 1);
        drop(first);

        let second = Arc::new(
            Form::read_property("zenoh://clinkz/things/lamp/properties/temperature")
                .build()
                .expect("build second form"),
        );
        binding
            .plan_for(&thing, &second, Operation::ReadProperty)
            .expect("cache second plan");

        assert_eq!(
            binding.plan_cache.with_read_recover(|cache| cache.len()),
            1,
            "stale entries from dropped forms should be pruned on the next cache miss"
        );
    }
}
