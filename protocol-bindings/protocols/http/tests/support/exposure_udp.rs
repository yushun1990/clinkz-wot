//! Independently registered, test-only datagram Read on one deployment-owned socket.
use super::*;
use clinkz_wot_core::data_schema::{PreparedSchema, ValueType};
use clinkz_wot_planning::{producer_composition::*, type_schema::prepare_type_schema};
use clinkz_wot_td::semantic::{SemanticScope, UriVariableScope};
use std::{any::Any, task::Poll};

#[derive(Clone)]
pub struct Datagram {
    receive_bytes: usize,
    pub enabled: Rc<RefCell<PublicationState<bool>>>,
    socket: Rc<UdpSocket>,
    leased: Rc<Cell<bool>>,
}
impl Datagram {
    pub fn new() -> Self {
        Self::with_receive_bytes(16)
    }
    pub fn with_receive_bytes(receive_bytes: usize) -> Self {
        let socket = UdpSocket::bind(zero()).unwrap();
        socket.set_nonblocking(true).unwrap();
        Self {
            receive_bytes,
            enabled: Rc::new(RefCell::new(PublicationState::new(true))),
            socket: Rc::new(socket),
            leased: Rc::new(Cell::new(false)),
        }
    }
    pub fn address(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }
    fn matches(&self, input: ProducerOperationInput<'_>) -> CoreResult<bool> {
        if !*self.enabled.borrow().get()
            || input.site.site() != AffordanceSite::Property("absolute")
            || input.operation != Operation::ReadProperty
            || input.form != 0
            || input.target != format!("udp://{}/read", self.address())
        {
            return Ok(false);
        }
        let form = input
            .document
            .form(input.site.owner(), input.form)
            .ok_or_else(refused)?;
        let form_extensions = input
            .document
            .scope(SemanticScope::Form {
                property: "absolute",
                index: input.form,
            })
            .ok_or_else(refused)?;
        Ok(form_extensions.children().next().is_none()
            && input
                .document
                .uri_variables(UriVariableScope::Thing)
                .is_none()
            && input
                .document
                .uri_variables(UriVariableScope::Property)
                .is_none()
            && !input.document.property().schema.facts().write_only
            && form.content_type == "application/json"
            && form.subprotocol.is_none()
            && form.content_coding.is_none()
            && !form.additional_responses
            && form.response_content_type.is_none()
            && form.scopes.is_none()
            && form.effective_security.len() == 1
            && input
                .document
                .security(&form.effective_security[0])
                .map_err(|_| refused())?
                .scheme
                == "nosec"
            && prepare_type_schema(input.document.property().schema)?
                == PreparedSchema::Type(ValueType::Integer))
    }
}
fn refused() -> CoreError {
    CoreError::UnsupportedOperation(ErrorContext::new(ErrorPhase::Binding, RetryClass::Never))
}
impl ProducerCapability for Datagram {
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(70), BindingGeneration::INITIAL)
    }
    fn observe_configuration(&self, facts: &mut PublicationObserver<'_>) -> CoreResult<()> {
        self.enabled
            .try_borrow()
            .map_err(|_| {
                CoreError::Backpressure(ErrorContext::new(ErrorPhase::Prepare, RetryClass::Safe))
            })?
            .observe(facts)
    }
    fn definition_bytes(&self, d: &dyn Any) -> Option<usize> {
        d.downcast_ref::<SocketAddr>().map(size_of_val)
    }
    fn final_check_temporary_bytes(&self) -> Option<usize> {
        Some(4096)
    }
    fn prepare(
        &self,
        i: ProducerOperationInput<'_>,
        _: &mut WorkBudget,
    ) -> CoreResult<Option<Box<dyn Any>>> {
        Ok(self
            .matches(i)?
            .then(|| Box::new(self.address()) as Box<dyn Any>))
    }
    fn supports(&self, i: ProducerOperationInput<'_>, budget: &mut WorkBudget) -> CoreResult<bool> {
        budget.consume(WorkClass::ProviderProbes, 1).map_err(|_| {
            CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
        })?;
        self.matches(i)
    }
    fn check_final(
        &self,
        i: ProducerOperationInput<'_>,
        d: &dyn Any,
        _: &mut WorkBudget,
    ) -> CoreResult<bool> {
        Ok(d.downcast_ref::<SocketAddr>() == Some(&self.address()) && self.matches(i)?)
    }
}
impl FiniteProducerService for Datagram {
    type Binding = Native;
    fn setup_peak_bytes(&self) -> usize {
        4096 + self.receive_bytes
    }
    fn assemble<'a>(
        &self,
        key: ProducerInstanceKey,
        mut ops: impl Iterator<Item = &'a PreparedOperation>,
    ) -> CoreResult<(Native, (ProducerInstanceKey, SocketAddr))> {
        let address = *ops
            .next()
            .and_then(|o| o.definition::<SocketAddr>())
            .ok_or_else(refused)?;
        if address != self.address() || ops.next().is_some() {
            return Err(refused());
        }
        let config = (key, address);
        Ok((
            Native {
                receive: vec![0; self.receive_bytes].into_boxed_slice(),
                config,
                socket: self.socket.clone(),
                lease: self.leased.clone(),
                installed: false,
            },
            config,
        ))
    }
}

