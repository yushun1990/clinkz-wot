use clinkz_wot_core::*;
use clinkz_wot_planning::{binding_compiler::*, binding_registration::*};
use consumer_runtime_entry_probe::{Rejection, StaticInput, StaticOwner};
use std::{
    cell::Cell,
    task::{Context, Poll},
};

// Intentionally implements no PollServerBinding or RouteServerBinding.
struct Client<'a, const N: usize> {
    compatibility: BindingArtifactCompatibility,
    drops: &'a Cell<u32>,
    extra: BindingLifetimeFootprint,
    transient: u64,
}
impl<const N: usize> Drop for Client<'_, N> {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
impl<const N: usize> clinkz_wot_planning::PrepareClientBinding for Client<'_, N> {
    type Compiler = ResolvedTargetCompiler;
}
impl<const N: usize> PollClientBinding for Client<'_, N> {
    type Artifact =
        <ResolvedTargetCompiler as clinkz_wot_planning::BindingCompilerExtension>::Artifact;
    type RequestState = [u8; N];
    fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }
    fn request_state_layout(&self) -> BindingStateLayout {
        BindingStateLayout::of::<Self::RequestState>(self.extra)
            .with_transient(BindingTransientFootprint::new(self.transient))
    }
    fn start_request(
        &mut self,
        _: OutboundRequest,
        _: &BindingArtifactEnvelope<ResolvedTargetArtifact>,
        _: &mut ClientRequestSlot<Self::RequestState>,
        _: &mut WorkBudget,
    ) -> Result<StartStatus<CoreResult<InteractionOutput>>, BindingInputRejection<OutboundRequest>>
    {
        panic!("construction performed protocol work")
    }
    fn poll_request(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ClientRequestSlot<Self::RequestState>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<InteractionOutput>> {
        panic!("construction polled")
    }
    fn start_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: CleanupPhaseContext,
        _: &mut ClientRequestSlot<Self::RequestState>,
        _: &mut WorkBudget,
    ) -> CoreResult<StartStatus<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        panic!("construction cancelled")
    }
    fn poll_cancel_request(
        &mut self,
        _: &mut Context<'_>,
        _: &mut ClientRequestSlot<Self::RequestState>,
        _: &mut WorkBudget,
    ) -> Poll<CoreResult<BindingCallSettlement<CoreResult<InteractionOutput>>>> {
        panic!("construction polled cancellation")
    }
    fn acknowledge_request(
        &mut self,
        _: &mut ClientRequestSlot<Self::RequestState>,
    ) -> CoreResult<()> {
        panic!("construction acknowledged")
    }
}
use clinkz_wot_foundation::WorkBudget;

fn identity(compiler: &ResolvedTargetCompiler) -> BindingRegistrationIdentity {
    BindingRegistrationIdentity::new(
        BindingId::new(211),
        BindingGeneration::INITIAL.checked_next().unwrap(),
        compiler.configuration(),
        compiler.compatibility(),
        23,
    )
}
fn resources(bytes: u64) -> BindingResourceDeclarations {
    // No fictitious server/route/readiness/response state or ingress owner.
    BindingResourceDeclarations::new(
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(8, bytes),
    )
    .with_state_footprints(
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(0, 0),
        BindingLifetimeFootprint::new(0, 0),
    )
}
fn input<const N: usize>(drops: &Cell<u32>, bytes: u64) -> StaticInput<Client<'_, N>> {
    let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
    StaticInput {
        identity: identity(&compiler),
        execution: BindingExecutionSupport::application_static(),
        client: Client {
            compatibility: compiler.compatibility(),
            drops,
            extra: BindingLifetimeFootprint::new(0, 0),
            transient: 0,
        },
        compiler,
        resources: resources(bytes),
        status: BindingStatusPolicy::new(0, 0),
    }
}
fn reject<const N: usize>(input: StaticInput<Client<'_, N>>, expected: Rejection) {
    let original = input.identity;
    let original_config = input.compiler.configuration();
    let (returned, reason) = match StaticOwner::try_new(input) {
        Err(e) => e,
        Ok(_) => panic!("accepted invalid model"),
    };
    assert_eq!(reason, expected);
    assert_eq!(returned.identity, original);
    assert_eq!(returned.compiler.configuration(), original_config);
    assert_eq!(returned.client.drops.get(), 0);
    drop(returned);
}

#[test]
fn real_client_only_parts_construct_without_server_or_protocol_work() {
    let drops = Cell::new(0);
    let input = input::<17>(&drops, 4096);
    let expected = input.identity;
    let owner = StaticOwner::try_new(input).unwrap_or_else(|_| panic!("valid model"));
    assert_eq!(owner.identity(), expected);
    assert_eq!(drops.get(), 0);
    let returned = owner.into_input();
    assert_eq!(
        returned.resources.route_state(),
        BindingLifetimeFootprint::new(0, 0)
    );
    drop(returned);
    assert_eq!(drops.get(), 1);
}

