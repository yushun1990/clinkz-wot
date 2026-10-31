#![cfg(feature = "prepare")]
//! A linked third-party Action capability with actual UDP I/O and the stock
//! HTTP Property participant share one logical Servient publication. This is
//! not an HTTP Action mapping or a simulated native readiness callback.
use clinkz_wot_binding_http::*;
use clinkz_wot_core::{action::*, finite::*, producer::*, *};
use clinkz_wot_foundation::*;
use clinkz_wot_planning::producer_composition::*;
use clinkz_wot_servient::producer_composition::publish_prepared_thing;
use clinkz_wot_servient_finite::*;
use clinkz_wot_td::{
    ValidatedProducerScope,
    semantic::{FormOwner, SemanticScope},
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

#[path = "composed_thing/managed.rs"]
mod managed;

fn budget() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, k| b.with_remaining(k, 1_000_000))
}
fn cx() -> Context<'static> {
    Context::from_waker(Waker::noop())
}
fn now() -> MonotonicInstant {
    MonotonicInstant::new(ClockId::new(1), 0)
}
fn key(slot: u32, generation: Generation) -> ProducerInstanceAuthority {
    ProducerInstanceAuthority::try_new(ThingSlotId::new(SlotIndex::new(slot), generation)).unwrap()
}
fn tcp_addr() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
fn udp_addr() -> SocketAddr {
    UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
fn description(tcp: SocketAddr, udp: SocketAddr) -> Arc<str> {
    Arc::from(serde_json::json!({"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"composed",
        "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"},"unused":{"scheme":"private", "urn:custom:factor":"preserved"}},
        "properties":{"level":{"type":"integer","readOnly":true,"forms":[{"href":format!("http://{tcp}/value")}] }},
        "actions":{"run":{"forms":[{"href":format!("udp://{udp}/run"),"op":"invokeaction"},
            {"href":format!("udp://{udp}/status"),"op":"queryaction"},
            {"href":format!("udp://{udp}/stop"),"op":"cancelaction"}]}}
    }).to_string())
}

fn http_properties(first: SocketAddr, second: SocketAddr) -> Arc<str> {
    Arc::from(
        serde_json::json!({"@context":"https://www.w3.org/2022/wot/td/v1.1", "title":"two listeners",
            "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
            "properties":{
                "left":{"type":"integer","readOnly":true,
                    "forms":[{"href":format!("http://{first}/value")}]},
                "right":{"type":"integer","readOnly":true,
                    "forms":[{"href":format!("http://{second}/value")}]}
            }
        })
        .to_string(),
    )
}

fn http_read(
    owner: &RawProducerServient,
    address: SocketAddr,
    program: HttpProducerProgram,
    instance: ProducerInstanceKey,
    name: &str,
    id: BindingId,
    validation: FiniteValidation,
) -> RawFiniteRuntime<HttpFiniteBinding> {
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_loopback(address).unwrap(),
        program,
        instance,
        clinkz_wot_core::AffordanceTarget::Property(Arc::from(name)),
        100,
    )
    .unwrap();
    RawFiniteRuntime::try_new(
        HttpFiniteBinding::new(config.clone(), id, BindingGeneration::INITIAL),
        config,
        owner,
        RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 0,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        },
        validation,
        Deadline::NONE,
    )
    .ok()
    .unwrap()
}

struct UnselectedParticipant(Rc<Cell<bool>>, ProducerParticipationId, Option<Rc<dyn Any>>);
impl ClosedProducerParticipation for UnselectedParticipant {
    fn participation_id(&self) -> &ProducerParticipationId {
        &self.1
    }
    fn retain_owner(&mut self, owner: Rc<dyn Any>) {
        self.2 = Some(owner);
    }
    fn begin_retire(&mut self) {}
    fn ready_for(&self, _: &RawProducerServient, _: ProducerInstanceKey) -> bool {
        true
    }
    fn ingress_gate(&self) -> &Rc<Cell<bool>> {
        &self.0
    }
    fn retired(&self) -> bool {
        false
    }
}

#[test]
fn publication_never_partially_opens_two_native_gates() {
    let mut owner = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 1,
        behaviors_per_instance: 1,
        target_name_bytes: 64,
    })
    .unwrap();
    let authority = key(1, Generation::INITIAL);
    let instance = RawProducerInstance::new(
        authority,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("level")),
            Operation::ReadProperty,
            Property(Rc::new(Counts::default())),
        )],
    )
    .with_description(Arc::from("closed publication gate test"));
    let mut publication = owner.prepare_publication(instance, None).ok().unwrap();
    let shared = Rc::new(Cell::new(false));
    let mut first = UnselectedParticipant(Rc::clone(&shared), ProducerParticipationId::new(), None);
    let mut second =
        UnselectedParticipant(Rc::clone(&shared), ProducerParticipationId::new(), None);
    assert!(
        owner
            .publish_composed(&mut publication, &mut [&mut first, &mut second])
            .is_err()
    );
    assert!(!publication.is_committed());
    assert!(!shared.get());
    assert_eq!(owner.description(publication.key()), None);
    second.0 = Rc::new(Cell::new(false));
    owner
        .publish_composed(&mut publication, &mut [&mut first, &mut second])
        .unwrap();
    assert!(first.0.get() && second.0.get());
    publication.revoke();
    assert!(!first.0.get() && !second.0.get());
}

// Only this linked Binding interprets its UDP mapping. Each operation has an
// independently prepared route; no generic URL-to-session or request/reply
// Action assumption is installed in the Engine.
struct UdpActionProvider {
    address: SocketAddr,
    identity: (BindingId, BindingGeneration),
}
impl UdpActionProvider {
    fn new(address: SocketAddr) -> Self {
        Self {
            address,
            identity: (BindingId::new(51), BindingGeneration::INITIAL),
        }
    }
    fn compile_route(
        &self,
        input: ProducerOperationInput<'_>,
        _: &mut WorkBudget,
    ) -> CoreResult<Option<ActionRoute>> {
        let FormSite::Action(name) = input.site else {
            return Ok(None);
        };
        if name != "run" {
            return Ok(None);
        }
        let facts = input
            .document
            .form(FormOwner::Action(name), input.form)
            .unwrap();
        if facts.effective_security.len() != 1
            || input
                .document
                .security(&facts.effective_security[0])
                .map_err(|_| unsupported())?
                .scheme
                != "nosec"
            || facts.content_type != "application/json"
            || facts.subprotocol.is_some()
            || facts.content_coding.is_some()
            || facts.scopes.is_some()
            || facts.additional_responses
            || facts.response_content_type.is_some()
            || input
                .document
                .affordance_schema(FormOwner::Action(name), "input")
                .is_some()
            || input
                .document
                .affordance_schema(FormOwner::Action(name), "output")
                .is_some()
            || input.document.action_synchronous(name) == Some(Some(true))
            || input
                .document
                .scope(SemanticScope::AffordanceForm {
                    owner: FormOwner::Action(name),
                    index: input.form,
                })
                .unwrap()
                .children()
                .next()
                .is_some()
            || input
                .document
                .uri_variables(clinkz_wot_td::semantic::UriVariableScope::Thing)
                .is_some()
            || input
                .document
                .uri_variables(clinkz_wot_td::semantic::UriVariableScope::Action)
                .is_some()
        {
            return Err(unsupported());
        }
        let path = match input.operation {
            Operation::InvokeAction => "/run",
            Operation::QueryAction => "/status",
            Operation::CancelAction => "/stop",
            _ => return Ok(None),
        };
        if input.target != format!("udp://{}{path}", self.address) {
            return Err(unsupported());
        }
        Ok(Some(ActionRoute {
            binding: self.identity,
            mapping: ActionMappingKey(input.form as u64 + 1),
        }))
    }
}
impl ProducerCapability for UdpActionProvider {
    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.identity
    }
    fn definition_bytes(&self, definition: &dyn Any) -> Option<usize> {
        definition
            .downcast_ref::<ActionRoute>()
            .map(|_| std::mem::size_of::<ActionRoute>())
    }
    fn prepare(
        &self,
        input: ProducerOperationInput<'_>,
        b: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>> {
        Ok(self
            .compile_route(input, b)?
            .map(|route| Box::new(route) as Box<dyn Any>))
    }
    fn supports(&self, input: ProducerOperationInput<'_>, b: &mut WorkBudget) -> CoreResult<bool> {
        Ok(self.compile_route(input, b)?.is_some())
    }
    fn final_check_temporary_bytes(&self) -> Option<usize> {
        Some(512)
    }
    fn check_final(
        &self,
        input: ProducerOperationInput<'_>,
        definition: &dyn Any,
        b: &mut WorkBudget,
    ) -> CoreResult<bool> {
        let Some(expected) = definition.downcast_ref::<ActionRoute>() else {
            return Ok(false);
        };
        Ok(self.compile_route(input, b)?.as_ref() == Some(expected))
    }
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}

