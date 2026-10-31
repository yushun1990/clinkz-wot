//! First joined v6 Producer: one whole qualified Property Read description,
//! closed native setup, local publication and shared finite execution.
//! The startup provider is frozen. This optional facade is not execution-only;
//! its runtime/dispatch/retirement owners live in `servient-finite`.
use crate::host_accounts::{HostCapacityReservation, HostResourceAccounts};
use alloc::{rc::Rc, sync::Arc, vec};
use clinkz_wot_core::{finite::*, producer::*, *};
use clinkz_wot_foundation::{MonotonicInstant, ResourceKind as K, ResourceLimits, WorkBudget};
use clinkz_wot_planning::finite_producer::{
    FiniteProducerCompiler, PRODUCER_DESCRIPTION_BYTES, qualify_property_read,
};
use clinkz_wot_servient_finite::*;
use core::{
    mem::size_of,
    task::{Context, Poll},
};

const REGISTRY: RawProducerRegistryLimits = RawProducerRegistryLimits {
    instances: 1,
    behaviors_per_instance: 1,
    target_name_bytes: 256,
};

// Fixed, prepaid lifetime and cleanup leases. No caller-held lock can block
// their disposal. Extra SDK/OS backing belongs to the concrete Binding policy.
pub(crate) struct Funding {
    _leases: [Option<HostCapacityReservation>; 8],
}
impl Funding {
    pub(crate) fn reserve(
        bank: &Arc<HostResourceAccounts>,
        amounts: &[(K, usize)],
    ) -> CoreResult<Self> {
        let mut result = Self {
            _leases: core::array::from_fn(|_| None),
        };
        for (i, &(kind, amount)) in amounts.iter().enumerate() {
            result._leases[i] = Some(bank.reserve(kind, amount as u64).ok_or_else(busy)?);
        }
        Ok(result)
    }
}

// Only setup owns qualification scratch. Behavior/result retention must never
// keep this phase's temporary or overlapping Engine charge alive after it ends.
pub(crate) struct QualificationFunding {
    _engine: HostCapacityReservation,
    _temporary: HostCapacityReservation,
}
impl QualificationFunding {
    pub(crate) fn reserve(bank: &Arc<HostResourceAccounts>, bytes: usize) -> CoreResult<Self> {
        let engine = bank
            .reserve(K::EngineLiveBytesGlobalMax, bytes as u64)
            .ok_or_else(busy)?;
        let temporary = bank
            .reserve(K::AdmissionTemporaryBytesGlobalMax, bytes as u64)
            .ok_or_else(busy)?;
        Ok(Self {
            _engine: engine,
            _temporary: temporary,
        })
    }
}

struct FundedBehavior<H> {
    handler: H,
    // Shared with the runtime and original accepted instance; replacement or
    // lost handles cannot refund capacity while original work remains alive.
    _funding: Rc<Funding>,
}
impl<H: RawFiniteHandler> RawFiniteHandler for FundedBehavior<H> {
    fn authorize(&self, input: &InteractionInput) -> CoreResult<()> {
        self.handler.authorize(input)
    }
    fn handle(
        &self,
        context: HandlerContext<'_>,
        input: &InteractionInput,
    ) -> CoreResult<InteractionOutput> {
        self.handler.handle(context, input)
    }
}

/// One frozen actual preparation registration and one logical slot. Native
/// instances are separately owned and can retain old results across replacement.
/// This narrow facade makes no multi-participant or general exposure claim.
pub struct ProducerServient<C: FiniteProducerCompiler> {
    logical: RawProducerServient,
    compiler: C,
    limits: ResourceLimits,
    accounts: Arc<HostResourceAccounts>,
    _funding: Funding,
}

