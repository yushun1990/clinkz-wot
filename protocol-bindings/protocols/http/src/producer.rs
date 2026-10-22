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
#[derive(Debug, Eq, PartialEq)]
pub struct HttpProducerProgram {
    pub(super) address: SocketAddr,
    pub(super) target: [u8; 64],
    pub(super) target_len: u8,
    pub(super) authority_end: u8,
    pub(super) origin: [u8; 64],
    pub(super) origin_len: u8,
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
// transitive disposal claim. No semantic ingress/response SDK is installed yet.
const INSTALL_POLICY: &[u8] = b"clinkz-http-producer-install-v1;std-TcpListener;loopback-nonzero-unscoped;exclusive-native-listener;no-accept;no-serving;controlled-indirect=0/0/0;temporary=1024-not-stack-ceiling;largest-indirect=0;local-drop-work=1;finite-SDK-local-drop;no-task;no-network-wait";

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
            BindingArtifactCompatibility::new(*b"http-prod-ins-v1"),
            ordinal,
        ),
    }
}
impl ProducerExecutor for HttpProducerExecutor {
    type Configuration = HttpProducerConfig;
    type Program = HttpProducerProgram;
    type Installation = HttpProducerInstallation;
    fn configuration(&self) -> Self::Configuration {
        self.config
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
            listener: None,
            failed: None,
            closed: false,
        })
    }
}
/// Binding-owned native setup/retirement. This type deliberately exposes no
/// accept, handler, response or publication operation. Closed-gate connections
/// remain deployment/kernel state until local disposal closes the listener.
pub struct HttpProducerInstallation {
    config: HttpProducerConfig,
    identity: BindingRegistrationIdentity,
    instance: ThingSlotId,
    listener: Option<TcpListener>,
    failed: Option<CoreError>,
    closed: bool,
}
impl HttpProducerInstallation {
    fn failure(&mut self) -> CoreError {
        let error = CoreError::Binding(
            ErrorContext::new(ErrorPhase::Prepare, RetryClass::Never)
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
                .with_redacted_cause(623, "HTTP Producer native installation failed"),
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
        let listener = match TcpListener::bind(self.config.address) {
            Ok(listener) => listener,
            Err(_) => return Poll::Ready(Err(self.failure())),
        };
        // Retain the native owner before any fallible post-bind configuration.
        self.listener = Some(listener);
        if self
            .listener
            .as_ref()
            .unwrap()
            .set_nonblocking(true)
            .is_err()
        {
            return Poll::Ready(Err(self.failure()));
        }
        Poll::Ready(Ok(()))
    }
    fn poll_close(&mut self, _cx: &mut Context<'_>) -> Poll<CoreResult<()>> {
        self.closed = true;
        drop(self.listener.take());
        Poll::Ready(Ok(()))
    }
}
impl Drop for HttpProducerInstallation {
    fn drop(&mut self) {
        drop(self.listener.take());
    }
}