#[test]
fn http_action_registration_does_not_veto_a_different_bindings_action_forms() {
    let tcp = tcp_addr();
    let udp = udp_addr();
    let owner = key(1, Generation::INITIAL);
    let instance = instance(owner, Rc::new(Counts::default()));
    let http_action = HttpActionProducer::new(
        HttpProducerConfig::for_loopback(tcp).unwrap(),
        BindingId::new(60),
        BindingGeneration::INITIAL,
    );
    let http_read = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(tcp).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap();
    let udp_action = UdpActionProvider::new(udp);
    let prepared = prepare_thing(
        &description(tcp, udp),
        instance.key(),
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[&http_action, &http_read, &udp_action],
        |scope, op| match scope {
            ValidatedProducerScope::Property(n) => instance.supports_property(n, op),
            ValidatedProducerScope::Action(n) => instance.supports_action(n, op),
            _ => false,
        },
    )
    .unwrap();
    assert_eq!(prepared.operations().len(), 4);
    assert_eq!(
        prepared
            .operations()
            .iter()
            .filter(|p| p.definition::<ActionRoute>().is_some())
            .count(),
        3
    );
    assert_eq!(
        prepared
            .operations()
            .iter()
            .filter(|p| p.definition::<HttpProducerProgram>().is_some())
            .count(),
        1
    );
}

#[derive(Default)]
struct Counts {
    property: Cell<usize>,
    actions: Cell<usize>,
    cancels: Cell<usize>,
    work_drops: Cell<usize>,
    complete: Cell<bool>,
}
struct Property(Rc<Counts>);
impl RawFiniteHandler for Property {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(
        &self,
        ctx: HandlerContext<'_>,
        _: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        assert_eq!(ctx.operation(), Operation::ReadProperty);
        self.0.property.set(self.0.property.get() + 1);
        Ok(InteractionOutput::with_data(Payload::new(
            b"42".to_vec(),
            "application/json",
        )))
    }
}
struct StringProperty(Rc<Counts>);
impl RawFiniteHandler for StringProperty {
    fn authorize(&self, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn handle(&self, _: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<InteractionOutput> {
        self.0.property.set(self.0.property.get() + 1);
        Ok(InteractionOutput::with_data(Payload::new(
            b"\"wrong type\"".to_vec(),
            "application/json",
        )))
    }
}
struct Action(Rc<Counts>);
impl RawActionHandler for Action {
    fn authorize(&self, _: Operation, _: &InteractionInput) -> CoreResult<()> {
        Ok(())
    }
    fn invoke(&self, ctx: HandlerContext<'_>, _: &InteractionInput) -> CoreResult<RawActionStart> {
        assert_eq!(ctx.operation(), Operation::InvokeAction);
        self.0.actions.set(self.0.actions.get() + 1);
        Ok(RawActionStart::Continue(Box::new(Work(Rc::clone(&self.0)))))
    }
}
struct Work(Rc<Counts>);
impl RawActionWork for Work {
    fn poll(&mut self) -> Poll<CoreResult<InteractionOutput>> {
        if !self.0.complete.get() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(InteractionOutput::with_data(Payload::new(
            b"\"done\"".to_vec(),
            "application/json",
        ))))
    }
    fn request_cancel(&mut self) -> CoreResult<()> {
        self.0.cancels.set(self.0.cancels.get() + 1);
        Ok(())
    }
    fn poll_close(&mut self) -> Poll<CoreResult<()>> {
        Poll::Ready(Ok(()))
    }
    fn abort_local(self: Box<Self>) {
        drop(self);
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        self.0.work_drops.set(self.0.work_drops.get() + 1);
    }
}
fn instance(a: ProducerInstanceAuthority, counts: Rc<Counts>) -> RawProducerInstance {
    RawProducerInstance::new(
        a,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("level")),
            Operation::ReadProperty,
            Property(Rc::clone(&counts)),
        )],
    )
    .with_actions(vec![RawActionBehavior::new(
        Arc::from("run"),
        Action(counts),
        0,
        true,
        true,
    )])
}
fn owner() -> RawProducerServient {
    let mut s = RawProducerServient::try_empty(RawProducerRegistryLimits {
        instances: 2,
        behaviors_per_instance: 4,
        target_name_bytes: 64,
    })
    .unwrap();
    s.enable_raw_actions(RawActionLimits {
        invocations: 4,
        application_work_bytes: 64,
        input_fields: 8,
        input_bytes: 256,
        principal_id_bytes: 32,
        output_body_bytes: 64,
        output_media_capacity: 64,
        output_retained_bytes: 2048,
    })
    .unwrap();
    s
}

