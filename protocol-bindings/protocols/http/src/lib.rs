//! Optional Host HTTP/1.1 Consumer Property Read and Producer Property/Action binding.
//!
//! This slice supports cleartext IP literals, compiled JSON reads, status
//! 200, fixed-length/chunked/close-delimited responses and bounded discarded
//! trailers. The Producer offers bounded loopback Property GET and continuing
//! Action POST/GET/DELETE through explicit preparation and manually progressed
//! Servient ownership. See README.md for deployment and qualification limits.
use clinkz_wot_core::*;
use clinkz_wot_foundation::*;
use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};
#[cfg(feature = "prepare")]
mod compiler;
#[cfg(feature = "prepare")]
mod finite_preparation;
mod finite_producer;
#[cfg(feature = "prepare")]
pub use finite_preparation::HttpFiniteProducer;
#[cfg(all(test, feature = "prepare"))]
mod identity_tests;
mod producer;
pub use finite_producer::{HttpFiniteBinding, HttpFiniteConfig, HttpFiniteReply};
mod producer_validation;
mod program;
pub use producer::{
    HttpProducerConfig, HttpProducerExecutor, HttpProducerInstallation, HttpProducerProgram,
    producer_execution,
};
#[cfg(feature = "prepare")]
mod producer_compiler;
#[cfg(feature = "prepare")]
pub use producer_compiler::try_producer_registration;
mod transport;
#[cfg(feature = "prepare")]
use compiler::HttpCompiler;
use program::{COMPATIBILITY, HttpArtifact};

const BODY: u64 = 4096;
const MEDIA: u64 = 16;
// All immutable behavior (including dependency configuration) is versioned here.
// Bump this encoding when any supported behavior or private ceiling changes.
const PRIVATE_POLICY: &[u8] = concat!(
    "clinkz-http-ip-v6\0",
    "hyper1.12.0(client,http1);httparse1.10.1(std);hyper-util0.1.20(tokio);",
    "tokio1.52.3(net,rt);http-body-util0.1.3;bytes1.11.1;socket2-0.6.4;",
    "GET-default;explicit-GET,POST,PUT,PATCH,DELETE;header8/1024;NoSec;200;json-exact;noCE;noUpgrade;no1xx;http1.1;IP;target64;",
    "noUserinfo;noFragment;noTemplate;noUriVariables;cleartext-permission;",
    "fresh;concurrency1-until-release;header32/8192inclusive;body4096;",
    "trailer32/8192exclusive-duplicates-count;hyper-cumulative-extensions<16384;",
    "wire32768;read4096;paid-callback4096;hyperbuf8192-adaptive;writevFalse;",
    "trailer-discard-unannounced;declaration-empty-elements-ignore-SP-HTAB;",
    "forbidden-trailer=content-type,content-length,transfer-encoding,content-encoding,trailer,upgrade,connection,host,te;",
    "noTEadvertisement;single-chunked;noTEandCL;singleCL;",
    "call-owned-driver;noSpawn;local-drop-before-settlement;noNetworkWait;",
    "SDK-disposal-may-allocate;noPool;noRetry;noTLS;noDNS;",
    "connect-io=DeploymentFailure"
).as_bytes();
const PRIVATE: [u8; 32] = sha2_const_stable::Sha256::new()
    .update(PRIVATE_POLICY)
    .finalize();

/// Checked immutable permission and policy for the cleartext-IP slice.
#[derive(Clone, Copy, Debug)]
pub struct HttpConsumerConfig {
    configuration: ConsumerBindingConfiguration,
}
impl HttpConsumerConfig {
    /// Explicitly permits cleartext HTTP to application-selected IP literals.
    /// Construction does not inspect a runtime or perform I/O.
    pub fn for_cleartext_ip() -> CoreResult<Self> {
        Ok(Self {
            configuration: ConsumerBindingConfiguration::for_semantic_host(
                64,
                BindingConfigurationDigest::new(PRIVATE),
                COMPATIBILITY,
                BODY,
                MEDIA,
            )?,
        })
    }
}

fn error(code: u16, cause: &'static str) -> CoreError {
    resource_error(code, cause, ResourceFailureDomain::BindingPolicy)
}
fn resource_error(code: u16, cause: &'static str, domain: ResourceFailureDomain) -> CoreError {
    CoreError::Binding(
        ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
            .with_resource_failure(BindingConfigurationDigest::new(PRIVATE), domain, None, None)
            .with_redacted_cause(code, cause),
    )
}
fn reject(
    request: OutboundRequest,
    config: HttpConsumerConfig,
    domain: ResourceFailureDomain,
    unsupported: bool,
) -> BindingInputRejection<OutboundRequest> {
    let context = ErrorContext::new(ErrorPhase::Binding, RetryClass::Never)
        .with_plan(request.plan_id())
        .with_binding(request.binding_id(), request.binding_generation())
        .with_resource_failure(config.configuration.private_policy(), domain, None, None)
        .with_redacted_cause(600, "HTTP request cannot be accepted");
    BindingInputRejection::new(
        request,
        BindingOperationalError::new(if unsupported {
            CoreError::UnsupportedOperation(context)
        } else {
            CoreError::Backpressure(context)
        }),
    )
}

