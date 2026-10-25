//! Source-free native Producer configuration and instructions. No TD or
//! Planning dependency is needed to retain these runtime values.
use super::*;

/// Immutable listen operands for the initial loopback HTTP Producer adapter.
/// Construction is pure. Native installation uses the captured same value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpProducerConfig {
    pub(super) address: SocketAddr,
}
impl HttpProducerConfig {
    pub fn for_loopback(address: SocketAddr) -> CoreResult<Self> {
        if !address.ip().is_loopback()
            || address.port() == 0
            || matches!(address, SocketAddr::V6(v6) if v6.flowinfo()!=0 || v6.scope_id()!=0)
        {
            return Err(producer_unsupported());
        }
        Ok(Self { address })
    }
    pub fn listen_address(&self) -> SocketAddr {
        self.address
    }
}

/// Owned immutable native instructions, without a Copy/inline requirement on
/// any shared compiler interface. This concrete first program is inline.
/// Possession grants no installation, publication, dispatch or accept authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpProducerProgram {
    pub(super) address: SocketAddr,
    pub(super) target: [u8; 64],
    pub(super) target_len: u8,
    pub(super) authority_end: u8,
    pub(super) origin: [u8; 64],
    pub(super) origin_len: u8,
    pub(super) response_schema: clinkz_wot_core::value_type::TypeSchema,
}
impl HttpProducerProgram {
    pub fn listen_address(&self) -> SocketAddr {
        self.address
    }
    pub fn target(&self) -> &str {
        std::str::from_utf8(&self.target[..self.target_len as usize]).unwrap()
    }
    pub fn authority(&self) -> &str {
        &self.target()[7..self.authority_end as usize]
    }
    pub fn path_and_query(&self) -> &str {
        std::str::from_utf8(&self.origin[..self.origin_len as usize]).unwrap()
    }
}
pub(super) fn producer_unsupported() -> CoreError {
    CoreError::UnsupportedOperation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_redacted_cause(621, "unsupported HTTP Producer mapping"),
    )
}

use clinkz_wot_core::producer::*;
use std::{
    net::TcpListener,
    task::{Context, Poll},
};

// Separate role identity: never borrow Consumer policy/seals or StrictV1's
// transitive disposal claim. Native parser/reply backing remains Binding-owned.
const INSTALL_POLICY: &[u8] = b"request-reply-v3;type-only-schema;exact-decimal-integer;Hyper1.12.0-server-http1;complete-request-half-close;httparse1.10.1;serde_json1.0.149-IgnoredAny+UTF8;manual-nonblocking-poll;head16/8192inclusive;read-write4096-per-poll;one-connection;one-correlation;JSON4096;media64;request-deadline-caller-ticks;no-pipeline-keepalive-expect-upgrade-body;native-parser-box-and-bridge-private;clinkz-http-producer-install-v1;socket2-0.6.4-TcpListener;reuseaddr-no-reuseport;backlog128;loopback-nonzero-unscoped;exclusive-native-listener;permit-accept;Core-sealed-response;controlled-indirect=0/0/0;temporary=1024-not-stack-ceiling;largest-indirect=0;local-drop-work=1;finite-SDK-local-drop;no-task;no-network-wait";