// One *native* resource can serve several Things. Each logical participation
// leases it independently; closing one never closes the other's socket.
struct ActionNative {
    socket: Option<UdpSocket>,
    address: SocketAddr,
    leases: Vec<(ProducerInstanceKey, Rc<Cell<bool>>)>,
    handles: Vec<(ProducerInstanceKey, RawActionHandle)>,
}
struct ActionLease {
    native: Rc<RefCell<ActionNative>>,
    key: ProducerInstanceKey,
    open: Rc<Cell<bool>>,
    validator: Rc<dyn ActionValidator>,
    validator_enabled: bool,
    control_routes_enabled: bool,
    closed: bool,
    participation: ProducerParticipationId,
    retained_owner: Option<Rc<dyn Any>>,
}
// This linked, test-only Binding has its own JSON/no-input mapping; even a TD
// without an explicit Action output schema needs a captured media service.
struct UdpJsonValidation;
impl ActionValidator for UdpJsonValidation {
    fn validate_input(&self, input: &InteractionInput) -> CoreResult<()> {
        if input.data.is_none() {
            Ok(())
        } else {
            Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Validate,
                RetryClass::Never,
            )))
        }
    }
    fn validate_output(&self, output: &InteractionOutput) -> CoreResult<()> {
        if output.data().is_some_and(|data| {
            data.content_type == "application/json"
                && data.content_coding.is_none()
                && serde_json::from_slice::<serde_json::Value>(&data.body).is_ok()
        }) {
            Ok(())
        } else {
            Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Validate,
                RetryClass::Never,
            )))
        }
    }
}
impl ActionNative {
    fn new(address: SocketAddr) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            socket: None,
            address,
            leases: vec![],
            handles: vec![],
        }))
    }
    fn start(native: &Rc<RefCell<Self>>) -> std::io::Result<()> {
        let mut n = native.borrow_mut();
        if n.socket.is_none() {
            let s = UdpSocket::bind(n.address)?;
            s.set_nonblocking(true)?;
            n.socket = Some(s);
        }
        Ok(())
    }
    fn lease(native: &Rc<RefCell<Self>>, key: ProducerInstanceKey) -> ActionLease {
        let open = Rc::new(Cell::new(false));
        native.borrow_mut().leases.push((key, Rc::clone(&open)));
        ActionLease {
            native: Rc::clone(native),
            key,
            open,
            validator: Rc::new(UdpJsonValidation),
            validator_enabled: true,
            control_routes_enabled: true,
            closed: false,
            participation: ProducerParticipationId::new(),
            retained_owner: None,
        }
    }
    fn step(
        native: &Rc<RefCell<Self>>,
        owner: &mut RawProducerServient,
        route: [ActionRoute; 3],
        budget: &mut WorkBudget,
    ) -> std::io::Result<bool> {
        let mut n = native.borrow_mut();
        if n.socket.is_none() {
            return Ok(false);
        };
        let mut buf = [0u8; 128];
        let (len, peer) = match n.socket.as_ref().unwrap().recv_from(&mut buf) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(e) => return Err(e),
        };
        let wire = std::str::from_utf8(&buf[..len]).unwrap();
        // The native Binding matches a concrete slot and owns correlation;
        // this numeric wire address alone cannot grant currentness or dispatch.
        let mut fields = wire.split(':');
        let op = fields.next().unwrap();
        let slot = fields.next().unwrap().parse::<u32>().unwrap();
        let generation = fields.next().unwrap().parse::<u32>().unwrap();
        let Some((key, gate)) = n
            .leases
            .iter()
            .find(|(key, _)| {
                key.slot().slot() == SlotIndex::new(slot)
                    && key.slot().generation().get() == generation
            })
            .cloned()
        else {
            n.socket.as_ref().unwrap().send_to(b"unknown", peer)?;
            return Ok(true);
        };
        if !gate.get() {
            n.socket.as_ref().unwrap().send_to(b"closed", peer)?;
            return Ok(true);
        }
        let input = InteractionInput::empty();
        let answer = match op {
            "invoke" => {
                let result = owner.invoke_raw_action(
                    RawActionRequest {
                        instance: key,
                        target: &AffordanceTarget::Action(Arc::from("run")),
                        input: &input,
                        invoke: route[0],
                        query: Some(route[1]),
                        cancel: Some(route[2]),
                    },
                    budget,
                );
                match result {
                    Ok(handle) => {
                        n.handles.push((key, handle));
                        "accepted"
                    }
                    Err(_) => "rejected",
                }
            }
            "query" => match n.handles.iter().rev().find(|(k, _)| *k == key) {
                Some((_, h)) => match owner.query_raw_action(
                    h,
                    route[1].binding,
                    route[1].mapping,
                    &input,
                    budget,
                ) {
                    Ok(view) => match view.state {
                        RawActionState::Running => "running",
                        RawActionState::Complete(result) => {
                            if result.is_ok() {
                                "complete"
                            } else {
                                "failed"
                            }
                        }
                    },
                    Err(_) => "rejected",
                },
                None => "missing",
            },
            "cancel" => match n.handles.iter().rev().find(|(k, _)| *k == key) {
                Some((_, h)) => match owner.cancel_raw_action(
                    h,
                    route[2].binding,
                    route[2].mapping,
                    &input,
                    budget,
                ) {
                    Ok(true) => "cancelling",
                    Ok(false) => "terminal",
                    Err(_) => "rejected",
                },
                None => "missing",
            },
            _ => "unsupported",
        };
        n.socket
            .as_ref()
            .unwrap()
            .send_to(answer.as_bytes(), peer)?;
        Ok(true)
    }
}
impl ClosedProducerParticipation for ActionLease {
    fn participation_id(&self) -> &ProducerParticipationId {
        &self.participation
    }
    fn retain_owner(&mut self, owner: Rc<dyn Any>) {
        self.retained_owner = Some(owner);
    }
    fn admission_backing_bytes(
        &self,
        _: &RawProducerServient,
        key: ProducerInstanceKey,
    ) -> Option<usize> {
        (!self.open.get() && !self.closed && self.retained_owner.is_none() && self.key == key)
            .then_some(
                std::mem::size_of::<Self>()
                    + 4 * std::mem::size_of::<usize>()
                    + std::mem::size_of::<Cell<bool>>(),
            )
    }
    fn begin_retire(&mut self) {
        self.close();
    }
    fn ready_for(&self, _: &RawProducerServient, key: ProducerInstanceKey) -> bool {
        !self.closed
            && self.key == key
            && self
                .native
                .borrow()
                .socket
                .as_ref()
                .and_then(|s| s.local_addr().ok())
                == Some(self.native.borrow().address)
    }
    fn ingress_gate(&self) -> &Rc<Cell<bool>> {
        &self.open
    }
    fn covers(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        operation: Operation,
        definition: &dyn Any,
    ) -> bool {
        let op = match form {
            0 => Operation::InvokeAction,
            1 => Operation::QueryAction,
            2 => Operation::CancelAction,
            _ => return false,
        };
        key == self.key
            && binding == (BindingId::new(51), BindingGeneration::INITIAL)
            && matches!(site, clinkz_wot_core::thing::AffordanceSite::Action("run"))
            && operation == op
            && definition.downcast_ref::<ActionRoute>()
                == Some(&ActionRoute {
                    binding,
                    mapping: ActionMappingKey(form as u64 + 1),
                })
    }
    fn action_validation(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        definition: &dyn Any,
    ) -> Option<(ActionRoute, Rc<dyn ActionValidator>)> {
        (self.validator_enabled
            && self.covers(
                key,
                binding,
                site,
                form,
                Operation::InvokeAction,
                definition,
            ))
        .then(|| {
            (
                *definition.downcast_ref::<ActionRoute>().unwrap(),
                Rc::clone(&self.validator),
            )
        })
    }
    fn action_control_route(
        &self,
        key: ProducerInstanceKey,
        binding: (BindingId, BindingGeneration),
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        operation: Operation,
        definition: &dyn Any,
    ) -> Option<ActionRoute> {
        (self.control_routes_enabled
            && matches!(operation, Operation::QueryAction | Operation::CancelAction)
            && self.covers(key, binding, site, form, operation, definition))
        .then(|| *definition.downcast_ref::<ActionRoute>().unwrap())
    }
    fn retired(&self) -> bool {
        self.closed
    }
}
impl ActionLease {
    fn close(&mut self) {
        self.open.set(false);
        self.closed = true;
    }
}
impl Drop for ActionLease {
    fn drop(&mut self) {
        self.close();
    }
}

