//! Checked pairing of optional compilers with runtime-only registrations.
use crate::binding_compiler::*;
#[cfg(feature = "std")]
use alloc::boxed::Box;
use clinkz_wot_core::*;
use core::ops::Deref;

/// Application preparation facet; the executor knows only its native artifact.
pub trait PrepareServerBinding: PollServerBinding {
    type Compiler: BindingCompilerExtension<Artifact = Self::Artifact>;
}
fn registration_error(identity: BindingRegistrationIdentity, code: u16) -> BindingOperationalError {
    BindingOperationalError::new(CoreError::Validation(
        ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
            .with_binding(identity.binding_id(), identity.binding_generation())
            .with_redacted_cause(code, "compiler and executor declarations are inconsistent"),
    ))
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
}
#[cfg(feature = "std")]
impl Deref for HostBindingPreparation {
    type Target = HostBindingRegistration;
    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}
#[cfg(feature = "std")]
pub struct HostConsumerBindingPreparation {
    compiler: HostBindingCompilerRegistration,
    runtime: HostConsumerBindingRegistration,
}
#[cfg(feature = "std")]
impl HostConsumerBindingPreparation {
    pub const fn compiler(&self) -> &HostBindingCompilerRegistration {
        &self.compiler
    }
}

#[cfg(feature = "std")]
impl Deref for HostConsumerBindingPreparation {
    type Target = HostConsumerBindingRegistration;
    fn deref(&self) -> &Self::Target {
        &self.runtime
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
            || compiler.consumer_metadata().is_none_or(|metadata| {
                metadata.configuration.configuration() != identity.configuration()
                    || Some(metadata.configuration) != runtime.consumer_configuration()
            })
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