/// A complete setup/execution/cleanup owner. Losing it revokes this incarnation
/// and performs the Binding's finite local abort, without claiming remote drain.
/// The publication and original Funding survive pending/failed retirement.
pub struct ProducerExposure<B: FiniteIngressBinding> {
    runtime: RawFiniteRuntime<B>,
    publication: ProducerPublication,
    description: Arc<str>,
    property: Arc<str>,
    qualification: Option<QualificationFunding>,
    _funding: Rc<Funding>,
}
impl<B: FiniteIngressBinding> ProducerExposure<B> {
    pub fn key(&self) -> ProducerInstanceKey {
        self.publication.key()
    }
    pub fn state(&self) -> RawFiniteState {
        self.runtime.state()
    }
    pub fn first_cause(&self) -> Option<&CoreError> {
        self.runtime.first_cause()
    }
    pub fn take_settlement(&mut self) -> Option<RawFiniteSettlement> {
        self.runtime.take_settlement()
    }
    pub fn begin_close(&mut self) {
        self.publication.revoke();
        self.runtime.begin_close();
        self.release_qualification();
    }
    pub fn cancel(&mut self) {
        self.fence(CoreError::Cancelled(ErrorContext::new(
            ErrorPhase::Handler,
            RetryClass::Never,
        )));
    }
    fn release_qualification(&mut self) {
        self.qualification = None;
    }
    fn fence(&mut self, cause: CoreError) {
        self.publication.revoke();
        self.runtime.fence(cause);
        self.release_qualification();
    }
    /// Native/cleanup progress after the Servient root has gone. No dispatch or
    /// publication can occur here; original settlement remains acknowledgeable.
    pub fn poll_cleanup(
        &mut self,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        now: MonotonicInstant,
    ) -> Poll<RawFiniteState> {
        self.begin_close();
        self.runtime.step(cx, budget, now)
    }
}
impl<B: FiniteIngressBinding> Drop for ProducerExposure<B> {
    fn drop(&mut self) {
        self.publication.revoke();
        self.release_qualification();
    }
}

impl<C: FiniteProducerCompiler> ProducerServient<C> {
    pub fn try_new(
        compiler: C,
        limits: ResourceLimits,
        accounts: Arc<HostResourceAccounts>,
    ) -> CoreResult<Self> {
        let bytes = RawProducerServient::publication_backing(REGISTRY)
            .ok_or_else(busy)?
            .checked_add(size_of::<Self>())
            .and_then(|n| n.checked_add(compiler.retained_bytes()))
            .ok_or_else(busy)?;
        let funding = Funding::reserve(&accounts, &[(K::EngineLiveBytesGlobalMax, bytes)])?;
        Ok(Self {
            logical: RawProducerServient::try_empty(REGISTRY)?,
            compiler,
            limits,
            accounts,
            _funding: funding,
        })
    }

