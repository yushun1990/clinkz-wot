//! Checked pairing of optional compilers with runtime-only registrations.
use crate::binding_compiler::*;
#[cfg(feature = "std")]
use alloc::boxed::Box;
use clinkz_wot_core::*;
use clinkz_wot_foundation::WorkBudget;
use core::{
    ops::Deref,
    task::{Context, Poll},
};

/// Application preparation facet; the executor knows only its native artifact.
pub trait PrepareServerBinding: PollServerBinding {
    type Compiler: BindingCompilerExtension<Artifact = Self::Artifact>;
}
/// Application preparation facet for a static client.
pub trait PrepareClientBinding: PollClientBinding {
    type Compiler: BindingCompilerExtension<Artifact = Self::Artifact>;
}
impl<S, C> PrepareServerBinding for StaticBindingComponents<S, C>
where
    S: PrepareServerBinding,
    C: PollClientBinding<Artifact = S::Artifact>,
{
    type Compiler = S::Compiler;
}
fn registration_error(identity: BindingRegistrationIdentity, code: u16) -> BindingOperationalError {
    BindingOperationalError::new(CoreError::Validation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_binding(identity.binding_id(), identity.binding_generation())
            .with_redacted_cause(code, "compiler and executor declarations are inconsistent"),
    ))
}
/// One preparation registration and Planning-issued closed Consumer support.
///
/// Borrowing the registration's compiler keeps the complete owner alive. The
/// descriptor is metadata, not a certificate that can be paired with another
/// owner. Only consuming capture below can construct this carrier.
///
/// ```compile_fail
/// # use clinkz_wot_planning::{InlineConsumerCompilerRegistration, InlineConsumerCompilerSupport};
/// let forged = InlineConsumerCompilerRegistration { registration: (), support: unsafe { core::mem::zeroed::<InlineConsumerCompilerSupport>() } };
/// ```
///
/// ```compile_fail
/// # use clinkz_wot_planning::InlineConsumerCompilerRegistration;
/// # fn reuse<R>(checked: InlineConsumerCompilerRegistration<R>) {
/// let original = checked.into_registration();
/// let reused = checked.support();
/// # }
/// ```
#[derive(Debug)]
pub struct InlineConsumerCompilerRegistration<R> {
    registration: R,
    support: InlineConsumerCompilerSupport,
}

impl<R> InlineConsumerCompilerRegistration<R> {
    /// Borrows the original validated complete registration.
    pub const fn registration(&self) -> &R {
        &self.registration
    }
    /// Borrows its read-only, source-owned support descriptor.
    pub const fn support(&self) -> &InlineConsumerCompilerSupport {
        &self.support
    }
    /// Consumes support and returns the original complete registration.
    pub fn into_registration(self) -> R {
        self.registration
    }
}

fn static_consumer_support<C: BindingCompilerExtension + 'static>(
    identity: BindingRegistrationIdentity,
    execution: BindingExecutionSupport,
    capabilities: BindingRegistrationCapabilities,
    compiler: &StaticBindingCompilerRegistration<C>,
    resources: BindingResourceDeclarations,
    client: Option<ConsumerBindingConfiguration>,
) -> Option<InlineConsumerCompilerSupport> {
    let actual =
        (compiler.compiler() as &dyn core::any::Any).downcast_ref::<ResolvedTargetCompiler>();
    if capabilities.supports_consumer_property_read()
        && execution.supports_application_static()
        && let Some(actual) = actual
        && identity.configuration() == actual.configuration()
        && consumer_metadata_matches(identity, resources, client)
        && actual.support(false).consumer_configuration() == client
        && identity.artifact_compatibility()
            == crate::binding_compiler::resolved_target_compatibility()
    {
        Some(actual.support(false))
    } else {
        None
    }
}

