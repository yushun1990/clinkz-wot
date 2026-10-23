//! Source-free Producer execution contracts. Native installation, current
//! acceptance authority, and Core-sealed delivery have separate ownership.
//!
//! The current entry/request/registration family implements the admitted Host
//! single-route, synchronous Property Read slice. It is not a general multi-route
//! dispatch representation: requests have no operation/recipient discriminator.
//! This availability limit does not constrain future Engine composition, native
//! multi-recipient matching, shared infrastructure, or other WoT operations.
use crate::*;
use alloc::sync::Arc;
use core::task::{Context, Poll};

/// Copyable identity is diagnostic/lookup data, never serving authority. A new
/// owner cannot reuse an old key merely by copying its slot/generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProducerInstanceKey {
    slot: ThingSlotId,
    incarnation: usize,
}
impl ProducerInstanceKey {
    pub fn slot(self) -> ThingSlotId {
        self.slot
    }
}
/// Unique move-only local instance authority, independent of source identifiers
/// and physical protocol resources. Exhaustion refuses rather than wrapping.
#[derive(Debug)]
pub struct ProducerInstanceAuthority(ProducerInstanceKey);
impl ProducerInstanceAuthority {
    pub fn try_new(slot: ThingSlotId) -> CoreResult<Self> {
        use core::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(1);
        let incarnation = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| {
                CoreError::Backpressure(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
            })?;
        Ok(Self(ProducerInstanceKey { slot, incarnation }))
    }
    pub fn key(&self) -> ProducerInstanceKey {
        self.0
    }
}

/// Controlled indirect backing, temporary work and local destruction declared
/// by the actual executor. SDK-private resources follow the Binding's policy;
/// these numbers must not be used to claim transitive allocation freedom.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProducerInstallPolicy {
    pub executor_bytes: u64,
    pub program_bytes: u64,
    pub installation_bytes: u64,
    /// Maximum additional controlled backing during construction, polling or
    /// finite local disposal, simultaneous with the retained owner. Reserved in
    /// the Engine live peak as well as the callback-temporary account.
    pub temporary_bytes: u64,
    /// Largest single controlled indirect allocation, independently of sums.
    pub largest_indirect_allocation: u64,
    pub local_drop_work: u64,
}

/// One actual registered synchronous capability. Its key may be empty. The
/// slot and authority incarnation, rather than a source TD identifier, identify
/// the owning instance. The capability cannot silently move to a fresh owner.
pub struct ProducerHandlerRegistration<H: ReadPropertyHandler> {
    instance: ProducerInstanceKey,
    property: Arc<str>,
    handler: H,
    footprint: HandlerFootprint,
}
impl<H: ReadPropertyHandler> ProducerHandlerRegistration<H> {
    pub fn new(
        authority: &ProducerInstanceAuthority,
        property: Arc<str>,
        handler: H,
        footprint: HandlerFootprint,
    ) -> Self {
        Self {
            instance: authority.key(),
            property,
            handler,
            footprint,
        }
    }
    pub fn instance(&self) -> ThingSlotId {
        self.instance.slot()
    }
    pub fn instance_key(&self) -> ProducerInstanceKey {
        self.instance
    }
    pub fn property(&self) -> &str {
        &self.property
    }
    pub fn footprint(&self) -> HandlerFootprint {
        self.footprint
    }
    pub fn into_parts(self) -> (ProducerInstanceKey, Arc<str>, H, HandlerFootprint) {
        (self.instance, self.property, self.handler, self.footprint)
    }
}

/// Immutable Engine dispatch facts plus native instructions. Construction of
/// this data alone is neither a coverage certificate nor installation power.
/// Planning's private completed preparation and Servient's funding are required
/// before execution. No independent logical plan or plan-set is retained.
pub struct ProducerExecutionEntry<P> {
    instance: ProducerInstanceKey,
    binding: BindingRegistrationIdentity,
    target: AffordanceTarget,
    program: P,
}
impl<P> ProducerExecutionEntry<P> {
    pub fn new(
        instance: ProducerInstanceKey,
        binding: BindingRegistrationIdentity,
        property: Arc<str>,
        program: P,
    ) -> Self {
        Self {
            instance,
            binding,
            target: AffordanceTarget::Property(property),
            program,
        }
    }
    pub fn instance(&self) -> ThingSlotId {
        self.instance.slot()
    }
    pub fn instance_key(&self) -> ProducerInstanceKey {
        self.instance
    }
    pub fn binding(&self) -> BindingRegistrationIdentity {
        self.binding
    }
    pub fn property(&self) -> &str {
        self.target.name().unwrap()
    }
    pub fn target(&self) -> &AffordanceTarget {
        &self.target
    }
    pub fn program(&self) -> &P {
        &self.program
    }
    pub fn operation(&self) -> Operation {
        Operation::ReadProperty
    }
}

