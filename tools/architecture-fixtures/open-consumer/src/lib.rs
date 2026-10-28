//! Independently authored datagram finite Binding, with an owned byte-by-byte
//! compiler. Its native protocol is `READ /path\n` -> one JSON integer datagram.
//! Only the optional preparation module imports TD/Planning.
use clinkz_wot_core::{consumer::*, *};
use clinkz_wot_foundation::{WorkBudget, WorkClass};
use std::{
    net::{SocketAddr, UdpSocket},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

const FORMAT: BindingArtifactCompatibility =
    BindingArtifactCompatibility::new(*b"external-udp-v01");
#[derive(Default)]
pub struct Probe {
    pub ports: std::sync::Mutex<Vec<SocketAddr>>,
    pub bounds: AtomicUsize,
    pub starts: AtomicUsize,
    pub steps: AtomicUsize,
    pub aborts: AtomicUsize,
    pub programs: AtomicUsize,
    pub compiler_drops: AtomicUsize,
    pub step_failure: AtomicBool,
    pub invalid_artifact: AtomicBool,
    pub stall_compiler: AtomicBool,
    pub short_compile_budget: AtomicBool,
    pub native: AtomicUsize,
    pub calls: AtomicUsize,
    pub validates: AtomicUsize,
    pub changed: AtomicBool,
    pub denied: AtomicBool,
    pub data_lost: AtomicBool,
    pub transfer_once: AtomicBool,
    pub late_on_cancel: AtomicBool,
    pub fail_close: AtomicBool,
    pub pending_close: AtomicUsize,
}
pub fn configuration() -> ConsumerBindingConfiguration {
    ConsumerBindingConfiguration::for_semantic_host(
        256,
        BindingConfigurationDigest::new([177; 32]),
        FORMAT,
        1024,
        16,
    )
    .unwrap()
}
pub struct Program {
    address: SocketAddr,
    request: Vec<u8>,
    probe: Arc<Probe>,
}
impl Drop for Program {
    fn drop(&mut self) {
        self.probe.programs.fetch_add(1, Ordering::Relaxed);
    }
}
struct Client {
    probe: Arc<Probe>,
}
impl binding::ClientBinding for Client {
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        FORMAT
    }
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        (!self.probe.changed.load(Ordering::Relaxed)).then(configuration)
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        let Some(program) = artifact.artifact().try_payload::<Program>(FORMAT) else {
            return Err(reject(request));
        };
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(program.request.len()).is_err() {
            return Err(reject(request));
        }
        bytes.extend_from_slice(&program.request);
        let selected = (
            request.binding_id(),
            request.binding_generation(),
            request.plan_id(),
        );
        let call = Call {
            selected,
            request: Some(request),
            address: program.address,
            bytes,
            socket: None,
            sent: false,
            probe: self.probe.clone(),
        };
        HostBindingCallBox::try_new(call).map_err(|mut call| reject(call.request.take().unwrap()))
    }
}
pub struct Call {
    selected: (BindingId, BindingGeneration, PlanId),
    request: Option<OutboundRequest>,
    address: SocketAddr,
    bytes: Vec<u8>,
    socket: Option<UdpSocket>,
    sent: bool,
    probe: Arc<Probe>,
}
impl Drop for Call {
    fn drop(&mut self) {
        drop(self.socket.take());
        self.probe.calls.fetch_add(1, Ordering::Relaxed);
    }
}
impl Call {
    fn result(&mut self, budget: &mut WorkBudget) -> Poll<CoreResult<InteractionOutput>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        if self.socket.is_none() {
            self.probe.native.fetch_add(1, Ordering::Relaxed);
            let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
            socket.set_nonblocking(true).unwrap();
            self.probe
                .ports
                .lock()
                .unwrap()
                .push(socket.local_addr().unwrap());
            self.socket = Some(socket);
        }
        let socket = self.socket.as_ref().unwrap();
        if !self.sent {
            match socket.send_to(&self.bytes, self.address) {
                Ok(n) if n == self.bytes.len() => self.sent = true,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Poll::Pending,
                _ => return Poll::Ready(Err(invalid())),
            }
        }
        let mut bytes = [0u8; 1025];
        match socket.recv_from(&mut bytes) {
            Ok((n, source)) if source == self.address && n <= 1024 => {
                let (binding, generation, plan) = self.selected;
                let output =
                    InteractionOutput::with_data(Payload::new(&bytes[..n], "application/json"))
                        .try_with_metadata(
                            InteractionOutputMetadata::default().with_untrusted_binding_response(
                                BindingResponseMetadata::primary(binding, generation, plan, 0),
                            ),
                        )
                        .unwrap();
                Poll::Ready(Ok(output))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Poll::Pending,
            _ => Poll::Ready(Err(invalid())),
        }
    }
    fn cancel(
        &mut self,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if self.probe.late_on_cancel.load(Ordering::Relaxed) {
            return self
                .result(budget)
                .map(|r| Ok(BindingCallSettlement::Returned(r)));
        }
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        if self.probe.fail_close.load(Ordering::Relaxed) {
            return Poll::Ready(Err(invalid()));
        }
        if self
            .probe
            .pending_close
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1))
            .is_ok()
        {
            return Poll::Pending;
        }
        drop(self.socket.take());
        Poll::Ready(Ok(BindingCallSettlement::Cancelled {
            retry_class: RetryClass::Never,
            disposition: BindingCancellationDisposition::Complete {
                successor: NoCleanupSuccessor,
            },
        }))
    }
}
impl HostBindingCall<CoreResult<InteractionOutput>> for Call {
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        BindingLifetimeFootprint::new(
            4,
            (std::mem::size_of::<Self>() + self.bytes.capacity()) as u64,
        )
    }
    fn next_deadline(&self) -> Option<Deadline> {
        None
    }
    fn poll_result(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        self.result(budget)
    }
    fn start_cancel(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if self.probe.transfer_once.swap(false, Ordering::Relaxed) {
            let request = CleanupPhaseContext::bind_with_transfer_owner(
                CleanupReservation::new(
                    CleanupSlotId::new(
                        clinkz_wot_foundation::SlotIndex::new(1),
                        clinkz_wot_foundation::Generation::INITIAL,
                    ),
                    self.lifetime_footprint(),
                    0,
                    WorkBudget::new(),
                ),
                CleanupSlotId::new(
                    clinkz_wot_foundation::SlotIndex::new(2),
                    clinkz_wot_foundation::Generation::INITIAL,
                ),
                CleanupOperation::CancelRequest,
                invalid(),
                Deadline::NONE,
            )
            .try_into_transfer_request()
            .unwrap();
            return Ok(StartStatus::Ready(BindingCallSettlement::Cancelled {
                retry_class: RetryClass::Never,
                disposition: BindingCancellationDisposition::TransferRequired(request),
            }));
        }
        match self.cancel(budget) {
            Poll::Pending => Ok(StartStatus::Pending),
            Poll::Ready(result) => result.map(StartStatus::Ready),
        }
    }
    fn poll_cancel(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.cancel(budget)
    }
}
pub fn execution(id: u16, probe: Arc<Probe>) -> HostConsumerBindingRegistration {
    let config = configuration();
    HostConsumerBindingRegistration::new(HostConsumerBindingRegistrationInput::new(
        BindingRegistrationIdentity::new(
            BindingId::new(u32::from(id)),
            BindingGeneration::INITIAL,
            config.configuration(),
            FORMAT,
            u32::from(id),
        ),
        BindingExecutionSupport::host_erased(),
        Box::new(Client { probe }),
        BindingResourceDeclarations::new(
            BindingLifetimeFootprint::new(4, 1024),
            BindingLifetimeFootprint::new(16, 16384),
        )
        .with_resource_contract(BindingResourceContract::ConsumerOwnedHostV2)
        .with_consumer_result_limits(1024, 16)
        .with_transient(BindingTransientFootprint::new(2048)),
        BindingStatusPolicy::new(2, 17),
    ))
    .unwrap_or_else(|e| panic!("{:?}", e.error()))
}
struct Data(Arc<Probe>);
impl ConsumerDataService for Data {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn policy(&self) -> ConsumerDataPolicy {
        ConsumerDataPolicy {
            retained_bytes: 64,
            temporary_bytes: 0,
            body_bytes: 1024,
            validation_work: 4096,
        }
    }
    fn supports_schema(&self, schema: value_type::TypeSchema) -> bool {
        schema.accepts(value_type::ValueType::Integer)
    }
    fn available(&self) -> bool {
        !self.0.data_lost.load(Ordering::Relaxed)
    }
    fn validate(
        &self,
        schema: value_type::TypeSchema,
        output: &InteractionOutput,
    ) -> CoreResult<()> {
        self.0.validates.fetch_add(1, Ordering::Relaxed);
        let data = output.data().ok_or_else(invalid)?;
        let integer = std::str::from_utf8(&data.body)
            .ok()
            .and_then(|s| s.parse::<i64>().ok());
        if data.content_type != "application/json"
            || data.content_coding.is_some()
            || integer.is_none()
            || !schema.accepts(value_type::ValueType::Integer)
        {
            return Err(invalid());
        }
        Ok(())
    }
}
struct Access(Arc<Probe>);
impl ConsumerAccessService for Access {
    fn authorize(
        &self,
        id: BindingRegistrationIdentity,
        target: &str,
        op: Operation,
    ) -> CoreResult<()> {
        if self.0.denied.load(Ordering::Relaxed)
            || id.configuration() != configuration().configuration()
            || op != Operation::ReadProperty
            || !target.starts_with("linkedudp://127.0.0.1:")
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn work(&self) -> u64 {
        2
    }
    fn retained_bytes(&self) -> u64 {
        64
    }
}
pub fn services(probe: Arc<Probe>) -> ConsumerServices {
    ConsumerServices {
        data: Arc::new(Data(probe.clone())),
        access: Arc::new(Access(probe)),
    }
}
fn reject(request: OutboundRequest) -> BindingInputRejection<OutboundRequest> {
    BindingInputRejection::new(request, BindingOperationalError::new(invalid()))
}

#[cfg(feature = "prepare")]
pub mod prepare {
    use super::*;
    use clinkz_wot_planning::*;
    pub struct Compiler {
        pub probe: Arc<Probe>,
        pub config: ConsumerBindingConfiguration,
        pub oversized_work: bool,
    }
    pub struct Cursor {
        bytes: Vec<u8>,
        copied: usize,
        address: SocketAddr,
    }
    impl Drop for Compiler {
        fn drop(&mut self) {
            self.probe.compiler_drops.fetch_add(1, Ordering::Relaxed);
        }
    }
    impl ConsumerCompiler for Compiler {
        const CONTRACT: ConsumerCompilerContract = ConsumerCompilerContract {
            inspection_work: 2,
            start_work: 2,
            step_work: 2,
            max_steps: 256,
            abort_work: 3,
            artifact_drop_work: 2,
            inspection_temporary_bytes: 64,
        };
        fn configuration(&self) -> ConsumerBindingConfiguration {
            self.config
        }
    }
    impl BindingCompilerExtension for Compiler {
        type Cursor = Cursor;
        type Artifact = Program;
        fn compatibility(&self) -> BindingArtifactCompatibility {
            FORMAT
        }
        fn bounds(&self, input: &BindingCompilerInput<'_>) -> CoreResult<BindingCompilerBounds> {
            self.probe.bounds.fetch_add(1, Ordering::Relaxed);
            let target = input.resolved_target();
            if input.operation() != Operation::ReadProperty || !target.starts_with("linkedudp://") {
                return Err(CoreError::UnsupportedOperation(context()));
            }
            address(target)?;
            if target.ends_with("/failure") {
                return Err(invalid());
            }
            Ok(BindingCompilerBounds::new(
                BindingArtifactFootprint::new(1, 512),
                512,
                128,
                WorkBudget::new().with_remaining(
                    WorkClass::BindingPolls,
                    if self.oversized_work {
                        u64::MAX
                    } else if self.probe.short_compile_budget.load(Ordering::Relaxed) {
                        2
                    } else {
                        512
                    },
                ),
            ))
        }
        fn start(&self, input: &BindingCompilerInput<'_>) -> CoreResult<Cursor> {
            self.probe.starts.fetch_add(1, Ordering::Relaxed);
            let mut bytes = Vec::new();
            bytes.try_reserve_exact(256).map_err(|_| busy())?;
            bytes.extend_from_slice(b"READ ");
            Ok(Cursor {
                bytes,
                copied: 0,
                address: address(input.resolved_target())?,
            })
        }
        fn step(
            &self,
            input: &BindingCompilerInput<'_>,
            mut cursor: Cursor,
            budget: &mut WorkBudget,
        ) -> BindingCompilerStep<Cursor, Program> {
            if budget.consume(WorkClass::BindingPolls, 2).is_err() {
                return BindingCompilerStep::Pending(cursor);
            }
            self.probe.steps.fetch_add(1, Ordering::Relaxed);
            if self.probe.step_failure.load(Ordering::Relaxed) {
                return BindingCompilerStep::Failed(BindingCompilerFailure::new(invalid(), cursor));
            }
            if self.probe.stall_compiler.load(Ordering::Relaxed) {
                return BindingCompilerStep::Pending(cursor);
            }
            let target = input.resolved_target();
            let path = target
                .strip_prefix("linkedudp://")
                .unwrap()
                .split_once('/')
                .unwrap()
                .1
                .as_bytes();
            if cursor.copied == 0 {
                cursor.bytes.push(b'/');
            }
            if cursor.copied < path.len() {
                cursor.bytes.push(path[cursor.copied]);
                cursor.copied += 1;
                return BindingCompilerStep::Pending(cursor);
            }
            cursor.bytes.push(b'\n');
            let footprint = BindingArtifactFootprint::new(
                1,
                if self.probe.invalid_artifact.load(Ordering::Relaxed) {
                    513
                } else {
                    (std::mem::size_of::<Program>() + cursor.bytes.capacity()) as u64
                },
            );
            BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                FORMAT,
                footprint,
                Program {
                    address: cursor.address,
                    request: cursor.bytes,
                    probe: self.probe.clone(),
                },
            )))
        }
        fn abort(&self, _: Cursor) {
            self.probe.aborts.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn address(target: &str) -> CoreResult<SocketAddr> {
        let (address, path) = target
            .strip_prefix("linkedudp://")
            .and_then(|s| s.split_once('/'))
            .ok_or_else(invalid)?;
        if path.len() > 240 || path.bytes().any(|b| b < 33 || b > 126) {
            return Err(invalid());
        }
        address.parse().map_err(|_| invalid())
    }
    pub fn registration(
        id: u16,
        probe: Arc<Probe>,
    ) -> clinkz_wot_planning::open_consumer::ConsumerRegistrationInput {
        let compiler = HostBindingCompilerRegistration::try_new_consumer_extension(Compiler {
            probe: probe.clone(),
            config: configuration(),
            oversized_work: false,
        })
        .unwrap_or_else(|e| panic!("{:?}", e.error()));
        let preparation =
            HostConsumerBindingPreparation::try_pair(execution(id, probe.clone()), compiler)
                .unwrap_or_else(|e| panic!("{:?}", e.error()));
        clinkz_wot_planning::open_consumer::ConsumerRegistrationInput {
            preparation,
            services: services(probe),
        }
    }
}

