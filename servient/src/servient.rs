//! `Servient` — composition root (baseline v4.0 §7 / phase-p3 §3.1–§3.2).
//! Non-generic; holds registries, bindings, a `Discoverer`, and the dispatch
//! logic. **Driving (loop / poll) is NOT on the Servient** — each binding
//! decides its own dispatch model (fan-in channel, direct dispatch, or poll)
//! via the `Dispatch` handle injected in `configure(&BindingContext)`.

use alloc::{boxed::Box, format, sync::Arc, vec::Vec};

use clinkz_wot_core::{
    ClientBindingFactory, Dispatch, EventBroker, EventName, ExposedThing, InboundRequest,
    InboundResponse, InteractionOutput, Payload, Principal, SecurityProvider, ServerBinding,
    ThingId, WotLock,
};
use clinkz_wot_discovery::{Discoverer, DiscoveryFilter, ProcessState, ThingDiscoveryProcess};
use clinkz_wot_td::{AbsoluteUri, thing::Thing};

use crate::handle::{ConsumedThingHandle, ExposedThingHandle};
use crate::registry::{ConsumedThingRegistry, ExposedThingRegistry, ExposedThingSlot};
use crate::{ServientError, ServientResult};

// `ClientBindingFactory` lives in `clinkz_wot_core` (moved in P0). Re-exported
// from the Servient crate for P0 backward compatibility; P1 will retire this
// re-export so application code imports it from `clinkz_wot_core` (or, more
// commonly, never names the trait directly and instead implements
// `clinkz_wot_core::ProtocolBinding`).

/// The Servient: composes exposed/consumed Things, server/client bindings, and
/// discovery. `Clone`, `&self`, `Send + Sync`.
///
/// **Driving is NOT here.** The Servient provides [`Dispatch::serve_request`]
/// (the dispatch logic). Each binding calls it however fits its transport:
///
/// | Binding type | How dispatch is invoked |
/// |---|---|
/// | zenoh (sync callbacks) | Binding owns a channel + draining task that calls `serve_request(req).await` |
/// | HTTP / CoAP (async handlers) | Route handler calls `serve_request(req).await` directly |
/// | bare no_std | Super-loop polls `try_accept`, then calls dispatch |
#[derive(Clone)]
pub struct Servient {
    pub(crate) exposed: ExposedThingRegistry,
    #[allow(dead_code)]
    consumed_registry: ConsumedThingRegistry,
    pub(crate) server_bindings: Arc<[Arc<dyn ServerBinding>]>,
    #[cfg(feature = "async")]
    pub(crate) client_factories: Arc<[Arc<dyn ClientBindingFactory>]>,
    pub(crate) security_providers: Arc<[Arc<dyn SecurityProvider>]>,
    pub(crate) discoverer: Arc<dyn Discoverer>,
    pub(crate) event_broker: EventBroker,
    shutdown: Arc<core::sync::atomic::AtomicBool>,
}

/// Drives the shutdown flag for graceful teardown.
#[derive(Clone)]
pub struct ShutdownHandle {
    flag: Arc<core::sync::atomic::AtomicBool>,
}

impl ShutdownHandle {
    pub fn shutdown(&self) {
        self.flag.store(true, core::sync::atomic::Ordering::SeqCst);
    }
}

