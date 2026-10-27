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
    program: HttpProducerProgram,
    instance: ProducerInstanceKey,
    property: Arc<str>,
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
            program,
            instance,
            property,
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
/// Limits match the retained HTTP transport: one connection/exchange, GET,
/// bodyless HTTP/1.1, exact path/Host, 16 headers/8192 bytes, 4096 I/O bytes per
/// native poll, JSON output <=4096 bytes. Hyper/socket allocations are native
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
    fn covers_definition(
        &self,
        key: ProducerInstanceKey,
        site: clinkz_wot_core::thing::AffordanceSite<'_>,
        _form: u32,
        operation: Operation,
        definition: &dyn Any,
    ) -> bool {
        matches!(site, clinkz_wot_core::thing::AffordanceSite::Property(name) if name == self.configuration.property.as_ref())
            && operation == Operation::ReadProperty
            && key == self.configuration.instance
            && definition.downcast_ref::<HttpProducerProgram>() == Some(&self.configuration.program)
    }
    fn output_validator(&self) -> Option<&dyn ProducerOutputValidator> {
        Some(&self.configuration.program)
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
                    self.connection =
                        Some(NativeConnection::new(stream, &self.configuration.program));
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
            Poll::Pending if connection.take_candidate() => Poll::Ready(Ok(FiniteIngress {
                candidates: vec![FiniteCandidate {
                    instance: self.configuration.instance,
                    target: AffordanceTarget::Property(self.configuration.property.clone()),
                    operation: Operation::ReadProperty,
                }],
                input: InteractionInput::default(),
                reply: HttpFiniteReply {
                    correlation: self.correlation,
                    issuer: Arc::downgrade(&self.issuer),
                },
            })),
            Poll::Pending => Poll::Pending,
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