pub struct Native {
    receive: Box<[u8]>,
    config: (ProducerInstanceKey, SocketAddr),
    socket: Rc<UdpSocket>,
    lease: Rc<Cell<bool>>,
    installed: bool,
}
impl Native {
    fn release(&mut self) {
        if self.installed {
            self.installed = false;
            self.lease.set(false);
        }
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        self.release();
    }
}
impl FiniteIngressBinding for Native {
    type Reply = SocketAddr;
    type Configuration = (ProducerInstanceKey, SocketAddr);
    fn identity(&self) -> (BindingId, BindingGeneration) {
        (BindingId::new(70), BindingGeneration::INITIAL)
    }
    fn configuration(&self) -> &Self::Configuration {
        &self.config
    }
    fn engine_backing_bytes(&self) -> Option<usize> {
        64usize.checked_add(self.receive.len())
    }
    fn qualified_operation_count(&self) -> Option<usize> {
        Some(1)
    }
    fn prepared_for_instance(&self, key: ProducerInstanceKey) -> bool {
        self.config.0 == key
    }
    fn covers_definition(
        &self,
        key: ProducerInstanceKey,
        site: AffordanceSite<'_>,
        form: u32,
        op: Operation,
        d: &dyn Any,
    ) -> bool {
        self.prepared_for_instance(key)
            && site == AffordanceSite::Property("absolute")
            && form == 0
            && op == Operation::ReadProperty
            && d.downcast_ref::<SocketAddr>() == Some(&self.config.1)
    }
    fn observe_installation(&self, _: &mut PublicationObserver<'_>) -> CoreResult<()> {
        if self.installed && self.socket.local_addr().ok() == Some(self.config.1) {
            Ok(())
        } else {
            Err(refused())
        }
    }
    fn validator(&self) -> Option<&dyn FiniteValidator> {
        Some(self)
    }
    fn poll_start(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        if self.lease.replace(true) {
            return Poll::Ready(Err(refused()));
        }
        self.installed = true;
        Poll::Ready(Ok(()))
    }
    fn poll_receive(
        &mut self,
        _: &mut Context<'_>,
        _: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<SocketAddr>>> {
        match self.socket.recv_from(&mut self.receive) {
            Ok((n, peer)) if &self.receive[..n] == b"READ" => Poll::Ready(Ok(FiniteIngress {
                candidates: vec![FiniteCandidate {
                    instance: self.config.0,
                    target: AffordanceTarget::Property("absolute".into()),
                    operation: Operation::ReadProperty,
                }],
                input: InteractionInput::empty(),
                reply: peer,
            })),
            Ok(_) => Poll::Pending,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            Err(_) => Poll::Ready(Err(refused())),
        }
    }
    fn poll_send(
        &mut self,
        _: &mut Context<'_>,
        reply: &RawFiniteReply<SocketAddr>,
    ) -> Poll<CoreResult<()>> {
        let data = reply
            .result
            .as_ref()
            .ok()
            .and_then(|o| o.data())
            .ok_or_else(refused)?;
        self.socket
            .send_to(&data.body, reply.ingress.reply)
            .map_err(|_| refused())?;
        Poll::Ready(Ok(()))
    }
    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        _: Option<&FiniteIngress<SocketAddr>>,
        _: Option<&RawFiniteReply<SocketAddr>>,
        _: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        self.release();
        Poll::Ready(Ok(FiniteDelivery::NotDelivered))
    }
}
impl FiniteValidator for Native {
    fn validate_input(&self, _: &FiniteCandidate, input: &InteractionInput) -> CoreResult<()> {
        if input.data.is_none() {
            Ok(())
        } else {
            Err(refused())
        }
    }
    fn validate_output(
        &self,
        _: &FiniteCandidate,
        _: &InteractionInput,
        out: &InteractionOutput,
    ) -> CoreResult<()> {
        clinkz_wot_codec_json::validate(&PreparedSchema::Type(ValueType::Integer), out)
    }
}