#[cfg(feature = "std")]
fn host_consumer_support(
    identity: BindingRegistrationIdentity,
    execution: BindingExecutionSupport,
    capabilities: BindingRegistrationCapabilities,
    compiler: &HostBindingCompilerRegistration,
    resources: BindingResourceDeclarations,
    client: Option<ConsumerBindingConfiguration>,
) -> Option<InlineConsumerCompilerSupport> {
    if capabilities.supports_consumer_property_read()
        && execution.supports_host_erased()
        && let Some(support) = compiler.consumer_support()
        && identity.configuration() == support.configuration()
        && consumer_metadata_matches(identity, resources, client)
        && support.consumer_configuration() == client
        && identity.artifact_compatibility() == support.compatibility()
    {
        Some(support)
    } else {
        None
    }
}

fn consumer_metadata_matches(
    identity: BindingRegistrationIdentity,
    resources: BindingResourceDeclarations,
    client: Option<ConsumerBindingConfiguration>,
) -> bool {
    match (resources.resource_contract(), client) {
        (BindingResourceContract::StrictV1, None) => true,
        (BindingResourceContract::StrictV1, Some(_)) | (_, None) => false,
        (contract, Some(actual)) => {
            actual.resource_contract() == contract
                && actual.configuration() == identity.configuration()
                && resources.consumer_result_limits() == Some(actual.result_limits())
        }
    }
}