struct Target {
    address: SocketAddr,
    uri: hyper::Uri,
}
impl Target {
    fn parse(text: &str) -> CoreResult<Self> {
        if text.len() > 64
            || text
                .bytes()
                .any(|c| c <= 32 || c >= 127 || b"#{}\\".contains(&c))
        {
            return Err(error(601, "unsupported HTTP target"));
        }
        let uri: hyper::Uri = text
            .parse()
            .map_err(|_| error(601, "unsupported HTTP target"))?;
        let authority = uri
            .authority()
            .ok_or_else(|| error(601, "missing HTTP authority"))?;
        if uri.scheme_str() != Some("http")
            || (authority.as_str().contains('@') || authority.as_str().ends_with(':'))
        {
            return Err(error(601, "cleartext IP target required"));
        }
        let host = uri.host().ok_or_else(|| error(601, "missing HTTP host"))?;
        let ip: IpAddr = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse()
            .map_err(|_| error(601, "IP literal required"))?;
        // Authority::port() returns None for an out-of-range explicit port.
        // Distinguish that from absent syntax before applying the default.
        let suffix = &authority.as_str()[host.len()..];
        let port = if suffix.is_empty() {
            80
        } else {
            suffix
                .strip_prefix(':')
                .and_then(|p| p.parse::<u16>().ok())
                .ok_or_else(|| error(601, "invalid HTTP port"))?
        };
        if port == 0 {
            return Err(error(601, "invalid HTTP port"));
        }
        Ok(Self {
            address: SocketAddr::new(ip, port),
            uri,
        })
    }
}

struct Client {
    config: HttpConsumerConfig,
    busy: Arc<AtomicBool>,
}
type Exchange = Pin<Box<dyn Future<Output = CoreResult<Vec<u8>>> + Send>>;
struct Call {
    request: Option<OutboundRequest>,
    target: Option<(Target, HttpArtifact)>,
    handle: tokio::runtime::Handle,
    future: Option<Exchange>,
    allowance: Arc<AtomicUsize>,
    busy: Arc<AtomicBool>,
}
impl Drop for Call {
    fn drop(&mut self) {
        // Retire the entire connection and its driver before releasing occupancy.
        drop(self.future.take());
        self.busy.store(false, Ordering::Release);
    }
}