impl Servient {
    /// Assembles a `Servient` (called by `ServientBuilder`).
    #[cfg(feature = "std")]
    pub(crate) fn assemble(
        exposed: ExposedThingRegistry,
        consumed_registry: ConsumedThingRegistry,
        server_bindings: Arc<[Arc<dyn ServerBinding>]>,
        client_factories: Arc<[Arc<dyn ClientBindingFactory>]>,
        security_providers: Arc<[Arc<dyn SecurityProvider>]>,
        discoverer: Arc<dyn Discoverer>,
        event_broker: EventBroker,
    ) -> Self {
        Self {
            exposed,
            consumed_registry,
            server_bindings,
            client_factories,
            security_providers,
            discoverer,
            event_broker,
            shutdown: Arc::new(core::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn shutdown_handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            flag: Arc::clone(&self.shutdown),
        }
    }

    // --- facade (WoT surface) ---

    pub fn produce(&self, td: Thing) -> ServientResult<ExposedThingHandle> {
        let id = td
            .id
            .as_ref()
            .map(|u| ThingId::from(u.as_str()))
            .ok_or(ServientError::MissingThingId)?;
        let slot = Arc::new(WotLock::new(ExposedThingSlot::new(ExposedThing::new(td))));
        Ok(ExposedThingHandle::new(self.clone(), slot, id))
    }

    #[cfg(feature = "async")]
    pub fn consume(&self, td: Thing) -> ServientResult<ConsumedThingHandle> {
        use clinkz_wot_core::ConsumedThing;
        let id = td
            .id
            .as_ref()
            .map(|u| ThingId::from(u.as_str()))
            .ok_or(ServientError::MissingThingId)?;
        let mut consumed = ConsumedThing::new(td);
        for factory in self.client_factories.iter() {
            consumed.register_binding(factory.build());
        }
        self.consumed_registry.track(id.clone());
        Ok(ConsumedThingHandle::new(self.clone(), consumed, id))
    }

    #[cfg(feature = "async")]
    pub fn discover(&self, filter: DiscoveryFilter) -> ThingDiscoveryProcess {
        match self.discoverer.discover(filter) {
            Ok(process) => process,
            Err(err) => ThingDiscoveryProcess::new(Box::new(ProcessState::done(Some(err)))),
        }
    }

    #[cfg(feature = "async")]
    pub async fn fetch_td(&self, url: &AbsoluteUri) -> ServientResult<Thing> {
        Ok(self.discoverer.request_thing_description(url).await?)
    }

    // --- lifecycle hooks (called by ExposedThingHandle) ---

    pub(crate) async fn expose_thing(
        &self,
        id: ThingId,
        slot: Arc<WotLock<ExposedThingSlot>>,
    ) -> ServientResult<()> {
        if self.exposed.contains(&id) {
            return Err(ServientError::AlreadyExposed(id));
        }
        let td = slot.with_read(|s| s.thing.thing_description().clone());
        let mut registered: Vec<usize> = Vec::new();
        for (i, binding) in self.server_bindings.iter().enumerate() {
            if let Err(err) = binding.register_thing(&id, &td) {
                for &j in registered.iter().rev() {
                    self.server_bindings[j].unregister_thing(&id);
                }
                return Err(ServientError::Serve(err));
            }
            registered.push(i);
        }
        if self.exposed.insert(id.clone(), slot).is_err() {
            for binding in self.server_bindings.iter() {
                binding.unregister_thing(&id);
            }
            return Err(ServientError::AlreadyExposed(id));
        }
        Ok(())
    }

    pub(crate) async fn destroy_thing(&self, id: &ThingId) -> ServientResult<()> {
        let Some(slot) = self.exposed.get(id) else {
            return Ok(());
        };
        for binding in self.server_bindings.iter() {
            binding.unregister_thing(id);
        }
        slot.with(|s| {
            s.draining.store(true, core::sync::atomic::Ordering::SeqCst);
        });
        self.exposed.remove(id);
        Ok(())
    }

    pub(crate) fn emit_event(
        &self,
        thing: &ThingId,
        name: &str,
        payload: Payload,
    ) -> Result<(), clinkz_wot_core::CoreError> {
        self.event_broker
            .publish(thing, &EventName::from(name), &payload)
    }

    // --- dispatch (the ONLY driving-related logic on the Servient) ---

    /// Dispatch routing (§3.6). Resolves the exposed Thing, checks draining,
    /// routes by operation to the handler.
    ///
    /// **This is called by bindings, not by a Servient-owned loop.** Each
    /// binding invokes it via [`Dispatch::serve_request`] in whatever context
    /// fits its transport model.
    pub(crate) async fn dispatch(&self, request: InboundRequest) -> InboundResponse {
        use clinkz_wot_core::CoreError;
        use clinkz_wot_td::data_type::Operation;

        let correlation = request.correlation.clone();
        let InboundRequest {
            thing_id,
            target,
            operation,
            mut input,
            auth,
            ..
        } = request;

        let Some(slot) = self.exposed.get(&thing_id) else {
            return InboundResponse::error(
                correlation,
                CoreError::InboundDispatch("Thing gone".into()),
            );
        };
        if slot.with_read(|s| s.draining.load(core::sync::atomic::Ordering::SeqCst)) {
            return InboundResponse::error(
                correlation,
                CoreError::InboundDispatch("Thing gone".into()),
            );
        }

        // --- Security verification (P-Sec) ---
        //
        // Resolve the Thing's effective security scheme(s) and verify each
        // against a registered SecurityProvider. An empty `security` list
        // means open access (W3C TD 1.1 default). If a scheme is declared
        // but no matching provider is registered, reject with
        // `UnsupportedScheme` — strict by default prevents accidental open
        // access. On success, the established Principal is injected into
        // the handler-facing InteractionInput.

        let security_check = slot.with_read(|s| -> Result<Option<Principal>, CoreError> {
            let td = s.thing.thing_description();
            if td.security.is_empty() {
                return Ok(None); // open access, no principal
            }
            let mut established_principal: Option<Principal> = None;
            for scheme_name in &td.security {
                let scheme = td.security_definitions.get(scheme_name).ok_or_else(|| {
                    CoreError::Security(clinkz_wot_core::SecurityError::SchemeFailure(
                        alloc::format!(
                            "security definition '{}' referenced by Thing.security but not found in securityDefinitions",
                            scheme_name
                        ),
                    ))
                })?;

                let provider = self
                    .security_providers
                    .iter()
                    .find(|p| p.scheme_name() == scheme_name.as_str());

                let provider = provider.ok_or(CoreError::Security(
                    clinkz_wot_core::SecurityError::UnsupportedScheme,
                ))?;

                // Reconstruct a minimal InboundRequest for the provider's
                // verify call. The provider only inspects `auth`, `target`,
                // and `operation`; the input/correlation fields are not
                // consulted by the built-in providers. This reconstruction
                // avoids borrowing `input` (which is `&mut` outside).
                let verify_req = InboundRequest::new(
                    thing_id.clone(),
                    target.clone(),
                    operation,
                    clinkz_wot_core::InteractionInput::empty(),
                );
                let mut verify_req = verify_req;
                verify_req.auth = auth.clone();

                let principal = provider
                    .verify(&verify_req, scheme)
                    .map_err(CoreError::from)?;
                if established_principal.is_none() {
                    established_principal = Some(principal);
                }
            }
            Ok(established_principal)
        });

        match security_check {
            Ok(principal) => {
                input.principal = principal;
            }
            Err(err) => {
                return InboundResponse::error(correlation, err);
            }
        }

        // --- Handler dispatch ---

        let result = slot.with_read(|s| -> Result<InteractionOutput, CoreError> {
            let name = target.name().unwrap_or("");
            match operation {
                Operation::ReadProperty => s.thing.read_property(name, &input),
                Operation::WriteProperty => s.thing.write_property(name, &mut input),
                Operation::InvokeAction => s.thing.invoke_action(name, &mut input),
                Operation::QueryAction => s.thing.query_action(name, &input),
                Operation::CancelAction => s.thing.cancel_action(name, &mut input),
                Operation::SubscribeEvent => s.thing.subscribe_event(name, &input, &mut |_| Ok(())),
                Operation::ObserveProperty => {
                    s.thing.observe_property(name, &input, &mut |_| Ok(()))
                }
                Operation::UnsubscribeEvent => s.thing.unsubscribe_event(name, &input),
                Operation::UnobserveProperty => s.thing.unobserve_property(name, &input),
                _ => Err(CoreError::UnsupportedOperation(format!(
                    "operation {:?} not handled",
                    operation
                ))),
            }
        });
        match result {
            Ok(output) => InboundResponse::new(output, correlation),
            Err(err) => InboundResponse::error(correlation, err),
        }
    }
}

/// Direct-dispatch: lets bindings call dispatch from their own context.
/// Bindings with sync callbacks (zenoh) spawn a draining task that calls this;
/// bindings with async handlers (HTTP/CoAP) call it directly in the route.
#[cfg(feature = "async")]
#[async_trait::async_trait]
impl Dispatch for Servient {
    async fn serve_request(&self, request: InboundRequest) -> InboundResponse {
        self.dispatch(request).await
    }
}
