//! Source-free Producer installation contracts. These values carry no serving
//! authority. Native installation and logical publication are separate events.
use crate::*;
use alloc::boxed::Box;
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
    property: Box<str>,
    handler: H,
    footprint: HandlerFootprint,
}
impl<H: ReadPropertyHandler> ProducerHandlerRegistration<H> {
    pub fn new(
        authority: &ProducerInstanceAuthority,
        property: Box<str>,
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
    pub fn into_parts(self) -> (ProducerInstanceKey, Box<str>, H, HandlerFootprint) {
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
    property: Box<str>,
    program: P,
}
impl<P> ProducerExecutionEntry<P> {
    pub fn new(
        instance: ProducerInstanceKey,
        binding: BindingRegistrationIdentity,
        property: Box<str>,
        program: P,
    ) -> Self {
        Self {
            instance,
            binding,
            property,
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
        &self.property
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
/// Host policy. No method accepts or dispatches a semantic request.
pub trait ProducerInstallation: Send + 'static {
    fn poll_install(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
    fn poll_close(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<()>>;
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
    entry: ProducerExecutionEntry<E::Program>,
    // Retained capability; installation cannot invoke it.
    #[allow(dead_code)]
    handler: H,
    footprint: HandlerFootprint,
}
impl<E: ProducerExecutor, H: ReadPropertyHandler> ProducerExecutionRegistration<E, H> {
    pub fn new(
        executor: E,
        configuration: E::Configuration,
        policy: ProducerInstallPolicy,
        entry: ProducerExecutionEntry<E::Program>,
        handler: H,
        footprint: HandlerFootprint,
    ) -> Self {
        Self {
            executor,
            configuration,
            policy,
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
    pub fn configuration(&self) -> &E::Configuration {
        &self.configuration
    }
    pub fn install_policy(&self) -> ProducerInstallPolicy {
        self.policy
    }
    pub fn check_current(&self) -> CoreResult<()> {
        if !self.executor.configuration_matches(&self.configuration)
            || self.executor.identity() != self.entry.binding()
            || self.executor.install_policy() != self.policy
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