impl binding::ClientBinding for Client {
    fn supports_operation(&self, operation: clinkz_wot_core::Operation) -> bool {
        operation == clinkz_wot_core::Operation::ReadProperty
    }
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        Some(self.config.configuration)
    }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        COMPATIBILITY
    }
    fn invoke(
        &self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<HostBindingArtifact>,
    ) -> Result<
        HostBindingCallBox<CoreResult<InteractionOutput>>,
        BindingInputRejection<OutboundRequest>,
    > {
        if !request.uri_variables().is_empty() {
            return Err(reject(
                request,
                self.config,
                ResourceFailureDomain::BindingPolicy,
                true,
            ));
        }
        let target = artifact
            .artifact()
            .try_payload::<HttpArtifact>(COMPATIBILITY)
            .and_then(|a| Some((Target::parse(a.target()).ok()?, *a)));
        let Some(target) = target else {
            return Err(reject(
                request,
                self.config,
                ResourceFailureDomain::BindingPolicy,
                true,
            ));
        };
        let handle = match transport::runtime() {
            Ok(h) => h,
            Err(_) => {
                return Err(reject(
                    request,
                    self.config,
                    ResourceFailureDomain::DeploymentFailure,
                    false,
                ));
            }
        };
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(reject(
                request,
                self.config,
                ResourceFailureDomain::BindingPolicy,
                false,
            ));
        }
        let call = Call {
            request: Some(request),
            target: Some(target),
            handle,
            future: None,
            allowance: Arc::new(AtomicUsize::new(0)),
            busy: self.busy.clone(),
        };
        match HostBindingCallBox::try_new(call) {
            Ok(call) => Ok(call),
            Err(mut call) => Err(reject(
                call.request.take().unwrap(),
                self.config,
                ResourceFailureDomain::DeploymentFailure,
                false,
            )),
        }
    }
}
impl Call {
    fn retire(&mut self) {
        drop(self.future.take());
        drop(self.target.take());
    }
    fn cancelled(&mut self) -> BindingCallSettlement<CoreResult<InteractionOutput>> {
        self.retire();
        BindingCallSettlement::Cancelled {
            retry_class: RetryClass::Never,
            disposition: BindingCancellationDisposition::Complete {
                successor: NoCleanupSuccessor,
            },
        }
    }
}
impl HostBindingCall<CoreResult<InteractionOutput>> for Call {
    fn lifetime_footprint(&self) -> BindingLifetimeFootprint {
        // Protocol future/buffers/reactor allocations are private, not engine
        // carrier bytes. Full engine-held call representation is charged here.
        BindingLifetimeFootprint::new(1, std::mem::size_of::<Self>() as u64)
    }
    fn next_deadline(&self) -> Option<Deadline> {
        self.request.as_ref().unwrap().deadline()
    }
    fn poll_result(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        self.allowance.store(4096, Ordering::Relaxed);
        if self.future.is_none() {
            let Some((target, instructions)) = self.target.take() else {
                return Poll::Ready(Err(error(602, "HTTP call already retired")));
            };
            self.future = Some(Box::pin(transport::exchange(
                target,
                instructions,
                self.allowance.clone(),
            )));
        }
        let handle = self.handle.clone();
        let _entered = handle.enter();
        match self.future.as_mut().unwrap().as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.retire();
                let request = self.request.as_ref().unwrap();
                let result = result.map_err(|cause| {
                    CoreError::Binding(
                        (*cause.context())
                            .with_plan(request.plan_id())
                            .with_binding(request.binding_id(), request.binding_generation()),
                    )
                });
                Poll::Ready(result.and_then(|body| {
                    InteractionOutput::with_data(Payload::new(body, "application/json"))
                        .try_with_metadata(
                            InteractionOutputMetadata::default().with_untrusted_binding_response(
                                BindingResponseMetadata::primary(
                                    request.binding_id(),
                                    request.binding_generation(),
                                    request.plan_id(),
                                    200,
                                ),
                            ),
                        )
                        .ok_or_else(|| error(609, "HTTP metadata rejected"))
                }))
            }
        }
    }
    fn start_cancel(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Ok(StartStatus::Pending);
        }
        Ok(StartStatus::Ready(self.cancelled()))
    }
    fn poll_cancel(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return Poll::Pending;
        }
        Poll::Ready(Ok(self.cancelled()))
    }
}

/// Creates the runtime registration without a TD parser or compiler.
pub fn try_consumer_execution(
    config: HttpConsumerConfig,
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    diagnostic_ordinal: u32,
) -> CoreResult<HostConsumerBindingRegistration> {
    let identity = BindingRegistrationIdentity::new(
        binding_id,
        binding_generation,
        config.configuration.configuration(),
        COMPATIBILITY,
        diagnostic_ordinal,
    );
    let resources = BindingResourceDeclarations::new(
        BindingLifetimeFootprint::new(3, std::mem::size_of::<Client>() as u64 + 32),
        BindingLifetimeFootprint::new(16, 8192),
    )
    .with_resource_contract(BindingResourceContract::ConsumerOwnedHostV2)
    .with_consumer_result_limits(BODY, MEDIA)
    .with_transient(BindingTransientFootprint::new(16384));
    HostConsumerBindingRegistration::new(HostConsumerBindingRegistrationInput::new(
        identity,
        BindingExecutionSupport::host_erased(),
        Box::new(Client {
            config,
            busy: Arc::new(AtomicBool::new(false)),
        }),
        resources,
        BindingStatusPolicy::new(2, 17),
    ))
    .map_err(|e| e.error().error().clone())
}
/// Pairs optional preparation with the exact runtime policy before any effects.
#[cfg(feature = "prepare")]
pub fn try_inline_consumer_registration(
    config: HttpConsumerConfig,
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    diagnostic_ordinal: u32,
) -> CoreResult<
    clinkz_wot_planning::InlineConsumerCompilerRegistration<
        clinkz_wot_planning::HostConsumerBindingPreparation,
    >,
> {
    use clinkz_wot_planning::*;
    let compiler = HostBindingCompilerRegistration::try_new_inline_semantic(HttpCompiler(config))
        .map_err(|e| e.error().error().clone())?;
    let runtime =
        try_consumer_execution(config, binding_id, binding_generation, diagnostic_ordinal)?;
    HostConsumerBindingPreparation::try_pair(runtime, compiler)
        .map_err(|e| e.error().error().clone())?
        .try_into_inline_consumer_compiler()
        .map_err(|e| e.error().error().clone())
}

