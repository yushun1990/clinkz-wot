//! Direct native participation in the shared finite execution architecture.
//! No old installation, accept permit, handler or Servient dependency is used.
use crate::{HttpProducerConfig, HttpProducerProgram, producer_transport::NativeConnection};
use clinkz_wot_core::{finite::*, producer::*, *};
use clinkz_wot_foundation::MonotonicInstant;
use std::{
    any::Any,
    net::TcpListener,
    sync::{Arc, Weak},
    task::{Context, Poll},
};

/// Complete immutable source-free operands for one bounded HTTP participant.
/// This is native configuration, not a final-TD qualification certificate.
/// The program comes from pure HTTP preparation; callers provision the owned
/// snapshot before runtime admission. Equality is bounded and passive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpFiniteConfig {
    listen: HttpProducerConfig,
    mappings: Arc<[(Arc<str>, HttpProducerProgram)]>,
    instance: ProducerInstanceKey,
    request_timeout_ticks: u64,
}

impl HttpFiniteConfig {
    pub fn try_new(
        listen: HttpProducerConfig,
        program: HttpProducerProgram,
        instance: ProducerInstanceKey,
        property: Arc<str>,
        request_timeout_ticks: u64,
    ) -> Result<Self, BindingInputRejection<(HttpProducerProgram, Arc<str>)>> {
        // Program fields are private and produced by the HTTP compiler. Check
        // actual listen operands as well as the bounded native target backing.
        if program.listen_address() != listen.listen_address()
            || property.len() > 256
            || request_timeout_ticks == 0
        {
            return Err(BindingInputRejection::new(
                (program, property),
                BindingOperationalError::new(crate::producer::producer_unsupported()),
            ));
        }
        Ok(Self {
            listen,
            mappings: Arc::from([(property, program)]),
            instance,
            request_timeout_ticks,
        })
    }

    /// One native listener with independently prepared Property mappings.
    /// Native method/path collisions refuse before socket acquisition. The
    /// complete mapping set shares immutable backing across configuration loans.
    #[allow(clippy::type_complexity)] // Refusal returns the complete original mapping owners.
    pub fn try_from_mappings(
        listen: HttpProducerConfig,
        mappings: Vec<(Arc<str>, HttpProducerProgram)>,
        instance: ProducerInstanceKey,
        request_timeout_ticks: u64,
    ) -> Result<Self, BindingInputRejection<Vec<(Arc<str>, HttpProducerProgram)>>> {
        if mappings.is_empty()
            || mappings.len() > 32
            || request_timeout_ticks == 0
            || mappings.iter().enumerate().any(|(i, (name, program))| {
                name.len() > 256
                    || program.listen_address() != listen.listen_address()
                    || mappings[..i].iter().any(|(_, old)| {
                        old.path_and_query() == program.path_and_query()
                            && old.operation() == program.operation()
                    })
            })
        {
            return Err(BindingInputRejection::new(
                mappings,
                BindingOperationalError::new(crate::producer::producer_unsupported()),
            ));
        }
        Ok(Self {
            listen,
            mappings: mappings.into(),
            instance,
            request_timeout_ticks,
        })
    }

    pub fn listen_address(&self) -> std::net::SocketAddr {
        self.listen.listen_address()
    }
}

/// One original exclusive native opportunity. It carries no logical authority,
/// is not Clone, and cannot be constructed independently of its HTTP owner.
pub struct HttpFiniteReply {
    correlation: u64,
    issuer: Weak<()>,
}

/// Manually polled HTTP listener/correlation owner. This facet directly uses
/// FiniteIngressBinding. Its only interaction with behavior is an owned candidate
/// and a borrowed result; native polling cannot dispatch application code.
///
/// At most 32 native Property mappings share one listener and one exchange.
/// HTTP/1.1 GET/PUT, exact path/Host, 16 headers/8192 bytes, JSON input/output
/// <=4096 bytes and 4096 I/O bytes per native poll. Hyper/socket allocations are native
/// Host backing, not a transitive allocation-free or aggregate-RAM guarantee.
/// Drop performs finite local socket/parser disposal without a task/network wait.
pub struct HttpFiniteBinding {
    configuration: HttpFiniteConfig,
    identity: (BindingId, BindingGeneration),
    issuer: Arc<()>,
    connection: Option<NativeConnection>,
    listener: Option<TcpListener>,
    correlation: u64,
    request_deadline: Deadline,
    failed: Option<CoreError>,
    closed: bool,
}

impl HttpFiniteBinding {
    /// Passive actual endpoint fact for final qualification, after readiness.
    /// This does not open semantic ingress or change the supplied Form meaning.
    pub fn bound_address(&self) -> Option<std::net::SocketAddr> {
        if self.closed || self.failed.is_some() {
            return None;
        }
        self.listener.as_ref()?.local_addr().ok()
    }