fn routes(prepared: &PreparedThing) -> [ActionRoute; 3] {
    let mut routes = [None; 3];
    for op in prepared.operations() {
        if let FormSite::Action(_) = op.site {
            routes[op.form as usize] = op.definition::<ActionRoute>().copied();
        }
    }
    routes.map(Option::unwrap)
}
fn udp_call(
    peer: &UdpSocket,
    native: &Rc<RefCell<ActionNative>>,
    s: &mut RawProducerServient,
    route: [ActionRoute; 3],
    key: ProducerInstanceKey,
    verb: &str,
) -> String {
    peer.send_to(
        format!(
            "{verb}:{}:{}",
            key.slot().slot().get(),
            key.slot().generation().get()
        )
        .as_bytes(),
        native.borrow().address,
    )
    .unwrap();
    let end = Instant::now() + Duration::from_secs(3);
    while !ActionNative::step(native, s, route, &mut budget()).unwrap() {
        assert!(Instant::now() < end);
    }
    let mut buf = [0; 128];
    let (n, _) = peer.recv_from(&mut buf).unwrap();
    std::str::from_utf8(&buf[..n]).unwrap().into()
}
fn read_peer(
    addr: SocketAddr,
    owner: &mut RawProducerServient,
    read: &mut RawFiniteRuntime<HttpFiniteBinding>,
) -> Vec<u8> {
    let mut peer = TcpStream::connect(addr).unwrap();
    peer.set_nonblocking(true).unwrap();
    peer.write_all(
        format!("GET /value HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .unwrap();
    let mut output = vec![];
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let _ = owner.step(read, &mut cx(), &mut budget(), now()).unwrap();
        let mut buf = [0; 256];
        match peer.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => output.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        }
        assert!(Instant::now() < end, "HTTP stalled: {:?}", read.state());
    }
    output
}

fn action_only(udp: SocketAddr) -> Arc<str> {
    let mut value: serde_json::Value = serde_json::from_str(&description(tcp_addr(), udp)).unwrap();
    value.as_object_mut().unwrap().remove("properties");
    Arc::from(value.to_string())
}
fn action_instance(a: ProducerInstanceAuthority, counts: Rc<Counts>) -> RawProducerInstance {
    RawProducerInstance::new(a, vec![]).with_actions(vec![RawActionBehavior::new(
        Arc::from("run"),
        Action(counts),
        0,
        true,
        true,
    )])
}
fn prepare_action(
    s: &RawProducerServient,
    a: ProducerInstanceAuthority,
    text: &Arc<str>,
    provider: &UdpActionProvider,
    counts: Rc<Counts>,
    expected: Option<ProducerInstanceKey>,
) -> (PreparedThing, ProducerPublication) {
    let instance = action_instance(a, counts);
    let k = instance.key();
    let prepared = prepare_thing(
        text,
        k,
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[provider],
        |scope, op| match scope {
            ValidatedProducerScope::Action(n) => instance.supports_action(n, op),
            _ => false,
        },
    )
    .unwrap();
    (
        prepared,
        s.prepare_publication(instance.with_description(Arc::clone(text)), expected)
            .ok()
            .unwrap(),
    )
}

#[test]
fn two_http_registrations_compose_without_opening_a_foreign_runtime() {
    let left_addr = tcp_addr();
    let right_addr = tcp_addr();
    let foreign_addr = tcp_addr();
    let a = key(1, Generation::INITIAL);
    let b = key(2, Generation::INITIAL);
    let ka = a.key();
    let kb = b.key();
    let counts = Rc::new(Counts::default());
    let foreign_counts = Rc::new(Counts::default());
    let foreign = RawProducerInstance::new(
        b,
        vec![RawFiniteBehavior::new(
            AffordanceTarget::Property(Arc::from("left")),
            Operation::ReadProperty,
            Property(Rc::clone(&foreign_counts)),
        )],
    );
    let mut s = RawProducerServient::try_new(
        vec![foreign],
        RawProducerRegistryLimits {
            instances: 2,
            behaviors_per_instance: 4,
            target_name_bytes: 64,
        },
    )
    .ok()
    .unwrap();
    let staged = RawProducerInstance::new(
        a,
        ["left", "right"]
            .into_iter()
            .map(|name| {
                RawFiniteBehavior::new(
                    AffordanceTarget::Property(Arc::from(name)),
                    Operation::ReadProperty,
                    Property(Rc::clone(&counts)),
                )
            })
            .collect(),
    );
    let description = http_properties(left_addr, right_addr);
    let left_id = BindingId::new(101);
    let right_id = BindingId::new(102);
    let left_provider = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(left_addr).unwrap(),
        left_id,
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap();
    let right_provider = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(right_addr).unwrap(),
        right_id,
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap();
    let providers: [&dyn ProducerCapability; 2] = [&left_provider, &right_provider];
    let prepared = prepare_thing(
        &description,
        ka,
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
        &providers,
        |scope, op| match scope {
            ValidatedProducerScope::Property(n) => staged.supports_property(n, op),
            _ => false,
        },
    )
    .unwrap();
    assert_eq!(prepared.operations().len(), 2);
    for operation in prepared.operations() {
        let expected = match &operation.site {
            FormSite::Property(name) if name == "left" => left_id,
            FormSite::Property(name) if name == "right" => right_id,
            _ => panic!("unexpected prepared operation"),
        };
        assert_eq!(operation.binding, (expected, BindingGeneration::INITIAL));
    }
    // A matching registration still rejects unsupported meaning; probing the
    // other listener must not turn that error into a successful fallback.
    let mut unsupported_td: serde_json::Value = serde_json::from_str(&description).unwrap();
    unsupported_td["properties"]["right"]["forms"][0]["contentType"] =
        serde_json::json!("text/plain");
    assert!(
        prepare_thing(
            &unsupported_td.to_string(),
            ka,
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
            &providers,
            |_, _| true,
        )
        .is_err()
    );
    let program = |name: &str| {
        prepared
            .operations()
            .iter()
            .find(|o| o.site == FormSite::Property(name.into()))
            .unwrap()
            .definition::<HttpProducerProgram>()
            .unwrap()
            .clone()
    };
    let mut publication = s
        .prepare_publication(staged.with_description(Arc::clone(&description)), None)
        .ok()
        .unwrap();
    let mut left = http_read(
        &s,
        left_addr,
        program("left"),
        ka,
        "left",
        left_id,
        FiniteValidation::Required,
    );
    let mut right = http_read(
        &s,
        right_addr,
        program("right"),
        ka,
        "right",
        right_id,
        FiniteValidation::Required,
    );

    // B has a live handler and a separate closed listener. It is not part of
    // A's qualified exposure, despite sharing the same Servient owner.
    let mut foreign_td: serde_json::Value =
        serde_json::from_str(&http_properties(foreign_addr, right_addr)).unwrap();
    foreign_td["properties"]
        .as_object_mut()
        .unwrap()
        .remove("right");
    let foreign_td = foreign_td.to_string();
    let foreign_id = BindingId::new(103);
    let foreign_provider = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(foreign_addr).unwrap(),
        foreign_id,
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap();
    let foreign_prepared = prepare_thing(
        &foreign_td,
        kb,
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[&foreign_provider],
        |scope, op| {
            matches!(scope, ValidatedProducerScope::Property("left"))
                && op == Operation::ReadProperty
        },
    )
    .unwrap();
    let mut unrelated = http_read(
        &s,
        foreign_addr,
        foreign_prepared.operations()[0]
            .definition::<HttpProducerProgram>()
            .unwrap()
            .clone(),
        kb,
        "left",
        foreign_id,
        FiniteValidation::Required,
    );
    for runtime in [&mut left, &mut right, &mut unrelated] {
        assert_eq!(
            runtime.step(&mut cx(), &mut budget(), now()),
            Poll::Ready(RawFiniteState::InstalledClosed)
        );
    }
    assert!(!unrelated.ready_for(&s, ka));
    // Even the lower-level publication entry cannot open B for A.
    assert!(
        s.publish_composed(&mut publication, &mut [&mut left, &mut unrelated])
            .is_err()
    );
    assert_eq!(unrelated.state(), RawFiniteState::InstalledClosed);
    assert!(
        publish_prepared_thing(
            &mut s,
            &mut publication,
            &prepared,
            &providers,
            &mut [&mut left, &mut right, &mut unrelated],
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
        )
        .is_err()
    );
    assert!(!publication.is_committed());
    assert_eq!(s.description(ka), None);
    assert_eq!(unrelated.state(), RawFiniteState::InstalledClosed);
    assert_eq!(left.state(), RawFiniteState::InstalledClosed);
    assert_eq!(right.state(), RawFiniteState::InstalledClosed);
    publish_prepared_thing(
        &mut s,
        &mut publication,
        &prepared,
        &providers,
        &mut [&mut left, &mut right],
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
    )
    .unwrap();
    assert!(read_peer(left_addr, &mut s, &mut left).starts_with(b"HTTP/1.1 200"));
    assert!(read_peer(right_addr, &mut s, &mut right).starts_with(b"HTTP/1.1 200"));
    assert_eq!(counts.property.get(), 2);
    assert_eq!(foreign_counts.property.get(), 0);
    assert_eq!(unrelated.state(), RawFiniteState::InstalledClosed);
    for runtime in [&mut left, &mut right] {
        assert_eq!(
            runtime.take_settlement().unwrap().delivery,
            FiniteDelivery::Delivered
        );
    }
    for runtime in [&mut left, &mut right, &mut unrelated] {
        runtime.begin_close();
        while runtime.state() != RawFiniteState::Closed {
            let _ = runtime.step(&mut cx(), &mut budget(), now());
        }
    }
    assert!(s.retire_composed(&mut publication, &[&left]).is_err());
    assert!(
        s.retire_composed(&mut publication, &[&left, &left])
            .is_err()
    );
    assert!(
        s.retire_composed(&mut publication, &[&left, &unrelated])
            .is_err()
    );
    s.retire_composed(&mut publication, &[&left, &right])
        .unwrap();
    drop(left);
    drop(right);
    drop(unrelated);
    for addr in [left_addr, right_addr, foreign_addr] {
        TcpListener::bind(addr).unwrap();
    }
}

