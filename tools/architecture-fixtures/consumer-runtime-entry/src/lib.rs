//! Entry discriminators and downstream production registration evidence.
//! Models below issue no authority; `production` uses actual Core owners.
#![no_std]
extern crate alloc;

pub mod production;

use clinkz_wot_core::*;
use clinkz_wot_planning::{binding_compiler::*, binding_registration::*};
use core::mem::size_of;

/// The existing Producer/dual-role static registration requires a server.
/// A client alone uses the separate Consumer-only complete owner.
/// ```compile_fail
/// use clinkz_wot_core::{PollClientBinding};
/// use clinkz_wot_planning::{ StaticBindingPreparation};
/// fn client_only<C: PollClientBinding>() {
///     let _ = core::mem::size_of::<StaticBindingPreparation<C>>();
/// }
/// ```
/// The current draft is local even if the concrete artifact is thread safe.
/// ```compile_fail
/// use clinkz_wot_planning::ResolvedTargetArtifact;
/// use clinkz_wot_planning::ConsumerPropertyReadDraft;
/// fn shared<T: Send + Sync>() {}
/// shared::<ConsumerPropertyReadDraft<ResolvedTargetArtifact>>();
/// ```
/// ```compile_fail
/// use clinkz_wot_planning::ResolvedTargetArtifact;
/// use clinkz_wot_planning::ConsumerPropertyReadDraft;
/// fn shared<T: Sync>() {}
/// shared::<ConsumerPropertyReadDraft<ResolvedTargetArtifact>>();
/// ```
/// Its cleanup destination and a detached cleanup lease are also local.
/// ```compile_fail
/// use clinkz_wot_planning::ResolvedTargetArtifact;
/// use clinkz_wot_planning::ConsumerPropertyReadCleanupSlot;
/// fn movable<T: Send>() {}
/// movable::<ConsumerPropertyReadCleanupSlot<ResolvedTargetArtifact>>();
/// ```
/// ```compile_fail
/// use clinkz_wot_planning::ResolvedTargetArtifact;
/// use clinkz_wot_planning::ConsumerPropertyReadRollback;
/// fn movable<T: Send>() {}
/// movable::<ConsumerPropertyReadRollback<ResolvedTargetArtifact>>();
/// ```
pub struct CurrentBoundary;

/// Wrapping the current graph does not change its ownership capabilities.
/// ```compile_fail
/// use std::sync::{Arc, Mutex};
/// use clinkz_wot_core::HostBindingArtifact;
/// use clinkz_wot_planning::ConsumerPropertyReadDraft;
/// fn movable<T: Send>() {}
/// movable::<Arc<Mutex<ConsumerPropertyReadDraft<HostBindingArtifact>>>>();
/// ```
/// ```compile_fail
/// use std::sync::Arc;
/// use clinkz_wot_core::HostBindingArtifact;
/// use clinkz_wot_planning::ConsumerPropertyReadDraft;
/// fn shared<T: Send + Sync>() {}
/// shared::<Arc<ConsumerPropertyReadDraft<HostBindingArtifact>>>();
/// ```
#[cfg(feature = "std")]
pub struct HostRootWrappers;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    Cell,
    Compatibility,
    Configuration,
    Resources,
}

fn resources_fit(r: BindingResourceDeclarations) -> bool {
    r.declared().fits_within(r.admitted())
        && r.route_state().fits_within(r.admitted())
        && r.readiness_state().fits_within(r.admitted())
        && r.response_state().fits_within(r.admitted())
}

/// A closed-compiler assembly model, rather than a replacement for the open
/// authoring SPI. No server, ingress policy, TD, allocator or callback issuer.
pub struct StaticInput<C: PrepareClientBinding<Compiler = ResolvedTargetCompiler>> {
    pub identity: BindingRegistrationIdentity,
    pub execution: BindingExecutionSupport,
    pub compiler: ResolvedTargetCompiler,
    pub client: C,
    pub resources: BindingResourceDeclarations,
    pub status: BindingStatusPolicy,
}

pub struct StaticOwner<C: PrepareClientBinding<Compiler = ResolvedTargetCompiler>>(StaticInput<C>);

impl<C: PrepareClientBinding<Compiler = ResolvedTargetCompiler>> StaticOwner<C> {
    pub fn try_new(input: StaticInput<C>) -> Result<Self, (StaticInput<C>, Rejection)> {
        let error = if !input.execution.supports_application_static() {
            Some(Rejection::Cell)
        } else if input.identity.artifact_compatibility() != input.compiler.compatibility()
            || input.identity.artifact_compatibility() != input.client.artifact_compatibility()
        {
            Some(Rejection::Compatibility)
        } else if input.identity.configuration() != input.compiler.configuration() {
            Some(Rejection::Configuration)
        } else {
            let raw = input.client.request_state_layout();
            let core = BindingLifetimeFootprint::new(
                1,
                size_of::<StaticConsumerPropertyReadSlot<C::RequestState>>() as u64,
            );
            let extra = raw.lifetime_footprint();
            let slot_fits = extra
                .retained_items()
                .checked_add(core.retained_items())
                .zip(extra.retained_bytes().checked_add(core.retained_bytes()))
                .is_some_and(|(items, bytes)| {
                    BindingLifetimeFootprint::new(items, bytes)
                        .fits_within(input.resources.admitted())
                });
            if !resources_fit(input.resources)
                || !slot_fits
                || raw.transient_footprint().peak_bytes() > input.resources.transient().peak_bytes()
            {
                Some(Rejection::Resources)
            } else {
                None
            }
        };
        match error {
            Some(error) => Err((input, error)),
            None => Ok(Self(input)),
        }
    }

    pub fn identity(&self) -> BindingRegistrationIdentity {
        self.0.identity
    }
    pub fn into_input(self) -> StaticInput<C> {
        self.0
    }
}

#[cfg(feature = "std")]
pub mod host {
    use super::*;
    use alloc::boxed::Box;
    use clinkz_wot_core::binding::ClientBinding;

    /// Uses the real fallible supported Host compiler adapter. Client boxing is
    /// caller startup provisioning; this model performs no allocation itself.
    pub struct Input {
        pub identity: BindingRegistrationIdentity,
        pub execution: BindingExecutionSupport,
        pub compiler: HostBindingCompilerRegistration,
        pub client: Box<dyn ClientBinding>,
        pub resources: BindingResourceDeclarations,
        pub status: BindingStatusPolicy,
    }
    pub struct Owner(Input);
    impl Owner {
        pub fn try_new(input: Input) -> Result<Self, (Input, Rejection)> {
            // This deliberately does not mint or emulate ConsumerCompilerSupport.
            let error = if !input.execution.supports_host_erased() {
                Some(Rejection::Cell)
            } else if input.identity.artifact_compatibility() != input.compiler.compatibility()
                || input.identity.artifact_compatibility() != input.client.artifact_compatibility()
            {
                Some(Rejection::Compatibility)
            } else if !resources_fit(input.resources) {
                Some(Rejection::Resources)
            } else {
                None
            };
            match error {
                Some(error) => Err((input, error)),
                None => Ok(Self(input)),
            }
        }
        pub fn identity(&self) -> BindingRegistrationIdentity {
            self.0.identity
        }
        pub fn into_input(self) -> Input {
            self.0
        }
    }
}