    /// Pure bounded preparation. The exact retained JSON is the final TD in
    /// this scope: endpoints are explicit, no silent reduction/rewrite occurs.
    /// All failures here precede native effects. Handler indirect backing is a
    /// trusted application declaration, separate from bounded output capacity.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare<H: RawFiniteHandler + 'static>(
        &self,
        description: Arc<str>,
        property: Arc<str>,
        authority: ProducerInstanceAuthority,
        handler: H,
        handler_backing_bytes: usize,
        expected: Option<ProducerInstanceKey>,
        deadline: Deadline,
        budget: &mut WorkBudget,
    ) -> CoreResult<ProducerExposure<C::Binding>> {
        if description.len() > PRODUCER_DESCRIPTION_BYTES || property.len() > 256 {
            return Err(CoreError::UnsupportedOperation(context()));
        }
        let finite = RawFiniteLimits {
            candidates: 1,
            input_body_bytes: 0,
            output_body_bytes: 4096,
            output_media_capacity: 64,
            output_retained_bytes: 8192,
        };
        // Keep final TD decoding and semantic-arena capacity reserved through
        // setup. A full parent bank cannot strand readiness finalization. The
        // Host decoder allowance is conservative, not measured allocator RAM.
        let temporary = usize::try_from(
            self.limits
                .get(K::AdmissionTemporaryBytesPerOperationMax)
                .unwrap_or(0),
        )
        .ok()
        .and_then(|n| n.checked_add(PRODUCER_DESCRIPTION_BYTES * 512))
        .ok_or_else(busy)?;
        let retained = size_of::<ProducerExposure<C::Binding>>()
            .checked_add(size_of::<<C::Binding as FiniteIngressBinding>::Configuration>())
            .and_then(|n| n.checked_add(RawProducerServient::publication_backing(REGISTRY)?))
            .and_then(|n| n.checked_add(size_of::<FundedBehavior<H>>()))
            .and_then(|n| n.checked_add(size_of::<Funding>() + 4 * size_of::<usize>()))
            .and_then(|n| {
                n.checked_add(description.len() + property.len() + 4 * size_of::<usize>())
            })
            .and_then(|n| n.checked_add(handler_backing_bytes))
            .and_then(|n| n.checked_add(self.compiler.retained_bytes()))
            .and_then(|n| n.checked_add(finite.output_retained_bytes))
            .ok_or_else(busy)?;
        let funding = Rc::new(Funding::reserve(
            &self.accounts,
            &[
                (K::EngineLiveBytesGlobalMax, retained),
                (K::CleanupBytesMax, retained),
                (K::CleanupItemsMax, 1),
                (K::ThingsGlobalMax, 1),
                (K::PendingHandlerCallsGlobalMax, 1),
                (K::InFlightResponsesGlobalMax, 1),
                (K::BindingIngressItemsGlobalMax, 1),
            ],
        )?);
        let qualification = QualificationFunding::reserve(&self.accounts, temporary)?;
        let key = authority.key();
        let (binding, configuration) = qualify_property_read(
            &description,
            &property,
            key,
            &self.limits,
            budget,
            |input, budget| self.compiler.prepare(input, key, budget),
        )?;
        let runtime = RawFiniteRuntime::try_new(
            binding,
            configuration,
            &self.logical,
            finite,
            FiniteValidation::Required,
            deadline,
        )
        .map_err(|_| CoreError::Validation(context()))?;
        let instance = RawProducerInstance::new(
            authority,
            vec![RawFiniteBehavior::new(
                AffordanceTarget::Property(Arc::clone(&property)),
                Operation::ReadProperty,
                FundedBehavior {
                    handler,
                    _funding: Rc::clone(&funding),
                },
            )],
        )
        .with_description(Arc::clone(&description));
        let publication = self
            .logical
            .prepare_publication(instance, expected)
            .map_err(|_| CoreError::StaleHandle(context()))?;
        Ok(ProducerExposure {
            runtime,
            publication,
            description,
            property,
            qualification: Some(qualification),
            _funding: funding,
        })
    }

    pub fn description(&self, key: ProducerInstanceKey) -> Option<&str> {
        self.logical.description(key)
    }

    /// Revoke the one logical incarnation without native shutdown authority.
    /// Its accepted result/retirement owner stays with the exposure.
    pub fn revoke(&mut self, key: ProducerInstanceKey) -> bool {
        self.logical.revoke(key)
    }

    /// Release the closed logical record only after its last settlement has
    /// been acknowledged. Funding remains with any original retained owner.
    pub fn retire(
        &mut self,
        exposure: &mut ProducerExposure<C::Binding>,
        budget: &mut WorkBudget,
    ) -> CoreResult<()> {
        budget
            .consume(clinkz_wot_foundation::WorkClass::CleanupItems, 1)
            .map_err(|_| busy())?;
        self.logical
            .retire_publication(&mut exposure.publication, &exposure.runtime)
    }

    /// One progress boundary. Readiness is observable as InstalledClosed; the
    /// next funded step requalifies and publishes. Cancellation is checked first
    /// and wins even with zero work credit. Only the logical owner dispatches.
    pub fn step(
        &mut self,
        exposure: &mut ProducerExposure<C::Binding>,
        cx: &mut Context<'_>,
        budget: &mut WorkBudget,
        now: MonotonicInstant,
        cancel: bool,
    ) -> CoreResult<Poll<RawFiniteState>> {
        if cancel {
            exposure.cancel();
        }
        let was_closed = exposure.state() == RawFiniteState::InstalledClosed;
        let progress = self.logical.step(&mut exposure.runtime, cx, budget, now)?;
        if matches!(
            exposure.state(),
            RawFiniteState::Closing | RawFiniteState::Closed
        ) {
            exposure.publication.revoke();
            exposure.release_qualification();
        }
        if was_closed && exposure.state() == RawFiniteState::InstalledClosed {
            let binding = exposure.runtime.closed_binding().expect("closed loan");
            match qualify_property_read(
                &exposure.description,
                &exposure.property,
                exposure.key(),
                &self.limits,
                budget,
                |input, budget| {
                    self.compiler
                        .check_final(input, exposure.key(), binding, budget)
                },
            ) {
                Err(CoreError::Backpressure(_)) => return Ok(Poll::Pending),
                Err(error) => {
                    exposure.fence(error);
                    return Ok(Poll::Ready(exposure.state()));
                }
                Ok(()) => {}
            }
            // The decoding/semantic owners have been destroyed by qualification.
            // Release scratch outside the local publication commit; only the
            // lifetime/result/cleanup leases remain shared with behavior.
            exposure.release_qualification();
            if let Err(error) = self
                .logical
                .publish(&mut exposure.publication, &mut exposure.runtime)
            {
                exposure.fence(error);
            } else {
                exposure.publication.release_retired();
            }
            return Ok(Poll::Ready(exposure.state()));
        }
        Ok(progress)
    }
}

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