pub struct StaticBindingPreparationInput<B: PrepareServerBinding> {
    compiler: StaticBindingCompilerRegistration<B::Compiler>,
    runtime: StaticBindingRegistrationInput<B>,
}
impl<B: PrepareServerBinding> StaticBindingPreparationInput<B> {
    pub fn new(
        identity: BindingRegistrationIdentity,
        capabilities: BindingRegistrationCapabilities,
        execution: BindingExecutionSupport,
        compiler: StaticBindingCompilerRegistration<B::Compiler>,
        server: B,
        resources: BindingResourceDeclarations,
        ingress: BindingIngressPolicy,
        status: BindingStatusPolicy,
    ) -> Self {
        Self {
            compiler,
            runtime: StaticBindingRegistrationInput::new(
                identity,
                capabilities,
                execution,
                server,
                resources,
                ingress,
                status,
            ),
        }
    }
    pub const fn identity(&self) -> BindingRegistrationIdentity {
        self.runtime.identity()
    }
    pub const fn compiler(&self) -> &StaticBindingCompilerRegistration<B::Compiler> {
        &self.compiler
    }
    pub fn server(&self) -> &B {
        self.runtime.server()
    }
}
pub struct StaticBindingPreparation<B: PrepareServerBinding> {
    compiler: StaticBindingCompilerRegistration<B::Compiler>,
    runtime: StaticBindingRegistration<B>,
}
impl<B: PrepareServerBinding> StaticBindingPreparation<B> {
    pub fn new(
        input: StaticBindingPreparationInput<B>,
    ) -> Result<Self, BindingInputRejection<StaticBindingPreparationInput<B>>> {
        let identity = input.identity();
        if identity.artifact_compatibility() != input.compiler.compiler().compatibility() {
            return Err(BindingInputRejection::new(
                input,
                registration_error(identity, 330),
            ));
        }
        let StaticBindingPreparationInput { compiler, runtime } = input;
        match StaticBindingRegistration::new(runtime) {
            Ok(runtime) => Ok(Self { compiler, runtime }),
            Err(rejected) => {
                let (runtime, error) = rejected.into_parts();
                Err(BindingInputRejection::new(
                    StaticBindingPreparationInput { compiler, runtime },
                    error,
                ))
            }
        }
    }
    pub const fn compiler(&self) -> &StaticBindingCompilerRegistration<B::Compiler> {
        &self.compiler
    }
    pub fn into_execution(self) -> StaticBindingRegistration<B> {
        self.runtime
    }
    pub fn into_input(self) -> StaticBindingPreparationInput<B> {
        StaticBindingPreparationInput {
            compiler: self.compiler,
            runtime: self.runtime.into_input(),
        }
    }
    pub fn try_into_inline_consumer_compiler(
        self,
    ) -> Result<InlineConsumerCompilerRegistration<Self>, BindingInputRejection<Self>>
    where
        B::Compiler: 'static,
    {
        if let Some(support) = static_consumer_support(
            self.identity(),
            self.execution(),
            self.capabilities(),
            &self.compiler,
            self.resources(),
            None,
        ) {
            return Ok(InlineConsumerCompilerRegistration {
                registration: self,
                support,
            });
        }
        let error = registration_error(self.identity(), 323);
        Err(BindingInputRejection::new(self, error))
    }
}
impl<B: PrepareServerBinding> Deref for StaticBindingPreparation<B> {
    type Target = StaticBindingRegistration<B>;
    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

#[cfg(feature = "std")]
pub struct HostBindingPreparationInput {
    compiler: HostBindingCompilerRegistration,
    runtime: HostBindingRegistrationInput,
}
#[cfg(feature = "std")]
impl HostBindingPreparationInput {
    pub fn new(
        identity: BindingRegistrationIdentity,
        capabilities: BindingRegistrationCapabilities,
        execution: BindingExecutionSupport,
        compiler: HostBindingCompilerRegistration,
        server: Box<dyn RouteServerBinding>,
        resources: BindingResourceDeclarations,
        ingress: BindingIngressPolicy,
        status: BindingStatusPolicy,
    ) -> Self {
        Self {
            compiler,
            runtime: HostBindingRegistrationInput::new(
                identity,
                capabilities,
                execution,
                server,
                resources,
                ingress,
                status,
            ),
        }
    }
    pub const fn identity(&self) -> BindingRegistrationIdentity {
        self.runtime.identity()
    }
    pub const fn compiler(&self) -> &HostBindingCompilerRegistration {
        &self.compiler
    }
    pub fn server(&self) -> &dyn RouteServerBinding {
        self.runtime.server()
    }
}
#[cfg(feature = "std")]
pub struct HostBindingPreparation {
    compiler: HostBindingCompilerRegistration,
    runtime: HostBindingRegistration,
}
#[cfg(feature = "std")]
impl HostBindingPreparation {
    pub fn new(
        input: HostBindingPreparationInput,
    ) -> Result<Self, BindingInputRejection<HostBindingPreparationInput>> {
        let identity = input.identity();
        if identity.artifact_compatibility() != input.compiler.compatibility() {
            return Err(BindingInputRejection::new(
                input,
                registration_error(identity, 330),
            ));
        }
        let HostBindingPreparationInput { compiler, runtime } = input;
        match HostBindingRegistration::new(runtime) {
            Ok(runtime) => Ok(Self { compiler, runtime }),
            Err(rejected) => {
                let (runtime, error) = rejected.into_parts();
                Err(BindingInputRejection::new(
                    HostBindingPreparationInput { compiler, runtime },
                    error,
                ))
            }
        }
    }
    pub const fn compiler(&self) -> &HostBindingCompilerRegistration {
        &self.compiler
    }
    pub fn into_execution(self) -> HostBindingRegistration {
        self.runtime
    }
    pub fn into_input(self) -> HostBindingPreparationInput {
        HostBindingPreparationInput {
            compiler: self.compiler,
            runtime: self.runtime.into_input(),
        }
    }
    pub fn try_into_inline_consumer_compiler(
        self,
    ) -> Result<InlineConsumerCompilerRegistration<Self>, BindingInputRejection<Self>> {
        if let Some(support) = host_consumer_support(
            self.identity(),
            self.execution(),
            self.capabilities(),
            &self.compiler,
            self.resources(),
            None,
        ) {
            return Ok(InlineConsumerCompilerRegistration {
                registration: self,
                support,
            });
        }
        let error = registration_error(self.identity(), 323);
        Err(BindingInputRejection::new(self, error))
    }
}
#[cfg(feature = "std")]
impl Deref for HostBindingPreparation {
    type Target = HostBindingRegistration;
    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}
pub struct StaticConsumerBindingPreparationInput<B: PrepareClientBinding> {
    compiler: StaticBindingCompilerRegistration<B::Compiler>,
    runtime: StaticConsumerBindingRegistrationInput<B>,
}
impl<B: PrepareClientBinding> StaticConsumerBindingPreparationInput<B> {
    pub fn new(
        identity: BindingRegistrationIdentity,
        execution: BindingExecutionSupport,
        compiler: StaticBindingCompilerRegistration<B::Compiler>,
        client: B,
        resources: BindingResourceDeclarations,
        status: BindingStatusPolicy,
    ) -> Self {
        Self {
            compiler,
            runtime: StaticConsumerBindingRegistrationInput::new(
                identity, execution, client, resources, status,
            ),
        }
    }
    pub const fn identity(&self) -> BindingRegistrationIdentity {
        self.runtime.identity()
    }
    pub const fn compiler(&self) -> &StaticBindingCompilerRegistration<B::Compiler> {
        &self.compiler
    }
    pub fn client(&self) -> &B {
        self.runtime.client()
    }
}
/// Paired preparation does not lend mutable whole-registration replacement.
/// ```compile_fail
/// # use clinkz_wot_core::{PollClientBinding, StaticConsumerBindingRegistration};
/// # use clinkz_wot_planning::{PrepareClientBinding, StaticConsumerBindingPreparation};
/// # fn replace<C:PrepareClientBinding>(prepared:&mut StaticConsumerBindingPreparation<C>,other:StaticConsumerBindingRegistration<C>) {
/// core::mem::replace(&mut **prepared,other);
/// # }
/// ```
pub struct StaticConsumerBindingPreparation<B: PrepareClientBinding> {
    compiler: StaticBindingCompilerRegistration<B::Compiler>,
    runtime: StaticConsumerBindingRegistration<B>,
}
impl<B: PrepareClientBinding> StaticConsumerBindingPreparation<B> {
    pub fn new(
        input: StaticConsumerBindingPreparationInput<B>,
    ) -> Result<Self, BindingInputRejection<StaticConsumerBindingPreparationInput<B>>> {
        let identity = input.identity();
        if identity.artifact_compatibility() != input.compiler.compiler().compatibility() {
            return Err(BindingInputRejection::new(
                input,
                registration_error(identity, 330),
            ));
        }
        let StaticConsumerBindingPreparationInput { compiler, runtime } = input;
        match StaticConsumerBindingRegistration::new(runtime) {
            Ok(runtime) => Ok(Self { compiler, runtime }),
            Err(rejected) => {
                let (runtime, error) = rejected.into_parts();
                Err(BindingInputRejection::new(
                    StaticConsumerBindingPreparationInput { compiler, runtime },
                    error,
                ))
            }
        }
    }
    pub const fn compiler(&self) -> &StaticBindingCompilerRegistration<B::Compiler> {
        &self.compiler
    }
    pub fn into_execution(self) -> StaticConsumerBindingRegistration<B> {
        self.runtime
    }
    pub fn into_input(self) -> StaticConsumerBindingPreparationInput<B> {
        StaticConsumerBindingPreparationInput {
            compiler: self.compiler,
            runtime: self.runtime.into_input(),
        }
    }
    pub fn try_into_inline_consumer_compiler(
        self,
    ) -> Result<InlineConsumerCompilerRegistration<Self>, BindingInputRejection<Self>>
    where
        B::Compiler: 'static,
    {
        if let Some(support) = static_consumer_support(
            self.identity(),
            self.execution(),
            self.capabilities(),
            &self.compiler,
            self.resources(),
            self.runtime.consumer_configuration(),
        ) {
            return Ok(InlineConsumerCompilerRegistration {
                registration: self,
                support,
            });
        }
        let error = registration_error(self.identity(), 323);
        Err(BindingInputRejection::new(self, error))
    }
}
impl<B: PrepareClientBinding> Deref for StaticConsumerBindingPreparation<B> {
    type Target = StaticConsumerBindingRegistration<B>;
    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

#[cfg(feature = "std")]
pub struct HostConsumerBindingPreparationInput {
    compiler: HostBindingCompilerRegistration,
    runtime: HostConsumerBindingRegistrationInput,
}
#[cfg(feature = "std")]
impl HostConsumerBindingPreparationInput {
    pub fn new(
        identity: BindingRegistrationIdentity,
        execution: BindingExecutionSupport,
        compiler: HostBindingCompilerRegistration,
        client: Box<dyn clinkz_wot_core::binding::ClientBinding>,
        resources: BindingResourceDeclarations,
        status: BindingStatusPolicy,
    ) -> Self {
        Self {
            compiler,
            runtime: HostConsumerBindingRegistrationInput::new(
                identity, execution, client, resources, status,
            ),
        }
    }
    pub const fn identity(&self) -> BindingRegistrationIdentity {
        self.runtime.identity()
    }
    pub const fn compiler(&self) -> &HostBindingCompilerRegistration {
        &self.compiler
    }
    pub fn client(&self) -> &dyn clinkz_wot_core::binding::ClientBinding {
        self.runtime.client()
    }
}
#[cfg(feature = "std")]
pub struct HostConsumerBindingPreparation {
    compiler: HostBindingCompilerRegistration,
    runtime: HostConsumerBindingRegistration,
}
#[cfg(feature = "std")]
impl HostConsumerBindingPreparation {
    pub fn new(
        input: HostConsumerBindingPreparationInput,
    ) -> Result<Self, BindingInputRejection<HostConsumerBindingPreparationInput>> {
        let identity = input.identity();
        if identity.artifact_compatibility() != input.compiler.compatibility() {
            return Err(BindingInputRejection::new(
                input,
                registration_error(identity, 330),
            ));
        }
        let HostConsumerBindingPreparationInput { compiler, runtime } = input;
        match HostConsumerBindingRegistration::new(runtime) {
            Ok(runtime) => Ok(Self { compiler, runtime }),
            Err(rejected) => {
                let (runtime, error) = rejected.into_parts();
                Err(BindingInputRejection::new(
                    HostConsumerBindingPreparationInput { compiler, runtime },
                    error,
                ))
            }
        }
    }
    pub const fn compiler(&self) -> &HostBindingCompilerRegistration {
        &self.compiler
    }
    pub fn into_execution(self) -> HostConsumerBindingRegistration {
        self.runtime
    }
    pub fn into_input(self) -> HostConsumerBindingPreparationInput {
        HostConsumerBindingPreparationInput {
            compiler: self.compiler,
            runtime: self.runtime.into_input(),
        }
    }
    pub fn try_into_inline_consumer_compiler(
        self,
    ) -> Result<InlineConsumerCompilerRegistration<Self>, BindingInputRejection<Self>> {
        if let Some(support) = host_consumer_support(
            self.identity(),
            self.execution(),
            self.capabilities(),
            &self.compiler,
            self.resources(),
            self.runtime.consumer_configuration(),
        ) {
            return Ok(InlineConsumerCompilerRegistration {
                registration: self,
                support,
            });
        }
        let error = registration_error(self.identity(), 323);
        Err(BindingInputRejection::new(self, error))
    }
}
#[cfg(feature = "std")]
impl Deref for HostConsumerBindingPreparation {
    type Target = HostConsumerBindingRegistration;
    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}
impl<S: PrepareServerBinding, C: PollClientBinding<Artifact = S::Artifact>>
    StaticBindingPreparationInput<StaticBindingComponents<S, C>>
{
    pub fn producer_and_consumer_property_read(
        identity: BindingRegistrationIdentity,
        execution: BindingExecutionSupport,
        compiler: StaticBindingCompilerRegistration<S::Compiler>,
        components: StaticBindingComponents<S, C>,
        resources: BindingResourceDeclarations,
        ingress: BindingIngressPolicy,
        status: BindingStatusPolicy,
    ) -> Self {
        Self {
            compiler,
            runtime: StaticBindingRegistrationInput::producer_and_consumer_property_read(
                identity, execution, components, resources, ingress, status,
            ),
        }
    }
}
impl<S: PrepareServerBinding, C: PollClientBinding<Artifact = S::Artifact>>
    StaticBindingPreparation<StaticBindingComponents<S, C>>
{
    pub fn producer_and_consumer_property_read(
        input: StaticBindingPreparationInput<StaticBindingComponents<S, C>>,
    ) -> Result<
        Self,
        BindingInputRejection<StaticBindingPreparationInput<StaticBindingComponents<S, C>>>,
    > {
        let identity = input.identity();
        if identity.artifact_compatibility() != input.compiler.compiler().compatibility() {
            return Err(BindingInputRejection::new(
                input,
                registration_error(identity, 330),
            ));
        }
        let StaticBindingPreparationInput { compiler, runtime } = input;
        match StaticBindingRegistration::producer_and_consumer_property_read(runtime) {
            Ok(runtime) => Ok(Self { compiler, runtime }),
            Err(rejected) => {
                let (runtime, error) = rejected.into_parts();
                Err(BindingInputRejection::new(
                    StaticBindingPreparationInput { compiler, runtime },
                    error,
                ))
            }
        }
    }
}
#[cfg(feature = "std")]
impl HostBindingPreparationInput {
    pub fn producer_and_consumer_property_read(
        identity: BindingRegistrationIdentity,
        execution: BindingExecutionSupport,
        compiler: HostBindingCompilerRegistration,
        server: Box<dyn RouteServerBinding>,
        client: Box<dyn clinkz_wot_core::binding::ClientBinding>,
        resources: BindingResourceDeclarations,
        ingress: BindingIngressPolicy,
        status: BindingStatusPolicy,
    ) -> Self {
        Self {
            compiler,
            runtime: HostBindingRegistrationInput::producer_and_consumer_property_read(
                identity, execution, server, client, resources, ingress, status,
            ),
        }
    }
}

#[cfg(feature = "std")]
impl HostConsumerBindingPreparation {
    /// Captures the actual immutable executor policy, preserving both owners on refusal.
    pub fn try_pair(
        runtime: HostConsumerBindingRegistration,
        compiler: HostBindingCompilerRegistration,
    ) -> Result<
        Self,
        BindingInputRejection<(
            HostConsumerBindingRegistration,
            HostBindingCompilerRegistration,
        )>,
    > {
        let identity = runtime.identity();
        if identity.artifact_compatibility() != compiler.compatibility()
            || compiler.consumer_metadata().is_some_and(|metadata| {
                metadata.configuration.configuration() != identity.configuration()
                    || Some(metadata.configuration) != runtime.consumer_configuration()
            })
            || compiler.consumer_support().is_some_and(|support| {
                support.configuration() != identity.configuration()
                    || support.consumer_configuration() != runtime.consumer_configuration()
            })
            || !consumer_metadata_matches(
                identity,
                runtime.resources(),
                runtime.consumer_configuration(),
            )
        {
            return Err(BindingInputRejection::new(
                (runtime, compiler),
                registration_error(identity, 330),
            ));
        }
        Ok(Self { runtime, compiler })
    }
}

impl<B: PrepareServerBinding> StaticBindingPreparation<B> {
    /// Lends bounded server progress without exposing either paired executor.
    ///
    /// ```compile_fail,E0308
    /// # use clinkz_wot_planning::{PrepareServerBinding, StaticBindingPreparation};
    /// fn replace<B: PrepareServerBinding>(preparation: &mut StaticBindingPreparation<B>, other: B) {
    ///     core::mem::replace(preparation.server_mut(), other);
    /// }
    /// ```
    /// ```compile_fail,E0308
    /// # use clinkz_wot_core::{PollClientBinding, StaticBindingComponents};
    /// # use clinkz_wot_planning::{PrepareServerBinding, StaticBindingPreparation};
    /// fn replace<S, C>(
    ///     preparation: &mut StaticBindingPreparation<StaticBindingComponents<S, C>>,
    ///     other: StaticBindingComponents<S, C>,
    /// ) where S: PrepareServerBinding, C: PollClientBinding<Artifact = S::Artifact> {
    ///     core::mem::replace(preparation.server_mut(), other);
    /// }
    /// ```
    /// ```
    /// # use clinkz_wot_core::PollServerBinding;
    /// # use clinkz_wot_planning::{PrepareServerBinding, StaticBindingPreparation};
    /// fn progress<B: PrepareServerBinding>(preparation: &mut StaticBindingPreparation<B>) {
    ///     let server = preparation.server_mut();
    ///     let _ = server.artifact_compatibility();
    /// }
    /// ```
    pub fn server_mut(&mut self) -> StaticServerProgress<'_, B> {
        self.runtime.server_mut()
    }
}

impl<S: PrepareServerBinding, C: PollClientBinding<Artifact = S::Artifact>>
    StaticBindingPreparation<StaticBindingComponents<S, C>>
{
    pub fn start_consumer_property_read(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<S::Artifact>,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        self.runtime
            .start_consumer_property_read(request, artifact, slot, budget)
    }
    pub fn poll_consumer_property_read(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        self.runtime.poll_consumer_property_read(cx, slot, budget)
    }
    pub fn start_cancel_consumer_property_read(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.runtime
            .start_cancel_consumer_property_read(cx, cleanup, slot, budget)
    }
    pub fn poll_cancel_consumer_property_read(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.runtime
            .poll_cancel_consumer_property_read(cx, slot, budget)
    }
    pub fn acknowledge_consumer_property_read(
        &mut self,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
    ) -> CoreResult<()> {
        self.runtime.acknowledge_consumer_property_read(slot)
    }
}

impl<C: PrepareClientBinding> StaticConsumerBindingPreparation<C> {
    pub fn start_consumer_property_read(
        &mut self,
        request: OutboundRequest,
        artifact: &BindingArtifactEnvelope<C::Artifact>,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
        budget: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        self.runtime
            .start_consumer_property_read(request, artifact, slot, budget)
    }
    pub fn poll_consumer_property_read(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        self.runtime.poll_consumer_property_read(cx, slot, budget)
    }
    pub fn start_cancel_consumer_property_read(
        &mut self,
        cx: &mut Context<'_>,
        cleanup: CleanupPhaseContext,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
        budget: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.runtime
            .start_cancel_consumer_property_read(cx, cleanup, slot, budget)
    }
    pub fn poll_cancel_consumer_property_read(
        &mut self,
        cx: &mut Context<'_>,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
        budget: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        self.runtime
            .poll_cancel_consumer_property_read(cx, slot, budget)
    }
    pub fn acknowledge_consumer_property_read(
        &mut self,
        slot: &mut StaticConsumerPropertyReadSlot<C::RequestState>,
    ) -> CoreResult<()> {
        self.runtime.acknowledge_consumer_property_read(slot)
    }
}

#[cfg(feature = "std")]
impl HostConsumerBindingPreparation {
    /// Consume the optional preparation half; only execution survives below it.
    pub fn into_parts(
        self,
    ) -> (
        HostConsumerBindingRegistration,
        HostBindingCompilerRegistration,
    ) {
        (self.runtime, self.compiler)
    }
}

impl<C: PrepareClientBinding> StaticConsumerBindingPreparation<C>
where
    C::Compiler: ConsumerCompiler,
{
    /// General typed/static capture uses the actual portable compiler contract,
    /// never ResolvedTargetCompiler identity or the inline support descriptor.
    pub fn consumer_contract(&self) -> CoreResult<ConsumerCompilerContract> {
        if self.compiler.compiler().configuration().configuration()
            != self.runtime.identity().configuration()
            || Some(self.compiler.compiler().configuration())
                != self.runtime.consumer_configuration()
        {
            return Err(registration_error(self.runtime.identity(), 330)
                .error()
                .clone());
        }
        Ok(<C::Compiler as ConsumerCompiler>::CONTRACT)
    }
}

impl<C: PrepareClientBinding> StaticConsumerBindingPreparation<C>
where
    C::Compiler: ConsumerCompiler,
{
    /// Recoverably pairs any portable Consumer compiler with its actual typed
    /// executor/configuration. No private inline support or compiler-name test.
    pub fn try_pair(
        runtime: StaticConsumerBindingRegistration<C>,
        compiler: StaticBindingCompilerRegistration<C::Compiler>,
    ) -> Result<
        Self,
        BindingInputRejection<(
            StaticConsumerBindingRegistration<C>,
            StaticBindingCompilerRegistration<C::Compiler>,
        )>,
    > {
        let identity = runtime.identity();
        let configuration = compiler.compiler().configuration();
        if Some(configuration) != runtime.consumer_configuration()
            || configuration.configuration() != identity.configuration()
            || compiler.compiler().compatibility() != identity.artifact_compatibility()
        {
            return Err(BindingInputRejection::new(
                (runtime, compiler),
                registration_error(identity, 330),
            ));
        }
        Ok(Self { runtime, compiler })
    }
}