/// Typed finite facet of the same downstream native implementation. No server
/// or notification methods are required for this Consumer-only registration.
#[cfg(feature = "prepare")]
pub struct StaticClient {
    pub probe: Arc<Probe>,
}
#[cfg(feature = "prepare")]
impl clinkz_wot_planning::PrepareClientBinding for StaticClient {
    type Compiler = prepare::Compiler;
}
#[cfg(feature = "prepare")]
impl PollClientBinding for StaticClient {
    type Artifact = Program;
    type RequestState = Call;
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        FORMAT
    }
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        Some(configuration())
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<Call>(BindingLifetimeFootprint::new(4, 8192))
            .with_transient(BindingTransientFootprint::new(2048))
    }
    fn start_request(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<Program>,
        slot: &mut ClientRequestSlot<Call>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Err(reject(request));
        }
        let program = artifact.artifact().payload();
        let selected = (
            request.binding_id(),
            request.binding_generation(),
            request.plan_id(),
        );
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(program.request.len()).is_err() {
            return Err(reject(request));
        }
        bytes.extend_from_slice(&program.request);
        slot.initialize(
            request,
            Call {
                selected,
                request: None,
                address: program.address,
                bytes,
                socket: None,
                sent: false,
                probe: self.probe.clone(),
            },
        );
        Ok(StartStatus::Pending)
    }
    fn poll_request(
        &mut self,
        _: &mut Context<'_>,
        slot: &mut ClientRequestSlot<Call>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        slot.state_mut().result(budget)
    }
    fn start_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        slot: &mut ClientRequestSlot<Call>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        match slot.state_mut().cancel(budget) {
            Poll::Pending => Ok(StartStatus::Pending),
            Poll::Ready(r) => r.map(StartStatus::Ready),
        }
    }
    fn poll_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        slot: &mut ClientRequestSlot<Call>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        slot.state_mut().cancel(budget)
    }
    fn acknowledge_request(&mut self, _: &mut ClientRequestSlot<Call>) -> CoreResult<()> {
        Ok(())
    }
}