    /// Pure construction; first funded poll_start owns all native setup.
    pub fn new(
        configuration: HttpFiniteConfig,
        id: BindingId,
        generation: BindingGeneration,
    ) -> Self {
        Self {
            configuration,
            identity: (id, generation),
            issuer: Arc::new(()),
            connection: None,
            listener: None,
            correlation: 0,
            request_deadline: Deadline::NONE,
            failed: None,
            closed: false,
        }
    }

    fn failure(&mut self, phase: ErrorPhase) -> CoreError {
        let error = CoreError::Binding(
            ErrorContext::new(phase, RetryClass::Never)
                .with_thing(self.configuration.instance.slot())
                .with_binding(self.identity.0, self.identity.1)
                .with_redacted_cause(627, "finite HTTP native operation failed"),
        );
        if self.failed.is_none() {
            self.failed = Some(error.clone());
        }
        error
    }
}

impl Drop for HttpFiniteBinding {
    fn drop(&mut self) {
        // Native disposal precedes configuration/program/service release.
        drop(self.connection.take());
        drop(self.listener.take());
    }
}

impl FiniteIngressBinding for HttpFiniteBinding {
    type Configuration = HttpFiniteConfig;
    type Reply = HttpFiniteReply;

    fn identity(&self) -> (BindingId, BindingGeneration) {
        self.identity
    }
    fn configuration(&self) -> &Self::Configuration {
        &self.configuration
    }
    fn engine_backing_bytes(&self) -> Option<usize> {
        // Both configuration snapshots and accepted candidates share the
        // original mapping/name backing. Input body capacity is charged by the
        // runtime; include its Arc header/alignment and media backing here.
        // The native issuer allocation is retained until replies are disposed.
        Some(
            self.configuration
                .mappings
                .iter()
                .map(|(name, _)| {
                    name.len().next_multiple_of(core::mem::size_of::<usize>())
                        + 2 * core::mem::size_of::<usize>()
                })
                .sum::<usize>()
                + self.configuration.mappings.len()
                    * core::mem::size_of::<(Arc<str>, HttpProducerProgram)>()
                + 4 * core::mem::size_of::<usize>()
                + 64,
        )
    }
    fn prepared_for_instance(&self, key: ProducerInstanceKey) -> bool {
        self.configuration.instance == key
    }
    fn covers_definition(
        &self,
        key: ProducerInstanceKey,
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        form: u32,
        operation: Operation,
        definition: &dyn Any,
    ) -> bool {
        key == self.configuration.instance && self.configuration.mappings.iter().any(|(name, program)| {
            matches!(site, clinkz_wot_core::thing::AffordanceSite::Property(n) if n == name.as_ref())
                && operation == program.operation && form == program.form
                && definition.downcast_ref::<HttpProducerProgram>() == Some(program)
        })
    }
    fn validator(&self) -> Option<&dyn FiniteValidator> {
        Some(&self.configuration)
    }
    fn request_deadline(&self) -> Deadline {
        self.request_deadline
    }
    fn poll_start(&mut self, _: &mut Context<'_>) -> Poll<CoreResult<()>> {
        if self.closed {
            return Poll::Ready(Err(crate::producer::producer_unsupported()));
        }
        if let Some(error) = &self.failed {
            return Poll::Ready(Err(error.clone()));
        }
        if self.listener.is_some() {
            return Poll::Ready(Ok(()));
        }
        let address = self.configuration.listen_address();
        let socket = match socket2::Socket::new(
            socket2::Domain::for_address(address),
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        ) {
            Ok(socket) => socket,
            Err(_) => return Poll::Ready(Err(self.failure(ErrorPhase::Prepare))),
        };
        // Retain the partial owner before every fallible native configuration.
        self.listener = Some(socket.into());
        let configured = (|| -> std::io::Result<()> {
            let socket = socket2::SockRef::from(self.listener.as_ref().unwrap());
            socket.set_reuse_address(true)?;
            socket.bind(&address.into())?;
            // Closed setup buffers only the bounded OS listen backlog. No
            // stream is accepted/parsed before the Servient publication commit.
            // The OS owns TCP buffering; peers may wait or be refused when full.
            socket.listen(128)?;
            socket.set_nonblocking(true)
        })();
        if configured.is_err() {
            return Poll::Ready(Err(self.failure(ErrorPhase::Prepare)));
        }
        Poll::Ready(Ok(()))
    }