#[test]
fn rejects_cell_compiler_client_and_actual_config_mismatches_without_losing_owner() {
    for kind in 0..4 {
        let drops = Cell::new(0);
        let mut i = input::<17>(&drops, 4096);
        let expected = match kind {
            0 => {
                i.execution = BindingExecutionSupport::host_erased();
                Rejection::Cell
            }
            1 => {
                i.client.compatibility = BindingArtifactCompatibility::new([99; 16]);
                Rejection::Compatibility
            }
            2 => {
                i.identity = BindingRegistrationIdentity::new(
                    i.identity.binding_id(),
                    i.identity.binding_generation(),
                    i.identity.configuration(),
                    BindingArtifactCompatibility::new([99; 16]),
                    23,
                );
                Rejection::Compatibility
            }
            _ => {
                i.identity = BindingRegistrationIdentity::new(
                    i.identity.binding_id(),
                    i.identity.binding_generation(),
                    BindingConfigurationDigest::new([99; 32]),
                    i.identity.artifact_compatibility(),
                    23,
                );
                Rejection::Configuration
            }
        };
        reject(i, expected);
        assert_eq!(drops.get(), 1);
    }
}

#[test]
fn complete_core_slot_boundary_and_transient_are_counted_separately() {
    let bytes = std::mem::size_of::<StaticConsumerPropertyReadSlot<[u8; 17]>>() as u64;
    for ceiling in [bytes - 1, bytes, bytes + 1] {
        let drops = Cell::new(0);
        let i = input::<17>(&drops, ceiling);
        if ceiling < bytes {
            reject(i, Rejection::Resources);
        } else {
            drop(StaticOwner::try_new(i).unwrap_or_else(|_| panic!("complete slot fits")));
        }
        assert_eq!(drops.get(), 1);
    }
    let drops = Cell::new(0);
    let mut i = input::<17>(&drops, 4096);
    i.client.transient = 1;
    reject(i, Rejection::Resources);
    assert_eq!(drops.get(), 1);
}

#[test]
fn additional_live_state_overflow_and_invalid_declarations_reject() {
    for case in 0..3 {
        let drops = Cell::new(0);
        let mut i = input::<17>(&drops, 4096);
        match case {
            0 => i.client.extra = BindingLifetimeFootprint::new(1, 4096),
            1 => i.client.extra = BindingLifetimeFootprint::new(u32::MAX, u64::MAX),
            _ => {
                i.resources = BindingResourceDeclarations::new(
                    BindingLifetimeFootprint::new(9, 4097),
                    BindingLifetimeFootprint::new(8, 4096),
                )
            }
        }
        reject(i, Rejection::Resources);
        assert_eq!(drops.get(), 1);
    }
}

#[cfg(feature = "std")]
mod host {
    use super::*;
    use clinkz_wot_core::binding::ClientBinding;
    use consumer_runtime_entry_probe::host::{Input, Owner};
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };
    struct HostClient {
        compatibility: BindingArtifactCompatibility,
        drops: Arc<AtomicU32>,
    }
    impl Drop for HostClient {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }
    impl ClientBinding for HostClient {
        fn artifact_compatibility(&self) -> BindingArtifactCompatibility {
            self.compatibility
        }
        fn invoke(
            &self,
            _: OutboundRequest,
            _: &BindingArtifactEnvelope<HostBindingArtifact>,
        ) -> Result<
            HostBindingCallBox<CoreResult<InteractionOutput>>,
            BindingInputRejection<OutboundRequest>,
        > {
            panic!("construction invoked client")
        }
    }
    #[test]
    fn host_retains_exact_client_and_supported_compiler_without_a_server() {
        for case in 0..4 {
            let compiler = ResolvedTargetCompiler::try_new(64).unwrap();
            let id = identity(&compiler);
            let drops = Arc::new(AtomicU32::new(0));
            let client: Box<dyn ClientBinding> = Box::new(HostClient {
                compatibility: if case == 2 {
                    BindingArtifactCompatibility::new([99; 16])
                } else {
                    compiler.compatibility()
                },
                drops: drops.clone(),
            });
            let address = &*client as *const dyn ClientBinding as *const ();
            let input = Input {
                identity: id,
                execution: if case == 1 {
                    BindingExecutionSupport::application_static()
                } else {
                    BindingExecutionSupport::host_erased()
                },
                compiler: HostBindingCompilerRegistration::try_new_consumer(compiler)
                    .unwrap_or_else(|_| panic!("supported adapter")),
                client,
                resources: if case == 3 {
                    BindingResourceDeclarations::new(
                        BindingLifetimeFootprint::new(9, 9),
                        BindingLifetimeFootprint::new(1, 1),
                    )
                } else {
                    resources(4096)
                },
                status: BindingStatusPolicy::new(0, 0),
            };
            let returned = match Owner::try_new(input) {
                Ok(owner) => {
                    assert_eq!(case, 0);
                    assert_eq!(owner.identity(), id);
                    owner.into_input()
                }
                Err((input, reason)) => {
                    assert_eq!(
                        reason,
                        [
                            Rejection::Cell,
                            Rejection::Cell,
                            Rejection::Compatibility,
                            Rejection::Resources
                        ][case]
                    );
                    input
                }
            };
            assert_eq!(returned.identity, id);
            assert_eq!(
                &*returned.client as *const dyn ClientBinding as *const (),
                address
            );
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            drop(returned);
            assert_eq!(drops.load(Ordering::SeqCst), 1);
        }
    }
}