#[cfg(test)]
mod qualification {
    use super::*;
    #[test]
    fn concrete_layouts_and_ip_target_eligibility() {
        let target = Target::parse("http://127.0.0.1/a").unwrap();
        let future = transport::exchange(
            target,
            HttpArtifact::default(),
            Arc::new(AtomicUsize::new(0)),
        );
        println!(
            "Call={}/{} Client={}/{} Target={}/{} exchange={}/{}",
            size_of::<Call>(),
            align_of::<Call>(),
            size_of::<Client>(),
            align_of::<Client>(),
            size_of::<Target>(),
            align_of::<Target>(),
            size_of_val(&future),
            align_of_val(&future)
        );
        assert!(Target::parse("http://[::1]:8080/a?x=1").is_ok());
        for text in [
            "http://127.0.0.1:/a",
            "http://127.0.0.1/a#f",
            "http://127.0.0.1/{x}",
            "http://127.0.0.1:65536/a",
            "http://127.0.0.1:0/a",
            "http://localhost/a",
            "https://127.0.0.1/a",
            "http://user@127.0.0.1/a",
            "http://127.0.0.1/a\\b",
        ] {
            assert!(Target::parse(text).is_err(), "{text}");
        }
    }
}

mod producer_transport;

mod action_producer;
pub use action_producer::{HttpActionDefinition, HttpActionDelivery, HttpActionRuntime};
#[cfg(feature = "prepare")]
mod action_preparation;
#[cfg(feature = "prepare")]
pub use action_preparation::HttpActionProducer;

/// Public open Consumer registration: actual compiler/executor/configuration
/// plus the concrete JSON/type and cleartext-IP policy services.
#[cfg(feature = "prepare")]
pub fn try_consumer_registration(
    config: HttpConsumerConfig,
    binding_id: BindingId,
    generation: BindingGeneration,
    ordinal: u32,
) -> CoreResult<clinkz_wot_planning::open_consumer::ConsumerRegistrationInput> {
    use clinkz_wot_planning::*;
    let compiler = HostBindingCompilerRegistration::try_new_semantic(HttpCompiler(config))
        .map_err(|e| e.error().error().clone())?;
    let runtime = try_consumer_execution(config, binding_id, generation, ordinal)?;
    let preparation = HostConsumerBindingPreparation::try_pair(runtime, compiler)
        .map_err(|e| e.error().error().clone())?;
    Ok(open_consumer::ConsumerRegistrationInput {
        preparation,
        services: config.consumer_services(),
    })
}
impl HttpConsumerConfig {
    pub fn consumer_services(self) -> clinkz_wot_core::consumer::ConsumerServices {
        clinkz_wot_core::consumer::ConsumerServices {
            data: Arc::new(HttpConsumerData),
            access: Arc::new(HttpConsumerAccess(self)),
        }
    }
}
struct HttpConsumerData;
impl clinkz_wot_core::consumer::ConsumerDataService for HttpConsumerData {
    fn content_type(&self) -> &str {
        "application/json"
    }
    fn validate(
        &self,
        schema: clinkz_wot_core::value_type::TypeSchema,
        payload: &Payload,
    ) -> CoreResult<()> {
        // The JSON service evaluates a payload independently of its finite
        // input/output role. The operation owner checks result metadata/shape.
        producer_validation::validate_payload(schema, payload)
    }
    fn policy(&self) -> clinkz_wot_core::consumer::ConsumerDataPolicy {
        clinkz_wot_core::consumer::ConsumerDataPolicy {
            retained_bytes: 32,
            // Conservative bounded Host decoder scratch/stack allowance;
            // no allocator-intercepted or process-wide RAM claim.
            temporary_bytes: 65_536,
            body_bytes: BODY,
            validation_work: BODY * 5 + MEDIA * 2,
        }
    }
}
struct HttpConsumerAccess(HttpConsumerConfig);
impl clinkz_wot_core::consumer::ConsumerAccessService for HttpConsumerAccess {
    fn authorize_instance(
        &self,
        _consumer: &clinkz_wot_core::consumer::ConsumerInstanceRef,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        clinkz_wot_core::consumer::ConsumerAccessService::authorize(
            self, binding, target, operation,
        )
    }
    fn authorize(
        &self,
        binding: BindingRegistrationIdentity,
        target: &str,
        operation: Operation,
    ) -> CoreResult<()> {
        if binding.configuration() != self.0.configuration.configuration()
            || operation != Operation::ReadProperty
        {
            return Err(error(641, "HTTP Consumer policy/configuration mismatch"));
        }
        Target::parse(target).map(|_| ())
    }
    fn retained_bytes(&self) -> u64 {
        std::mem::size_of::<Self>() as u64 + 32
    }
    fn work(&self) -> u64 {
        256
    }
}