/// Binding-owned participation in native infrastructure. One value need not
/// represent one physical listener/session: implementations may hold a lease on
/// shared infrastructure. Close releases only this participation. Pending and
/// failure never surrender its resources. Drop must finish finite local
/// disposal without network waits, tasks or a blocking join under the declared
/// Host policy. Only a current permit permits semantic acceptance; no native
/// method dispatches an application handler.
pub trait ProducerInstallation: Send + 'static {
    fn poll_install(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    fn poll_close(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    /// A short permit authorizes only this candidate's semantic acceptance.
    /// Native observation/matching may be shared by many installations. The
    /// Binding retains correlation and physical reply ownership until delivery
    /// or close; it cannot call an application handler.
    fn poll_request(
        &mut self,
        _cx: &mut Context<'_>,
        _permit: ProducerAcceptPermit<'_>,
        _now: clinkz_wot_foundation::MonotonicInstant,
        _timeout_ticks: u64,
    ) -> Poll<CoreResult<ProducerRequest>> {
        Poll::Pending
    }
    /// Borrows the original Core-sealed response. Pending never transfers or
    /// releases it; the Engine retains it until terminal delivery or disposal
    /// of this native participation. No request/response future is detached.
    fn poll_response(
        &mut self,
        _cx: &mut Context<'_>,
        _response: &ProducerResponse,
        _now: clinkz_wot_foundation::MonotonicInstant,
    ) -> Poll<CoreResult<()>> {
        Poll::Pending
    }
    /// Passive clock-qualified deadline of any retained native request/reply.
    fn request_deadline(&self) -> Deadline {
        Deadline::NONE
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProducerServicePolicy {
    pub response_body_bytes: u64,
    pub response_media_capacity: u64,
}

/// Nonretained current-instance permission. Possession of a copied lookup key
/// cannot construct it: its lifetime borrows the move-only authority.
pub struct ProducerAcceptPermit<'a>(&'a ProducerInstanceAuthority);
impl ProducerAcceptPermit<'_> {
    pub fn instance(&self) -> ProducerInstanceKey {
        self.0.key()
    }
}
impl ProducerInstanceAuthority {
    /// Engine publication lends this only while the instance is current and
    /// Serving. Native installations never retain an authority reference.
    pub fn accept_permit(&mut self) -> ProducerAcceptPermit<'_> {
        ProducerAcceptPermit(self)
    }
}

/// One native correlation pinned to a logical instance. There is no source TD
/// identifier, independent plan-set identity, or transport payload for this
/// narrow bodyless Property Read request.
#[derive(Debug)]
pub struct ProducerRequest {
    instance: ProducerInstanceKey,
    correlation: CorrelationId,
}
impl ProducerRequest {
    pub fn new(instance: ProducerInstanceKey, correlation: CorrelationId) -> Self {
        Self {
            instance,
            correlation,
        }
    }
    pub fn instance(&self) -> ProducerInstanceKey {
        self.instance
    }
    pub fn correlation(&self) -> CorrelationId {
        self.correlation
    }
}

/// Core-sealed, single-use reply opportunity. Bindings cannot create an
/// unchecked success. The original request survives handler failure as well.
pub struct ProducerResponse {
    request: ProducerRequest,
    result: CoreResult<InteractionOutput>,
}
impl ProducerResponse {
    pub fn seal(
        request: ProducerRequest,
        result: CoreResult<InteractionOutput>,
        body_limit: u64,
        media_capacity: u64,
    ) -> Self {
        let result = result.and_then(|output| {
            let valid = crate::response::property_read_handler_shape(&output)
                && output.data().is_some_and(|p| {
                    p.body.len() as u64 <= body_limit
                        && p.content_type.capacity() as u64 <= media_capacity
                        && p.content_coding
                            .as_ref()
                            .is_none_or(|c| c.capacity() as u64 <= media_capacity)
                });
            if valid {
                Ok(output)
            } else {
                Err(CoreError::Validation(
                    ErrorContext::new(ErrorPhase::Validate, RetryClass::Never)
                        .with_thing(request.instance.slot())
                        .with_correlation(request.correlation),
                ))
            }
        });
        Self { request, result }
    }
    pub fn request(&self) -> &ProducerRequest {
        &self.request
    }
    pub fn result(&self) -> Result<&InteractionOutput, &CoreError> {
        self.result.as_ref()
    }
}