#[test]
fn qualified_publication_requires_validation_and_rejects_invalid_output() {
    for validation in [FiniteValidation::Required, FiniteValidation::Raw] {
        let addr = tcp_addr();
        let description: Arc<str> = Arc::from(
            serde_json::json!({
                "@context":"https://www.w3.org/2022/wot/td/v1.1",
                "title":"integer output",
                "security":["n"],"securityDefinitions":{"n":{"scheme":"nosec"}},
                "properties":{"left":{"type":"integer","readOnly":true,
                    "forms":[{"href":format!("http://{addr}/value")}]}}
            })
            .to_string(),
        );
        let counts = Rc::new(Counts::default());
        let authority = key(1, Generation::INITIAL);
        let k = authority.key();
        let staged = RawProducerInstance::new(
            authority,
            vec![RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("left")),
                Operation::ReadProperty,
                StringProperty(Rc::clone(&counts)),
            )],
        );
        let mut s = owner();
        let id = BindingId::new(7);
        let provider = HttpFiniteProducer::try_new(
            HttpProducerConfig::for_loopback(addr).unwrap(),
            id,
            BindingGeneration::INITIAL,
            100,
        )
        .unwrap();
        let prepared = prepare_thing(
            &description,
            k,
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
            &[&provider],
            |scope, op| match scope {
                ValidatedProducerScope::Property(n) => staged.supports_property(n, op),
                _ => false,
            },
        )
        .unwrap();
        let program = prepared.operations()[0]
            .definition::<HttpProducerProgram>()
            .unwrap()
            .clone();
        let mut publication = s
            .prepare_publication(staged.with_description(Arc::clone(&description)), None)
            .ok()
            .unwrap();
        let mut runtime = http_read(&s, addr, program, k, "left", id, validation);
        publication.bind_participants(&[&runtime]).unwrap();
        assert_eq!(
            runtime.step(&mut cx(), &mut budget(), now()),
            Poll::Ready(RawFiniteState::InstalledClosed)
        );
        // Both modes have the actual validator; its mere presence cannot
        // qualify a runtime whose dispatch policy skips that service.
        assert!(runtime.closed_binding().unwrap().validator().is_some());
        let published = publish_prepared_thing(
            &mut s,
            &mut publication,
            &prepared,
            &[&provider],
            &mut [&mut runtime],
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
        );
        if validation == FiniteValidation::Raw {
            assert!(
                matches!(published, Err(CoreError::Validation(_))),
                "{published:?}"
            );
            assert!(
                s.publish_composed(&mut publication, &mut [&mut runtime])
                    .is_err()
            );
            assert!(!publication.is_committed());
            assert_eq!(s.description(k), None);
            assert_eq!(runtime.state(), RawFiniteState::InstalledClosed);
            assert_eq!(counts.property.get(), 0);
            assert!(runtime.take_settlement().is_none());
        } else {
            published.unwrap();
            let response = read_peer(addr, &mut s, &mut runtime);
            assert!(response.starts_with(b"HTTP/1.1 500"));
            assert!(response.ends_with(b"\r\n\r\nnull"));
            assert_eq!(counts.property.get(), 1);
            let settlement = runtime.take_settlement().unwrap();
            assert_eq!(settlement.recipient, Some(k));
            assert_eq!(settlement.effect, FiniteEffect::Unknown);
            assert_eq!(settlement.delivery, FiniteDelivery::Delivered);
            assert!(matches!(settlement.result, Err(CoreError::Validation(_))));
        }
        runtime.begin_close();
        while runtime.state() != RawFiniteState::Closed {
            let _ = runtime.step(&mut cx(), &mut budget(), now());
        }
        s.retire_composed(&mut publication, &[&runtime]).unwrap();
        assert_eq!(s.description(k), None);
        drop(runtime);
        TcpListener::bind(addr).unwrap();
    }
}