    fn poll_receive(
        &mut self,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
    ) -> Poll<CoreResult<FiniteIngress<Self::Reply>>> {
        if self.closed || self.failed.is_some() {
            return Poll::Ready(Err(crate::producer::producer_unsupported()));
        }
        if self.connection.is_none() {
            let Some(expires) = now.checked_add_ticks(self.configuration.request_timeout_ticks)
            else {
                return Poll::Ready(Err(self.failure(ErrorPhase::Binding)));
            };
            let Some(listener) = &self.listener else {
                return Poll::Ready(Err(crate::producer::producer_unsupported()));
            };
            match listener.accept() {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                Err(_) => return Poll::Ready(Err(self.failure(ErrorPhase::Binding))),
                Ok((stream, _)) => {
                    if stream.set_nonblocking(true).is_err() {
                        return Poll::Ready(Err(self.failure(ErrorPhase::Binding)));
                    }
                    let Some(next) = self.correlation.checked_add(1) else {
                        return Poll::Ready(Err(self.failure(ErrorPhase::Binding)));
                    };
                    self.correlation = next;
                    self.request_deadline = Deadline::at(expires);
                    self.connection = Some(NativeConnection::from_programs(
                        stream,
                        self.configuration
                            .mappings
                            .iter()
                            .map(|(_, p)| p.clone())
                            .collect(),
                    ));
                }
            }
        }
        let connection = self.connection.as_mut().unwrap();
        match connection.poll(cx) {
            Poll::Ready(_) => {
                // Unsupported/malformed/disconnected input never escapes as a
                // semantic candidate. Disposing it leaves the listener usable.
                drop(self.connection.take());
                self.request_deadline = Deadline::NONE;
                Poll::Pending
            }
            Poll::Pending => {
                let Some((index, input)) = connection.take_request() else {
                    return Poll::Pending;
                };
                let (property, program) = &self.configuration.mappings[index];
                Poll::Ready(Ok(FiniteIngress {
                    candidates: vec![FiniteCandidate {
                        instance: self.configuration.instance,
                        target: AffordanceTarget::Property(property.clone()),
                        operation: program.operation,
                    }],
                    input,
                    reply: HttpFiniteReply {
                        correlation: self.correlation,
                        issuer: Arc::downgrade(&self.issuer),
                    },
                }))
            }
        }
    }

    fn poll_send(
        &mut self,
        cx: &mut Context<'_>,
        reply: &RawFiniteReply<Self::Reply>,
    ) -> Poll<CoreResult<()>> {
        if reply.ingress.reply.issuer.as_ptr() != Arc::as_ptr(&self.issuer)
            || reply.ingress.reply.correlation != self.correlation
            || self.closed
        {
            return Poll::Ready(Err(crate::producer::producer_unsupported()));
        }
        let Some(connection) = self.connection.as_mut() else {
            return Poll::Ready(Err(crate::producer::producer_unsupported()));
        };
        if let Err(error) = connection.reply(reply.result.as_ref()) {
            return Poll::Ready(Err(error));
        }
        match connection.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                drop(self.connection.take());
                self.request_deadline = Deadline::NONE;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(_)) => Poll::Ready(Err(self.failure(ErrorPhase::Binding))),
        }
    }

    fn poll_close(
        &mut self,
        _: &mut Context<'_>,
        _: Option<&FiniteIngress<Self::Reply>>,
        _: Option<&RawFiniteReply<Self::Reply>>,
        _: Option<&CoreError>,
    ) -> Poll<CoreResult<FiniteDelivery>> {
        let delivery = if self.connection.as_ref().is_some_and(|c| c.write_started()) {
            FiniteDelivery::Unknown
        } else {
            FiniteDelivery::NotDelivered
        };
        self.closed = true;
        // Abort is this native facet's retirement policy; no refusal is claimed
        // delivered and a partial write is never reported as NotDelivered.
        drop(self.connection.take());
        drop(self.listener.take());
        Poll::Ready(Ok(delivery))
    }
}

impl HttpFiniteConfig {
    fn program_for(&self, candidate: &FiniteCandidate) -> CoreResult<&HttpProducerProgram> {
        if candidate.instance != self.instance {
            return Err(crate::producer::producer_unsupported());
        }
        self.mappings
            .iter()
            .find(|(name, p)| {
                matches!(&candidate.target, AffordanceTarget::Property(n) if n == name)
                    && candidate.operation == p.operation
            })
            .map(|(_, program)| program)
            .ok_or_else(crate::producer::producer_unsupported)
    }
}
impl FiniteValidator for HttpFiniteConfig {
    fn validate_input(
        &self,
        candidate: &FiniteCandidate,
        input: &InteractionInput,
    ) -> CoreResult<()> {
        let program = self.program_for(candidate)?;
        match (program.operation, input.data.as_ref()) {
            (Operation::ReadProperty, None) => Ok(()),
            (Operation::WriteProperty, Some(payload)) => {
                crate::producer_validation::validate_payload(program.value_schema, payload)
            }
            _ => Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Validate,
                RetryClass::Never,
            ))),
        }
    }
    fn validate_output(
        &self,
        candidate: &FiniteCandidate,
        output: &InteractionOutput,
    ) -> CoreResult<()> {
        let program = self.program_for(candidate)?;
        match program.operation {
            Operation::ReadProperty => program.validate(output),
            Operation::WriteProperty if raw_finite_property_shape(program.operation, output) => {
                Ok(())
            }
            _ => Err(CoreError::Validation(ErrorContext::new(
                ErrorPhase::Validate,
                RetryClass::Never,
            ))),
        }
    }
}
