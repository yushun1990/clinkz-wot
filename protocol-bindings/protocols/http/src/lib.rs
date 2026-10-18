//! Optional Host HTTP/1.1 Consumer Property Read binding.
//!
//! This first slice supports cleartext IP literals, default JSON GET, status
//! 200, fixed-length/chunked/close-delimited responses and bounded discarded
//! trailers. See README.md for deployment and qualification limits.
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
#[cfg(test)]
mod identity_tests;
mod transport;

const BODY: u64 = 4096;
const MEDIA: u64 = 16;
// All immutable behavior (including dependency configuration) is versioned here.
// Bump this encoding when any supported behavior or private ceiling changes.
const PRIVATE_POLICY: &[u8] = concat!(
    "clinkz-http-ip-v5\0",
    "hyper1.12.0(client,http1);httparse1.10.1(std);hyper-util0.1.20(tokio);",
    "tokio1.52.3(net,rt);http-body-util0.1.3;bytes1.11.1;socket2-0.6.4;",
    "GET;NoSec;200;json-exact;noCE;noUpgrade;no1xx;http1.1;IP;target64;",
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
            configuration: ConsumerBindingConfiguration::try_new(
                BindingResourceContract::ConsumerOwnedHostV2,
                64,
                BindingConfigurationDigest::new(PRIVATE),
                BODY,
                MEDIA,
            )?
            .with_default_json_forms()?,
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
    target: Option<Target>,
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
    fn consumer_configuration(&self) -> Option<ConsumerBindingConfiguration> {
        Some(self.config.configuration)
    }
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        ResolvedTargetCompiler::new_owned(self.config.configuration).compatibility()
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
            .try_payload::<ResolvedTargetArtifact>(self.artifact_compatibility())
            .and_then(|a| Target::parse(a.target()).ok());
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
            let Some(target) = self.target.take() else {
                return Poll::Ready(Err(error(602, "HTTP call already retired")));
            };
            self.future = Some(Box::pin(transport::exchange(
                target,
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

/// Creates one complete restricted Consumer-only registration, without I/O.
/// Use with an application-owned Tokio runtime with its I/O driver enabled.
pub fn try_consumer_registration(
    config: HttpConsumerConfig,
    binding_id: BindingId,
    binding_generation: BindingGeneration,
    diagnostic_ordinal: u32,
) -> CoreResult<ConsumerCompilerRegistration<HostConsumerBindingRegistration>> {
    let compiler = ResolvedTargetCompiler::new_owned(config.configuration);
    let identity = BindingRegistrationIdentity::new(
        binding_id,
        binding_generation,
        compiler.configuration(),
        compiler.compatibility(),
        diagnostic_ordinal,
    );
    let resources = BindingResourceDeclarations::new(
        BindingLifetimeFootprint::new(3, std::mem::size_of::<Client>() as u64 + 32),
        BindingLifetimeFootprint::new(16, 8192),
    )
    .with_resource_contract(BindingResourceContract::ConsumerOwnedHostV2)
    .with_consumer_result_limits(BODY, MEDIA)
    .with_transient(BindingTransientFootprint::new(16384));
    let compiler = HostBindingCompilerRegistration::try_new_consumer(compiler)
        .map_err(|e| e.error().error().clone())?;
    HostConsumerBindingRegistration::new(HostConsumerBindingRegistrationInput::new(
        identity,
        BindingExecutionSupport::host_erased(),
        compiler,
        Box::new(Client {
            config,
            busy: Arc::new(AtomicBool::new(false)),
        }),
        resources,
        BindingStatusPolicy::new(2, 17),
    ))
    .map_err(|e| e.error().error().clone())?
    .try_into_consumer_compiler()
    .map_err(|e| e.error().error().clone())
}

#[cfg(test)]
mod qualification {
    use super::*;
    #[test]
    fn concrete_layouts_and_ip_target_eligibility() {
        let target = Target::parse("http://127.0.0.1/a").unwrap();
        let future = transport::exchange(target, Arc::new(AtomicUsize::new(0)));
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