#[test]
fn complete_thing_ready_independently_publishes_once_and_retains_continuing_work() {
    let tcp = tcp_addr();
    let udp = udp_addr();
    let text = description(tcp, udp);
    let counts = Rc::new(Counts::default());
    let a = key(1, Generation::INITIAL);
    let k = a.key();
    let mut s = owner();
    let native = ActionNative::new(udp);
    let http = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(tcp).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap();
    let action = UdpActionProvider::new(udp);
    let providers: [&dyn ProducerCapability; 2] = [&http, &action];
    let staged = instance(a, Rc::clone(&counts));
    let prepared = prepare_thing(
        &text,
        k,
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
        &providers,
        |scope, op| match scope {
            ValidatedProducerScope::Property(n) => staged.supports_property(n, op),
            ValidatedProducerScope::Action(n) => staged.supports_action(n, op),
            _ => false,
        },
    )
    .unwrap();
    assert_eq!(prepared.operations().len(), 4);
    let program = prepared.operations()[0]
        .definition::<HttpProducerProgram>()
        .unwrap();
    let config = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_loopback(tcp).unwrap(),
        program.clone(),
        k,
        clinkz_wot_core::AffordanceTarget::Property(Arc::from("level")),
        100,
    )
    .unwrap();
    let binding = HttpFiniteBinding::new(
        config.clone(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
    );
    let mut read = RawFiniteRuntime::try_new(
        binding,
        config,
        &s,
        RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 0,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        },
        FiniteValidation::Required,
        Deadline::NONE,
    )
    .ok()
    .unwrap();
    let mut publication = s
        .prepare_publication(staged.with_description(Arc::clone(&text)), None)
        .ok()
        .unwrap();
    let mut lease = ActionNative::lease(&native, k);
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    ActionNative::start(&native).unwrap();
    // Neither native participant can grant dispatch before the local commit.
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&prepared), k, "invoke"),
        "closed"
    );
    assert_eq!(counts.actions.get(), 0);
    assert!(
        publish_prepared_thing(
            &mut s,
            &mut publication,
            &prepared,
            &providers,
            &mut [&mut read, &mut lease],
            &GatewayDefaultV1::LIMITS,
            &mut budget()
        )
        .is_err()
    );
    assert_eq!(s.description(k), None);
    assert_eq!(
        read.step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    // Checking every advertised Form is not enough: a staged handler without
    // a matching Form could otherwise be reached through native ingress.
    let hidden = RawProducerInstance::new(
        key(1, Generation::INITIAL),
        vec![
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("level")),
                Operation::ReadProperty,
                Property(Rc::clone(&counts)),
            ),
            RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::from("hidden")),
                Operation::ReadProperty,
                Property(Rc::clone(&counts)),
            ),
        ],
    )
    .with_actions(vec![RawActionBehavior::new(
        Arc::from("run"),
        Action(Rc::clone(&counts)),
        0,
        true,
        true,
    )])
    .with_description(Arc::clone(&text));
    let mut hidden_publication = s.prepare_publication(hidden, None).ok().unwrap();
    assert!(
        publish_prepared_thing(
            &mut s,
            &mut hidden_publication,
            &prepared,
            &providers,
            &mut [&mut read, &mut lease],
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
        )
        .is_err()
    );
    assert!(!hidden_publication.is_committed());
    assert_eq!(s.description(k), None);
    assert!(
        publish_prepared_thing(
            &mut s,
            &mut publication,
            &prepared,
            &providers,
            &mut [&mut read],
            &GatewayDefaultV1::LIMITS,
            &mut budget()
        )
        .is_err()
    );
    // A participant that claims readiness but covers no selected Form must
    // not be opened, even when every advertised operation has an owner.
    let mut extra = UnselectedParticipant(
        Rc::new(Cell::new(false)),
        ProducerParticipationId::new(),
        None,
    );
    assert!(
        publish_prepared_thing(
            &mut s,
            &mut publication,
            &prepared,
            &providers,
            &mut [&mut read, &mut lease, &mut extra],
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
        )
        .is_err()
    );
    assert!(!extra.0.get());
    assert!(!lease.open.get());
    assert!(!publication.is_committed());
    assert_eq!(read.state(), RawFiniteState::InstalledClosed);
    assert_eq!(s.description(k), None);
    // The HTTP port is live but semantic ingress remains closed.
    let mut waiting = TcpStream::connect(tcp).unwrap();
    waiting.set_nonblocking(true).unwrap();
    assert!(
        publish_prepared_thing(
            &mut s,
            &mut publication,
            &prepared,
            &providers,
            &mut [&mut read, &mut lease],
            &GatewayDefaultV1::LIMITS,
            &mut budget()
        )
        .is_ok()
    );
    publication.release_retired();
    assert_eq!(s.description(k), Some(text.as_ref()));
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&prepared), k, "invoke"),
        "accepted"
    );
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&prepared), k, "query"),
        "running"
    );
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&prepared), k, "cancel"),
        "cancelling"
    );
    assert_eq!(counts.property.get(), 0);
    // A query and cancellation occurred over native I/O after the initiating
    // reply, without treating the Action continuation as a finite exchange.
    let handle = native.borrow().handles[0].1.clone();
    assert_eq!(
        s.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Pending
    );
    assert_eq!(counts.cancels.get(), 1);
    let mut unused = [0; 1];
    assert_eq!(
        waiting.read(&mut unused).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(waiting);
    assert!(read_peer(tcp, &mut s, &mut read).starts_with(b"HTTP/1.1 200"));
    assert_eq!(counts.property.get(), 1);
    assert_eq!(
        read.take_settlement().unwrap().delivery,
        FiniteDelivery::Delivered
    );
    counts.complete.set(true);
    assert_eq!(
        s.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&prepared), k, "query"),
        "complete"
    );
    let result = s.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert_eq!(result.instance, k);
    assert_eq!(
        result.result.unwrap().data().unwrap().body.as_ref(),
        b"\"done\""
    );
    assert_eq!(counts.work_drops.get(), 1);
    counts.complete.set(false);
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&prepared), k, "invoke"),
        "accepted"
    );
    let late = native.borrow().handles.last().unwrap().1.clone();
    // An already accepted finite exchange keeps its reply opportunity when
    // the publication gate closes; only a *new* receive must stop.
    let mut in_flight = TcpStream::connect(tcp).unwrap();
    in_flight.set_nonblocking(true).unwrap();
    in_flight
        .write_all(
            format!("GET /value HTTP/1.1\r\nHost: {tcp}\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    while read.state() != RawFiniteState::Delivering {
        let _ = s.step(&mut read, &mut cx(), &mut budget(), now()).unwrap();
        assert!(Instant::now() < end, "finite ingress stalled");
    }
    assert_eq!(counts.property.get(), 2);
    publication.revoke();
    while read.state() != RawFiniteState::Serving {
        let _ = s.step(&mut read, &mut cx(), &mut budget(), now()).unwrap();
        assert!(Instant::now() < end, "finite reply stalled");
    }
    let mut reply = Vec::new();
    loop {
        let mut buf = [0; 256];
        match in_flight.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => reply.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("{e}"),
        }
        assert!(Instant::now() < end, "finite peer stalled");
    }
    assert!(reply.starts_with(b"HTTP/1.1 200"));
    assert_eq!(
        read.take_settlement().unwrap().delivery,
        FiniteDelivery::Delivered
    );
    // This finite participant has already served a real request and returned
    // to Serving. Revocation must fence a new TCP peer before native receive,
    // rather than offering it as stale semantic ingress to the Servient.
    let mut after_revoke = TcpStream::connect(tcp).unwrap();
    after_revoke.set_nonblocking(true).unwrap();
    after_revoke
        .write_all(
            format!("GET /value HTTP/1.1\r\nHost: {tcp}\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .unwrap();
    for _ in 0..4 {
        assert_eq!(
            s.step(&mut read, &mut cx(), &mut budget(), now()).unwrap(),
            Poll::Pending
        );
    }
    assert_eq!(read.state(), RawFiniteState::Serving);
    assert_eq!(counts.property.get(), 2);
    assert!(read.take_settlement().is_none());
    assert_eq!(
        after_revoke.read(&mut unused).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(after_revoke);
    assert_eq!(
        s.step_raw_action(&late, &mut budget()).unwrap(),
        Poll::Pending
    );
    assert_eq!(counts.cancels.get(), 2); // explicit cancel + guard revocation
    counts.complete.set(true);
    assert_eq!(
        s.step_raw_action(&late, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    let result = s.take_raw_action_settlement(&late).unwrap().unwrap();
    assert!(result.cancel_requested);
    assert_eq!(
        result.result.unwrap().data().unwrap().body.as_ref(),
        b"\"done\""
    );
    assert_eq!(counts.work_drops.get(), 2);
    lease.close();
    read.begin_close();
    while read.state() != RawFiniteState::Closed {
        let _ = read.step(&mut cx(), &mut budget(), now());
    }
    s.retire_composed(&mut publication, &[&read, &lease])
        .unwrap();
    drop(read);
    drop(lease);
    drop(native);
    assert_eq!(s.description(k), None);
    TcpListener::bind(tcp).unwrap();
    UdpSocket::bind(udp).unwrap();
}

#[test]
fn lost_publication_guard_requests_cancellation_on_next_funded_action_step() {
    let udp = udp_addr();
    let native = ActionNative::new(udp);
    ActionNative::start(&native).unwrap();
    let provider = UdpActionProvider::new(udp);
    let text = action_only(udp);
    let counts = Rc::new(Counts::default());
    let mut s = owner();
    let a = key(1, Generation::INITIAL);
    let k = a.key();
    let (prepared, mut publication) =
        prepare_action(&s, a, &text, &provider, Rc::clone(&counts), None);
    let mut lease = ActionNative::lease(&native, k);
    publish_prepared_thing(
        &mut s,
        &mut publication,
        &prepared,
        &[&provider],
        &mut [&mut lease],
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
    )
    .unwrap();
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&prepared), k, "invoke"),
        "accepted"
    );
    let handle = native.borrow().handles[0].1.clone();
    drop(publication);
    assert_eq!(s.description(k), None);
    assert_eq!(counts.cancels.get(), 0);
    assert_eq!(
        s.step_raw_action(&handle, &mut WorkBudget::new()).unwrap(),
        Poll::Pending
    );
    assert_eq!(counts.cancels.get(), 0);
    assert_eq!(
        s.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Pending
    );
    assert_eq!(counts.cancels.get(), 1);
    assert_eq!(
        s.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Pending
    );
    assert_eq!(counts.cancels.get(), 1);
    assert_eq!(counts.work_drops.get(), 0);
    assert!(s.take_raw_action_settlement(&handle).unwrap().is_none());
    counts.complete.set(true);
    assert_eq!(
        s.step_raw_action(&handle, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    let result = s.take_raw_action_settlement(&handle).unwrap().unwrap();
    assert_eq!(result.instance, k);
    assert!(result.cancel_requested);
    assert_eq!(result.effect, ActionEffect::PossiblyApplied);
    assert_eq!(counts.cancels.get(), 1);
    assert_eq!(counts.work_drops.get(), 1);
    assert_eq!(
        result.result.unwrap().data().unwrap().body.as_ref(),
        b"\"done\""
    );
    lease.close();
    drop(lease);
    drop(s);
    drop(native);
    UdpSocket::bind(udp).unwrap();
}

#[test]
fn shared_native_socket_serves_two_things_and_replacement_does_not_retarget_old_work() {
    let udp = udp_addr();
    let native = ActionNative::new(udp);
    ActionNative::start(&native).unwrap();
    let provider = UdpActionProvider::new(udp);
    let text = action_only(udp);
    let mut s = owner();
    let first = key(1, Generation::INITIAL);
    let sibling = key(2, Generation::INITIAL);
    let k1 = first.key();
    let k2 = sibling.key();
    let old = Rc::new(Counts::default());
    let other = Rc::new(Counts::default());
    let (p1, mut pub1) = prepare_action(&s, first, &text, &provider, Rc::clone(&old), None);
    let (p2, mut pub2) = prepare_action(&s, sibling, &text, &provider, Rc::clone(&other), None);
    let mut l1 = ActionNative::lease(&native, k1);
    let mut l2 = ActionNative::lease(&native, k2);
    publish_prepared_thing(
        &mut s,
        &mut pub1,
        &p1,
        &[&provider],
        &mut [&mut l1],
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
    )
    .unwrap();
    publish_prepared_thing(
        &mut s,
        &mut pub2,
        &p2,
        &[&provider],
        &mut [&mut l2],
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
    )
    .unwrap();
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&p1), k1, "invoke"),
        "accepted"
    );
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&p2), k2, "invoke"),
        "accepted"
    );
    let h1 = native.borrow().handles[0].1.clone();
    let h2 = native.borrow().handles[1].1.clone();
    let next = key(1, Generation::INITIAL.checked_next().unwrap());
    let k3 = next.key();
    let newer = Rc::new(Counts::default());
    let (p3, mut pub3) = prepare_action(&s, next, &text, &provider, Rc::clone(&newer), Some(k1));
    let mut l3 = ActionNative::lease(&native, k3);
    publish_prepared_thing(
        &mut s,
        &mut pub3,
        &p3,
        &[&provider],
        &mut [&mut l3],
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
    )
    .unwrap();
    pub3.release_retired();
    assert_eq!(s.description(k1), None);
    assert_eq!(s.description(k2), Some(text.as_ref()));
    assert_eq!(s.description(k3), Some(text.as_ref()));
    // Old native correlation and late work keep their original key. Numeric
    // slot reuse grants no fresh query/cancel or new invocation authority.
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&p1), k1, "query"),
        "rejected"
    );
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&p1), k1, "invoke"),
        "rejected"
    );
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&p3), k3, "invoke"),
        "accepted"
    );
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&p2), k2, "query"),
        "running"
    );
    assert_eq!(old.actions.get(), 1);
    assert_eq!(other.actions.get(), 1);
    assert_eq!(newer.actions.get(), 1);
    assert_eq!(old.cancels.get(), 0); // no callback during publication
    assert_eq!(
        s.step_raw_action(&h1, &mut budget()).unwrap(),
        Poll::Pending
    );
    assert_eq!(old.cancels.get(), 1); // displaced pending Action
    assert_eq!(other.cancels.get(), 0);
    assert_eq!(newer.cancels.get(), 0);
    old.complete.set(true);
    other.complete.set(true);
    newer.complete.set(true);
    assert_eq!(
        s.step_raw_action(&h1, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert_eq!(
        s.step_raw_action(&h2, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    let old_result = s.take_raw_action_settlement(&h1).unwrap().unwrap();
    assert_eq!(old_result.instance, k1);
    assert!(old_result.cancel_requested);
    assert_eq!(
        old_result.result.unwrap().data().unwrap().body.as_ref(),
        b"\"done\""
    );
    assert_eq!(
        s.take_raw_action_settlement(&h2).unwrap().unwrap().instance,
        k2
    );
    let h3 = native.borrow().handles.last().unwrap().1.clone();
    assert_eq!(
        s.step_raw_action(&h3, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert_eq!(
        s.take_raw_action_settlement(&h3).unwrap().unwrap().instance,
        k3
    );
    assert_eq!(old.cancels.get(), 1);
    assert_eq!(other.cancels.get(), 0);
    assert_eq!(newer.cancels.get(), 0);
    l1.close();
    s.retire_composed(&mut pub1, &[&l1]).unwrap();
    assert_eq!(
        udp_call(&peer, &native, &mut s, routes(&p3), k3, "invoke"),
        "accepted"
    );
    let h4 = native.borrow().handles.last().unwrap().1.clone();
    assert_eq!(
        s.step_raw_action(&h4, &mut budget()).unwrap(),
        Poll::Ready(())
    );
    assert_eq!(
        s.take_raw_action_settlement(&h4).unwrap().unwrap().instance,
        k3
    );
    assert_eq!(newer.cancels.get(), 0); // retiring the old guard cannot cancel its successor
    l2.close();
    l3.close();
    s.retire_composed(&mut pub2, &[&l2]).unwrap();
    s.retire_composed(&mut pub3, &[&l3]).unwrap();
    drop(l1);
    drop(l2);
    drop(l3);
    drop(native);
    UdpSocket::bind(udp).unwrap();
}

#[test]
fn partial_native_failure_and_cancelled_prepublication_retire_without_dispatch() {
    let tcp = tcp_addr();
    let udp = udp_addr();
    let occupied = UdpSocket::bind(udp).unwrap();
    let native = ActionNative::new(udp);
    let mut s = owner();
    let a = key(1, Generation::INITIAL);
    let k = a.key();
    let provider = UdpActionProvider::new(udp);
    let text = action_only(udp);
    let (prepared, mut pub1) =
        prepare_action(&s, a, &text, &provider, Rc::new(Counts::default()), None);
    // Another participant can be installed even if a later Binding fails.
    let http = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(tcp).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap();
    let document = description(tcp, udp);
    let app = Rc::new(Counts::default());
    let staged = instance(key(2, Generation::INITIAL), Rc::clone(&app));
    let k2 = staged.key();
    let combined = prepare_thing(
        &document,
        k2,
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
        &[&http, &provider],
        |scope, op| match scope {
            ValidatedProducerScope::Property(n) => staged.supports_property(n, op),
            ValidatedProducerScope::Action(n) => staged.supports_action(n, op),
            _ => false,
        },
    )
    .unwrap();
    let program = combined.operations()[0]
        .definition::<HttpProducerProgram>()
        .unwrap()
        .clone();
    let cfg = HttpFiniteConfig::try_new(
        HttpProducerConfig::for_loopback(tcp).unwrap(),
        program,
        k2,
        clinkz_wot_core::AffordanceTarget::Property(Arc::from("level")),
        100,
    )
    .unwrap();
    let mut read = RawFiniteRuntime::try_new(
        HttpFiniteBinding::new(cfg.clone(), BindingId::new(7), BindingGeneration::INITIAL),
        cfg,
        &s,
        RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 0,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        },
        FiniteValidation::Required,
        Deadline::NONE,
    )
    .ok()
    .unwrap();
    let mut pub2 = s
        .prepare_publication(staged.with_description(document), None)
        .ok()
        .unwrap();
    assert_eq!(
        read.step(&mut cx(), &mut budget(), now()),
        Poll::Ready(RawFiniteState::InstalledClosed)
    );
    let mut lease = ActionNative::lease(&native, k2);
    pub2.bind_participants(&[&read, &lease]).unwrap();
    assert!(ActionNative::start(&native).is_err());
    assert!(
        publish_prepared_thing(
            &mut s,
            &mut pub2,
            &combined,
            &[&http, &provider],
            &mut [&mut read, &mut lease],
            &GatewayDefaultV1::LIMITS,
            &mut budget()
        )
        .is_err()
    );
    assert_eq!(s.description(k2), None);
    assert_eq!(app.actions.get(), 0);
    assert_eq!(app.property.get(), 0);
    drop(occupied);
    ActionNative::start(&native).unwrap();
    pub2.revoke(); // cancellation wins after both independent native setups
    assert!(
        publish_prepared_thing(
            &mut s,
            &mut pub2,
            &combined,
            &[&http, &provider],
            &mut [&mut read, &mut lease],
            &GatewayDefaultV1::LIMITS,
            &mut budget()
        )
        .is_err()
    );
    let mut lease1 = ActionNative::lease(&native, k);
    assert!(
        publish_prepared_thing(
            &mut s,
            &mut pub1,
            &prepared,
            &[&provider],
            &mut [&mut lease1],
            &GatewayDefaultV1::LIMITS,
            &mut budget()
        )
        .is_ok()
    );
    assert_eq!(s.description(k), Some(text.as_ref()));
    lease1.close();
    s.retire_composed(&mut pub1, &[&lease1]).unwrap();
    lease.close();
    read.begin_close();
    while read.state() != RawFiniteState::Closed {
        let _ = read.step(&mut cx(), &mut budget(), now());
    }
    s.retire_composed(&mut pub2, &[&read, &lease]).unwrap();
    drop(read);
    drop(lease);
    drop(lease1);
    drop(native);
    TcpListener::bind(tcp).unwrap();
    UdpSocket::bind(udp).unwrap();
}

#[test]
fn unselected_extension_security_is_preserved_but_selected_extension_refuses_before_io() {
    let tcp = tcp_addr();
    let udp = udp_addr();
    let mut value: serde_json::Value = serde_json::from_str(&description(tcp, udp)).unwrap();
    let http = HttpFiniteProducer::try_new(
        HttpProducerConfig::for_loopback(tcp).unwrap(),
        BindingId::new(7),
        BindingGeneration::INITIAL,
        100,
    )
    .unwrap();
    let action = UdpActionProvider::new(udp);
    let providers: [&dyn ProducerCapability; 2] = [&http, &action];
    let k = key(1, Generation::INITIAL).key();
    let supported = prepare_thing(
        &value.to_string(),
        k,
        &GatewayDefaultV1::LIMITS,
        &mut budget(),
        &providers,
        |scope, _| {
            !matches!(
                scope,
                ValidatedProducerScope::Thing | ValidatedProducerScope::Event(_)
            )
        },
    );
    assert!(supported.is_ok());
    value["security"] = serde_json::json!(["unused"]);
    assert!(
        prepare_thing(
            &value.to_string(),
            k,
            &GatewayDefaultV1::LIMITS,
            &mut budget(),
            &providers,
            |scope, _| !matches!(
                scope,
                ValidatedProducerScope::Thing | ValidatedProducerScope::Event(_)
            ),
        )
        .is_err()
    );
    TcpListener::bind(tcp).unwrap();
    UdpSocket::bind(udp).unwrap();
}