/// An actual captured executor, independently of optional preparation. Native
/// types and complete typed configuration are paired, then `check_program`
/// checks native operands; matching tags alone do not authorize execution.
/// Configuration matching, identity/policy queries and operand validation run
/// before funding: they must be passive and allocation-free. The captured
/// configuration is caller-provisioned and moves through preparation/runtime
/// ownership without requiring Clone or a duplicate buffer.
pub trait ProducerExecutor: Send + 'static {
    type Program: Send + 'static;
    type Configuration: Eq + Send + 'static;
    type Installation: ProducerInstallation;
    /// Compare the complete current configuration with the borrowed captured
    /// value without cloning it, allocating or performing native effects.
    fn configuration_matches(&self, captured: &Self::Configuration) -> bool;
    fn identity(&self) -> BindingRegistrationIdentity;
    fn install_policy(&self) -> ProducerInstallPolicy;
    fn service_policy(&self) -> Option<ProducerServicePolicy> {
        None
    }
    /// Pure validation of native operands and complete controlled backing
    /// against the captured policy. It must not perform native installation.
    fn check_program(&self, program: &Self::Program) -> CoreResult<()>;
    /// Pure construction, after Engine admission. No socket, registration or
    /// other native effect may occur here. All partial native effects belong to
    /// the returned installation, starting with its first funded install poll.
    fn create_installation(
        &self,
        entry: &ProducerExecutionEntry<Self::Program>,
    ) -> CoreResult<Self::Installation>;
}

/// Captured runtime ownership, below the preparation dependency boundary.
/// This data alone is not a complete-description coverage certificate. The
/// Engine must receive a completed preparation before admitting installation.
pub struct ProducerExecutionRegistration<E: ProducerExecutor, H: ReadPropertyHandler> {
    executor: E,
    configuration: E::Configuration,
    policy: ProducerInstallPolicy,
    service: Option<ProducerServicePolicy>,
    entry: ProducerExecutionEntry<E::Program>,
    // Retained capability; installation cannot invoke it.
    handler: H,
    footprint: HandlerFootprint,
}
impl<E: ProducerExecutor, H: ReadPropertyHandler> ProducerExecutionRegistration<E, H> {
    pub fn new(
        executor: E,
        configuration: E::Configuration,
        policy: ProducerInstallPolicy,
        service: Option<ProducerServicePolicy>,
        entry: ProducerExecutionEntry<E::Program>,
        handler: H,
        footprint: HandlerFootprint,
    ) -> Self {
        Self {
            executor,
            configuration,
            policy,
            service,
            entry,
            handler,
            footprint,
        }
    }
    pub fn entry(&self) -> &ProducerExecutionEntry<E::Program> {
        &self.entry
    }
    pub fn footprint(&self) -> HandlerFootprint {
        self.footprint
    }
    pub fn dispatch(&self, context: HandlerContext<'_>) -> CoreResult<InteractionOutput> {
        self.handler.handle(context, &InteractionInput::default())
    }
    pub fn configuration(&self) -> &E::Configuration {
        &self.configuration
    }
    pub fn install_policy(&self) -> ProducerInstallPolicy {
        self.policy
    }
    pub fn service_policy(&self) -> Option<ProducerServicePolicy> {
        self.service
    }
    pub fn check_current(&self) -> CoreResult<()> {
        if !self.executor.configuration_matches(&self.configuration)
            || self.executor.identity() != self.entry.binding()
            || self.executor.install_policy() != self.policy
            || self.executor.service_policy() != self.service
        {
            return Err(CoreError::Validation(
                ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
                    .with_redacted_cause(622, "Producer captured executor changed"),
            ));
        }
        self.executor.check_program(self.entry.program())
    }
    /// Pure driver construction. The Engine funds all owner and cleanup backing
    /// before calling, then retains the result through native setup/retirement.
    pub fn create_installation(&self) -> CoreResult<E::Installation> {
        self.check_current()?;
        self.executor.create_installation(&self.entry)
    }
}