/// Runtime-only executor. It retains neither a compiler nor source TD.
pub struct HttpProducerExecutor {
    config: HttpProducerConfig,
    identity: BindingRegistrationIdentity,
}
pub fn producer_execution(
    config: HttpProducerConfig,
    id: BindingId,
    generation: BindingGeneration,
    ordinal: u32,
) -> HttpProducerExecutor {
    let mut operands = [0; 19];
    match config.address.ip() {
        IpAddr::V4(ip) => {
            operands[0] = 4;
            operands[1..5].copy_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            operands[0] = 6;
            operands[1..17].copy_from_slice(&ip.octets());
        }
    }
    operands[17..].copy_from_slice(&config.address.port().to_be_bytes());
    let digest = sha2_const_stable::Sha256::new()
        .update(INSTALL_POLICY)
        .update(&operands)
        .finalize();
    HttpProducerExecutor {
        config,
        identity: BindingRegistrationIdentity::new(
            id,
            generation,
            BindingConfigurationDigest::new(digest),
            BindingArtifactCompatibility::new(*b"http-prod-rpl-v3"),
            ordinal,
        ),
    }
}
impl ProducerExecutor for HttpProducerExecutor {
    type Configuration = HttpProducerConfig;
    type Program = HttpProducerProgram;
    type Installation = HttpProducerInstallation;
    fn configuration_matches(&self, captured: &Self::Configuration) -> bool {
        self.config == *captured
    }
    fn identity(&self) -> BindingRegistrationIdentity {
        self.identity
    }
    fn install_policy(&self) -> ProducerInstallPolicy {
        ProducerInstallPolicy {
            executor_bytes: 0,
            program_bytes: 0,
            installation_bytes: 0,
            temporary_bytes: 1024,
            largest_indirect_allocation: 0,
            local_drop_work: 1,
        }
    }
    fn service_policy(&self) -> Option<ProducerServicePolicy> {
        Some(ProducerServicePolicy {
            response_body_bytes: 4096,
            response_media_capacity: 64,
        })
    }
    fn check_program(&self, program: &Self::Program) -> CoreResult<()> {
        // Inspect native operands, not a caller-supplied compatibility tag.
        if program.address != self.config.address
            || program.target_len as usize > program.target.len()
            || program.origin_len as usize > program.origin.len()
            || program.authority_end > program.target_len
        {
            return Err(producer_unsupported());
        }
        Ok(())
    }
    fn output_validator<'a>(
        &'a self,
        program: &'a Self::Program,
    ) -> Option<&'a dyn ProducerOutputValidator> {
        Some(program)
    }
    fn create_installation(
        &self,
        entry: &ProducerExecutionEntry<Self::Program>,
    ) -> CoreResult<Self::Installation> {
        self.check_program(entry.program())?;
        if entry.binding() != self.identity {
            return Err(producer_unsupported());
        }
        Ok(HttpProducerInstallation {
            config: self.config,
            identity: self.identity,
            instance: entry.instance(),
            instance_key: entry.instance_key(),
            program: entry.program().clone(),
            connection: None,
            correlation: 0,
            request_deadline: Deadline::NONE,
            listener: None,
            failed: None,
            closed: false,
        })
    }
}
impl ProducerOutputValidator for HttpProducerProgram {
    fn validate(&self, output: &InteractionOutput) -> CoreResult<()> {
        super::producer_validation::validate(self.response_schema, output)
    }
}
/// Binding-owned native setup, request correlation, reply and retirement.
/// Acceptance requires a borrowed permit. There is no publication or handler
/// operation; closed-gate connections remain deployment/kernel state.
pub struct HttpProducerInstallation {
    config: HttpProducerConfig,
    identity: BindingRegistrationIdentity,
    instance: ThingSlotId,
    instance_key: ProducerInstanceKey,
    program: HttpProducerProgram,
    connection: Option<super::producer_transport::NativeConnection>,
    correlation: u64,
    request_deadline: Deadline,
    listener: Option<TcpListener>,
    failed: Option<CoreError>,
    closed: bool,
}
impl HttpProducerInstallation {
    fn failure(&mut self, phase: ErrorPhase) -> CoreError {
        let error = CoreError::Binding(
            ErrorContext::new(phase, RetryClass::Never)
                .with_thing(self.instance)
                .with_binding(
                    self.identity.binding_id(),
                    self.identity.binding_generation(),
                )
                .with_resource_failure(
                    self.identity.configuration(),
                    ResourceFailureDomain::DeploymentFailure,
                    None,
                    None,
                )
                .with_redacted_cause(623, "HTTP Producer native operation failed"),
        );
        self.failed = Some(error.clone());
        error
    }
}
impl ProducerInstallation for HttpProducerInstallation {
    fn poll_install(&mut self, _cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        if let Some(error) = &self.failed {
            return Poll::Ready(Err(error.clone()));
        }
        if self.closed {
            return Poll::Ready(Err(producer_unsupported()));
        }
        if self.listener.is_some() {
            return Poll::Ready(Ok(()));
        }
        let socket = match socket2::Socket::new(
            socket2::Domain::for_address(self.config.address),
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        ) {
            Ok(socket) => socket,
            Err(_) => return Poll::Ready(Err(self.failure(ErrorPhase::Prepare))),
        };
        // Retain even an unbound native socket before fallible configuration.
        self.listener = Some(socket.into());
        let configured = (|| -> std::io::Result<()> {
            let socket = socket2::SockRef::from(self.listener.as_ref().unwrap());
            // Restart after our own closed connections enter TIME_WAIT. This
            // does not enable SO_REUSEPORT or share a live listening endpoint.
            socket.set_reuse_address(true)?;
            socket.bind(&self.config.address.into())?;
            socket.listen(128)?;
            socket.set_nonblocking(true)
        })();
        if configured.is_err() {
            return Poll::Ready(Err(self.failure(ErrorPhase::Prepare)));
        }
        Poll::Ready(Ok(()))
    }
    fn poll_request(
        &mut self,
        cx: &mut Context<'_>,
        permit: ProducerAcceptPermit<'_>,
        now: clinkz_wot_foundation::MonotonicInstant,
        timeout_ticks: u64,
    ) -> Poll<CoreResult<ProducerRequest>> {
        if permit.instance() != self.instance_key
            || self.closed
            || self.failed.is_some()
            || timeout_ticks == 0
        {
            return Poll::Ready(Err(producer_unsupported()));
        }
        if self.connection.is_none() {
            let deadline = match now.checked_add_ticks(timeout_ticks) {
                Some(t) => Deadline::at(t),
                None => return Poll::Ready(Err(producer_unsupported())),
            };
            let Some(listener) = &self.listener else {
                return Poll::Ready(Err(producer_unsupported()));
            };
            match listener.accept() {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                Err(_) => return Poll::Ready(Err(self.failure(ErrorPhase::Binding))),
                Ok((stream, _)) => {
                    // The local socket is already owned, including configuration failure.
                    if stream.set_nonblocking(true).is_err() {
                        return Poll::Ready(Err(self.failure(ErrorPhase::Binding)));
                    }
                    let Some(next) = self.correlation.checked_add(1) else {
                        return Poll::Ready(Err(self.failure(ErrorPhase::Binding)));
                    };
                    self.correlation = next;
                    self.request_deadline = deadline;
                    self.connection = Some(super::producer_transport::NativeConnection::new(
                        stream,
                        &self.program,
                    ));
                }
            }
        }
        let connection = self.connection.as_mut().unwrap();
        match connection.poll(cx) {
            Poll::Ready(_) => {
                // Malformed/unsupported/disconnected peers retire their socket;
                // no semantic request escaped and the listener remains usable.
                drop(self.connection.take());
                self.request_deadline = Deadline::NONE;
                Poll::Pending
            }
            Poll::Pending if connection.take_candidate() => Poll::Ready(Ok(ProducerRequest::new(
                self.instance_key,
                CorrelationId::new(self.correlation),
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
    fn poll_response(
        &mut self,
        cx: &mut Context<'_>,
        response: &ProducerResponse,
        _now: clinkz_wot_foundation::MonotonicInstant,
    ) -> Poll<CoreResult<()>> {
        if response.request().instance() != self.instance_key
            || response.request().correlation().get() != self.correlation
            || self.connection.is_none()
            || self.closed
        {
            return Poll::Ready(Err(producer_unsupported()));
        }
        let connection = self.connection.as_mut().unwrap();
        if let Err(error) = connection.reply(response.result()) {
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
    fn request_deadline(&self) -> Deadline {
        self.request_deadline
    }
    fn poll_close(&mut self, _cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        self.closed = true;
        drop(self.connection.take());
        drop(self.listener.take());
        Poll::Ready(Ok(()))
    }
}
impl Drop for HttpProducerInstallation {
    fn drop(&mut self) {
        drop(self.connection.take());
        drop(self.listener.take());
    }
}
