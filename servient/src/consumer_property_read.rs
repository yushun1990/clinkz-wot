//! Manual, Consumer-only Property Read runtime.
//!
//! `consume` lends a local build adapter to a caller's progress scope. The
//! scope may suspend on caller work or timers, but cannot export an input loan
//! or a local Planning root. A completed source-free owner can subsequently
//! wait for final installation credit in the runtime. Leaving the scope early
//! hands its complete rollback to the runtime. No worker or executor is used.
//!
//! Binding author declarations are a trusted finite-callback contract, checked
//! against the actual moved registration. They are not certification of an
//! arbitrary Rust callback. Request maps are moved, never cloned. Rejection
//! before construction returns the original options; rejection by Core returns
//! the original request alongside the remaining selection controls.

use crate::CompiledPlanSetState;
use clinkz_wot_core::*;
use clinkz_wot_foundation::{
    AdmissionLedger, Generation, MonotonicInstant, ResourceAccount, ResourceKind as K,
    ResourceLimits, ResourceProfileId, RuntimeClock, SlotIndex, WorkBudget, WorkClass as W,
};
use clinkz_wot_planning::{
    ConsumerPropertyReadBuild as PlanningBuild, ConsumerPropertyReadBuildConfig,
    ConsumerPropertyReadBuildStep as PlanningStep, ConsumerPropertyReadCleanupSlot as PlanRoot,
    ConsumerPropertyReadDraft as Draft, ConsumerPropertyReadPreflight as Preflight,
    ConsumerPropertyReadRegistration, ConsumerPropertyReadRollback as Rollback,
    ConsumerPropertyReadRollbackStep as ReclaimStep, build_consumer_property_read_aggregate,
    select_consumer_property_read_aggregate,
};
use clinkz_wot_td::{
    ValidatedThingAdmissionConfig, ValidatedThingCause, ValidatedThingCursor,
    ValidatedThingProgress, thing::Thing,
};
use core::{
    cell::{Cell, RefCell},
    mem::size_of,
    task::{Context, Poll},
};

#[cfg(feature = "std")]
mod host;
mod identity;
#[cfg(feature = "std")]
mod shared;
#[cfg(feature = "std")]
pub use host::{
    ConsumerPropertyReadCall, ConsumerPropertyReadHandle, ConsumerPropertyReadRuntime,
    HostConsumerPropertyReadBuild,
};

const N: usize = 20;
const ENGINE: usize = 0;
const TEMP: usize = 1;
const PEAK: usize = 2;
const OUTPUT: usize = 3;
const CURSOR: usize = 4;
const ARTIFACTS: usize = 5;
const ARTIFACT_BYTES: usize = 6;
const SETS: usize = 7;
const PINS: usize = 8;
const CALLS: usize = 9;
const CLEANUP_ITEMS: usize = 10;
const CLEANUP_BYTES: usize = 11;
const BINDINGS: usize = 12;
const THINGS: usize = 13;
const HOST_BYTES: usize = 14;
const SLOT_BYTES: usize = 15;
const POLL_TEMP: usize = 16;
const TRANSFER_SLOTS: usize = 17;
const TRANSFER_BYTES: usize = 18;
const CANCEL_BYTES: usize = 19;
const KINDS: [K; N] = [
    K::EngineLiveBytesGlobalMax,
    K::AdmissionTemporaryBytesGlobalMax,
    K::AdmissionPeakLiveBytesGlobalMax,
    K::CompiledRuntimeBytesGlobalMax,
    K::BindingCompilerCursorBytesGlobalMax,
    K::BindingArtifactsGlobalMax,
    K::BindingArtifactBytesGlobalMax,
    K::PlanSetsGlobalMax,
    K::PlanPinsGlobalMax,
    K::PendingClientCallsGlobalMax,
    K::CleanupItemsMax,
    K::CleanupBytesMax,
    K::BindingsGlobalMax,
    K::ThingsGlobalMax,
    K::HostBindingCallBytesGlobalMax,
    K::BindingSlotStateBytesGlobalMax,
    K::BindingPollTemporaryBytesGlobalMax,
    K::CleanupTransferSlotsGlobalMax,
    K::CleanupTransferBytesGlobalMax,
    K::BindingCancelBufferBytesGlobalMax,
];
// Phase, nonce, policy owner, complete registration, three generations, backing,
// pin gate, cancellation, destination, and install. Bridge return is additional.
const FINAL_ITEMS: u64 = 12;

fn context() -> ErrorContext {
    ErrorContext::new(ErrorPhase::Admission, RetryClass::Never)
        .with_operation(clinkz_wot_td::data_type::Operation::ReadProperty)
}
fn unsupported() -> CoreError {
    CoreError::UnsupportedOperation(context())
}
fn invalid() -> CoreError {
    CoreError::Validation(context())
}
fn busy() -> CoreError {
    CoreError::Backpressure(context())
}
fn invariant() -> CoreError {
    CoreError::InternalInvariant(context())
}
fn stale() -> CoreError {
    CoreError::StaleHandle(context())
}
fn add(a: u64, b: u64) -> CoreResult<u64> {
    a.checked_add(b).ok_or_else(unsupported)
}
fn mul(a: u64, b: u64) -> CoreResult<u64> {
    a.checked_mul(b).ok_or_else(unsupported)
}
fn check(limits: &ResourceLimits, kind: K, n: u64) -> CoreResult<()> {
    let max = limits.get(kind).ok_or_else(unsupported)?;
    if n > max {
        Err(CoreError::LimitExceeded {
            resource: kind,
            limit: max,
            requested: Some(n),
            observed: None,
            context: context(),
        })
    } else {
        Ok(())
    }
}
fn stamp_resource(
    mut error: CoreError,
    identity: BindingConfigurationDigest,
    domain: ResourceFailureDomain,
    local: Option<(SlotIndex, Generation)>,
    parent: Option<(SlotIndex, Generation)>,
) -> CoreError {
    let context = match &mut error {
        CoreError::InvalidDocument(c)
        | CoreError::Validation(c)
        | CoreError::NotFound(c)
        | CoreError::UnsupportedOperation(c)
        | CoreError::Application(c)
        | CoreError::Binding(c)
        | CoreError::Payload(c)
        | CoreError::Backpressure(c)
        | CoreError::Cancelled(c)
        | CoreError::TimedOut(c)
        | CoreError::StaleHandle(c)
        | CoreError::Lifecycle(c)
        | CoreError::Cleanup(c)
        | CoreError::InternalInvariant(c)
        | CoreError::LimitExceeded { context: c, .. }
        | CoreError::Selection { context: c, .. }
        | CoreError::Security { context: c, .. } => c,
        _ => return error,
    };
    if context.resource_failure().is_none() {
        *context = context.with_resource_failure(identity, domain, local, parent);
    }
    error
}
const fn compilation_cell() -> &'static str {
    if cfg!(feature = "std") {
        "std"
    } else if cfg!(feature = "async") {
        "async-no-std"
    } else {
        "no-default"
    }
}
fn pay(b: &mut WorkBudget, costs: &[(W, u64)]) -> bool {
    if costs.iter().any(|&(w, n)| b.remaining(w) < n) {
        return false;
    }
    for &(w, n) in costs {
        b.consume(w, n).expect("whole debit checked");
    }
    true
}
// One allowance covers every cleanup branch in a public progress operation,
// including nested step during shutdown. Restore only the caller's withheld
// excess; consumed credit is never refunded, including during unwinding.
struct CleanupStep<'a> {
    work: &'a mut WorkBudget,
    withheld: u64,
}
impl<'a> CleanupStep<'a> {
    fn new(work: &'a mut WorkBudget, limit: u64) -> Self {
        let available = work.remaining(W::CleanupItems);
        let allowed = available.min(limit);
        work.set_remaining(W::CleanupItems, allowed);
        Self {
            work,
            withheld: available - allowed,
        }
    }
}
impl Drop for CleanupStep<'_> {
    fn drop(&mut self) {
        self.work.set_remaining(
            W::CleanupItems,
            self.work
                .remaining(W::CleanupItems)
                .checked_add(self.withheld)
                .expect("only unused caller credit is restored"),
        );
    }
}
fn reject<T>(input: T, error: CoreError) -> BindingInputRejection<T> {
    BindingInputRejection::new(input, BindingOperationalError::new(error))
}
fn td_error(cause: ValidatedThingCause) -> CoreError {
    match cause {
        ValidatedThingCause::Invalid(_) => {
            CoreError::Validation(ErrorContext::new(ErrorPhase::Validate, RetryClass::Never))
        }
        ValidatedThingCause::Limit(l) => CoreError::LimitExceeded {
            resource: l.kind(),
            limit: l.configured(),
            requested: Some(l.observed()),
            observed: None,
            context: context(),
        },
        ValidatedThingCause::Cancelled { .. } => CoreError::Cancelled(context()),
        ValidatedThingCause::Failed(f)
            if f.kind() == clinkz_wot_td::ValidatedThingFailureKind::AllocationFailed =>
        {
            busy()
        }
        ValidatedThingCause::Failed(_) => invalid(),
    }
}

/// Immutable checked policy and binding-author declarations.
///
/// `input` is [URI variable count, UTF-8 bytes per string, actual String
/// capacity per string, property-name bytes]. `result` is [body bytes,
/// content-type/coding capacity]. `work` is finite nonzero work for
/// [constructor, footprint, deadline, poll, cancel, local destructor, static
/// acknowledgement]. `timeouts` are finite [call, cancel-drain] logical ticks.
/// `scratch_bytes` backs both TD and Planning, each projected to half of it.
/// `registration` includes the author's actual client/compiler/shared backing;
/// `call` includes the complete raw state and all request/result backing.
/// Core's envelope and Runtime records are accounted in addition to these.
pub struct ConsumerPropertyReadPolicy {
    contract: BindingResourceContract,
    origin: ResourceProfileId,
    identity: BindingConfigurationDigest,
    limits: ResourceLimits,
    td: ValidatedThingAdmissionConfig,
    planning: ConsumerPropertyReadBuildConfig,
    registration: BindingLifetimeFootprint,
    call: BindingLifetimeFootprint,
    input: [u64; 4],
    result: [u64; 2],
    work: [u64; 7],
    timeouts: [u64; 2],
    scratch: u64,
}
impl ConsumerPropertyReadPolicy {
    pub const PARENT_KINDS: [K; N] = KINDS;
    pub const FINAL_INSTALL_WORK: u64 = FINAL_ITEMS;

    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        limits: ResourceLimits,
        registration: BindingLifetimeFootprint,
        call: BindingLifetimeFootprint,
        input: [u64; 4],
        result: [u64; 2],
        work: [u64; 7],
        timeouts: [u64; 2],
        scratch_bytes: u64,
    ) -> CoreResult<Self> {
        if work.contains(&0)
            || timeouts.contains(&0)
            || input[1..].contains(&0)
            || input[2] < input[1]
            || result.contains(&0)
            || scratch_bytes == 0
            || call.retained_items() == 0
            || call.retained_bytes() == 0
        {
            return Err(unsupported());
        }
        check(&limits, K::UriVariablesPerFormMax, input[0])?;
        check(&limits, K::StringBytesMax, input[1])?;
        check(&limits, K::StringBytesMax, input[3])?;
        check(&limits, K::PayloadBytesMax, result[0])?;
        check(
            &limits,
            K::AdmissionTemporaryBytesPerOperationMax,
            scratch_bytes,
        )?;
        for kind in [
            K::PlanSetsPerThingMax,
            K::PlanPinsPerPlanSetMax,
            K::CleanupWorkItemsPerStepMax,
            K::PlanReclaimBytesPerStepMax,
            K::PlanCompileWorkUnitsPerStepMax,
        ] {
            check(&limits, kind, 1)?;
        }
        for kind in KINDS {
            if !matches!(
                kind,
                K::PendingClientCallsGlobalMax
                    | K::HostBindingCallBytesGlobalMax
                    | K::BindingSlotStateBytesGlobalMax
                    | K::BindingPollTemporaryBytesGlobalMax
                    | K::CleanupTransferSlotsGlobalMax
                    | K::CleanupTransferBytesGlobalMax
                    | K::BindingCancelBufferBytesGlobalMax
            ) {
                check(&limits, kind, 1)?;
            }
        }
        let mut projection = limits.clone();
        assert!(projection.set(
            K::AdmissionTemporaryBytesPerOperationMax,
            Some(scratch_bytes / 2)
        ));
        let td = ValidatedThingAdmissionConfig::try_from_limits(&projection)
            .map_err(|_| unsupported())?;
        let planning = ConsumerPropertyReadBuildConfig::try_from_limits(&projection)?;
        let mut policy = Self {
            contract: BindingResourceContract::StrictV1,
            origin: ResourceProfileId::APPLICATION_DEFINED,
            identity: BindingConfigurationDigest::new([0; 32]),
            limits,
            td,
            planning,
            registration,
            call,
            input,
            result,
            work,
            timeouts,
            scratch: scratch_bytes,
        };
        policy.request_bound()?;
        add(work[0], work[1])?;
        add(work[5], work[6])?;
        call.retained_items()
            .checked_add(1)
            .ok_or_else(unsupported)?;
        policy.terminal_fixed_work()?;
        add(policy.call_bytes()?, policy.registration.retained_bytes())?;
        mul(
            work[5],
            add(
                u64::from(call.retained_items()),
                u64::from(registration.retained_items()),
            )?,
        )?;
        policy.identity = identity::policy_key(&policy);
        Ok(policy)
    }

    /// Checks an explicit Consumer ownership interpretation of the full policy.
    /// The profile supplies values only; it never implicitly selects a contract.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new_owned(
        contract: BindingResourceContract,
        profile_origin: ResourceProfileId,
        limits: ResourceLimits,
        registration: BindingLifetimeFootprint,
        call: BindingLifetimeFootprint,
        input: [u64; 4],
        result: [u64; 2],
        work: [u64; 7],
        timeouts: [u64; 2],
        scratch_bytes: u64,
    ) -> CoreResult<Self> {
        if contract == BindingResourceContract::StrictV1
            || (contract == BindingResourceContract::ConsumerOwnedHostV2 && !cfg!(feature = "std"))
        {
            return Err(unsupported());
        }
        // The closed configuration performs the same full Payload/String Layout
        // representability check. It is fixed metadata work, with no callbacks.
        ConsumerBindingConfiguration::try_new(
            contract,
            1,
            BindingConfigurationDigest::new([0; 32]),
            result[0],
            result[1],
        )?;
        let mut policy = Self::try_new(
            limits,
            registration,
            call,
            input,
            result,
            work,
            timeouts,
            scratch_bytes,
        )?;
        policy.contract = contract;
        policy.origin = profile_origin;
        policy.identity = identity::policy_key(&policy);
        policy.call_bytes()?;
        Ok(policy)
    }
    /// Returns the explicitly checked resource/disposal contract.
    pub const fn resource_contract(&self) -> BindingResourceContract {
        self.contract
    }
    /// Returns the complete key of every independent policy operand.
    pub const fn resource_identity(&self) -> BindingConfigurationDigest {
        self.identity
    }
    /// Returns the supplied origin of the numeric values.
    pub const fn profile_origin(&self) -> ResourceProfileId {
        self.origin
    }
    /// Borrows the full immutable numeric table.
    pub const fn resource_limits(&self) -> &ResourceLimits {
        &self.limits
    }
    /// Creates real initially empty parent accounts. Share these *same accounts*
    /// between roots; making another array creates another capacity authority.
    /// Keep the bank intact while roots/reports hold charges. During those
    /// loans the Runtime owns reservation and release; callers can read totals.
    pub fn parent_accounts(&self) -> [ResourceAccount; N] {
        core::array::from_fn(|i| {
            ResourceAccount::new(
                SlotIndex::new(i as u32),
                Generation::INITIAL,
                KINDS[i],
                self.limits.get(KINDS[i]).unwrap_or(0),
            )
        })
    }
    fn qualify<R: ConsumerPropertyReadRegistration>(
        &self,
        r: &R,
        resources: BindingResourceDeclarations,
        host: bool,
    ) -> CoreResult<()> {
        if resources.resource_contract() != self.contract {
            return Err(self.resource_error(unsupported(), ResourceFailureDomain::EngineCapacity));
        }
        if self.contract != BindingResourceContract::StrictV1 {
            if r.support()
                .consumer_configuration()
                .is_none_or(|c| c.compilation_cell() != compilation_cell())
            {
                return Err(
                    self.resource_error(unsupported(), ResourceFailureDomain::EngineCapacity)
                );
            }
            let Some((body, media)) = resources.consumer_result_limits() else {
                return Err(unsupported());
            };
            if body > self.result[0] || media > self.result[1] {
                return Err(
                    self.resource_error(unsupported(), ResourceFailureDomain::EngineCapacity)
                );
            }
        }
        if !resources.declared().fits_within(self.registration)
            || !self.call.fits_within(resources.admitted())
            || (host && self.limits.get(K::PendingClientCallsGlobalMax).unwrap_or(0) == 0)
        {
            return Err(unsupported());
        }
        if host {
            self.check(K::PendingClientCallsPerBindingMax, 1)?;
            self.check(K::PendingClientCallsPerThingMax, 1)?;
            self.check_binding_storage(true, self.call_bytes()?)?;
        }
        for kind in [
            K::BindingPollTemporaryBytesPerCallMax,
            K::BindingPollTemporaryBytesGlobalMax,
        ] {
            self.check(kind, resources.transient().peak_bytes())?;
        }
        for kind in [
            K::BindingCancelBufferBytesPerCallMax,
            K::BindingCancelBufferBytesGlobalMax,
        ] {
            self.check(kind, self.call_bytes()?)?;
        }
        let release = if host {
            self.work[5]
        } else {
            add(self.work[5], self.work[6])?
        };
        // Complete registration destruction and Core's associated-state
        // acknowledgement/destruction are indivisible existing operations.
        self.check(
            K::CleanupWorkItemsPerStepMax,
            release.max(self.registration_cleanup_work()?),
        )?;
        self.check(
            K::BindingArtifactBytesPerItemMax,
            r.support().artifact_layout().size() as u64,
        )?;
        self.check(
            K::BindingCompilerCursorBytesPerItemMax,
            r.support().cursor_layout().size() as u64,
        )?;
        // Planning qualifies the captured generic compiler work/layouts against
        // this policy's build config. Source limits belong to that admission;
        // an external compiler need not have a literal target-buffer capacity.
        add(self.call_temporary(resources)?, self.call_bytes()?)?;
        self.check(
            K::LargestContiguousAllocationBytesMax,
            self.call.retained_bytes().max(self.core_backing()),
        )?;
        Ok(())
    }
    fn check_binding_storage(&self, host: bool, bytes: u64) -> CoreResult<()> {
        let kinds: &[K] = if host {
            &[
                K::HostBindingCallBytesPerItemMax,
                K::HostBindingCallBytesPerBindingMax,
                K::HostBindingCallBytesPerThingMax,
                K::HostBindingCallBytesGlobalMax,
            ]
        } else {
            &[
                K::BindingSlotStateBytesPerItemMax,
                K::BindingSlotStateBytesPerThingMax,
                K::BindingSlotStateBytesGlobalMax,
            ]
        };
        for &kind in kinds {
            self.check(kind, bytes)?;
        }
        Ok(())
    }
    fn registration_cleanup_work(&self) -> CoreResult<u64> {
        mul(
            self.work[5],
            u64::from(self.registration.retained_items().max(1)),
        )
    }
    fn request_bound(&self) -> CoreResult<u64> {
        // A conservative native BTreeMap node envelope, separately measured in
        // evidence. No declaration here is reported as a contiguous allocation.
        let node = (size_of::<(alloc::string::String, alloc::string::String)>() * 11
            + size_of::<usize>() * 32) as u64;
        add(
            mul(self.input[0].max(1), node)?,
            mul(self.input[0], mul(2, self.input[2])?)?,
        )
    }
    fn terminal_fixed_work(&self) -> CoreResult<u64> {
        // One call/result, complete registration, bridge/root transport, report
        // and the fixed local/parent account array. All arithmetic precedes TD.
        add(
            mul(
                self.work[5],
                add(
                    u64::from(self.call.retained_items()),
                    add(u64::from(self.registration.retained_items()), 4)?,
                )?,
            )?,
            (N as u64) * 4 + 8,
        )
    }
    fn call_temporary(&self, resources: BindingResourceDeclarations) -> CoreResult<u64> {
        // Callback scratch is already paired in the durable registration
        // owner, including for terminal destructors after the final call.
        // Reserve only constructor overlap here; check the whole operation.
        let n = add(self.call.retained_bytes(), self.core_backing())?;
        self.check(
            K::AdmissionTemporaryBytesPerOperationMax,
            add(n, resources.transient().peak_bytes())?,
        )?;
        Ok(n)
    }
    fn core_backing(&self) -> u64 {
        // Covers both Core mediators, including the larger accepted-error arm.
        // The concrete Core allocation is observed separately by integration.
        size_of::<(
            CoreError,
            CoreError,
            OutboundRequest,
            BindingLifetimeFootprint,
            [usize; 8],
        )>() as u64
    }
    fn call_bytes(&self) -> CoreResult<u64> {
        let result = if self.contract == BindingResourceContract::StrictV1 {
            add(self.result[0], mul(2, self.result[1])?)?
        } else {
            let body = usize::try_from(self.result[0]).map_err(|_| unsupported())?;
            let arc = core::alloc::Layout::new::<[usize; 2]>()
                .extend(core::alloc::Layout::array::<u8>(body).map_err(|_| unsupported())?)
                .map_err(|_| unsupported())?
                .0
                .pad_to_align();
            add(arc.size() as u64, mul(2, self.result[1])?)?
        };
        add(
            add(self.call.retained_bytes(), self.core_backing())?,
            add(self.request_bound()?, result)?,
        )
    }
    fn resource_error(&self, error: CoreError, domain: ResourceFailureDomain) -> CoreError {
        stamp_resource(error, self.identity, domain, None, None)
    }
    fn result_error(&self, error: CoreError) -> CoreError {
        // Core's owned result seal knows the backing bound and selected call,
        // while this Runtime owns the complete engine policy. Enrich only that
        // refusal; preserve strict errors and any binding-private context.
        if self.contract != BindingResourceContract::StrictV1
            && matches!(&error, CoreError::Validation(c) if c.cause_code() == Some(327))
        {
            self.resource_error(error, ResourceFailureDomain::EngineCapacity)
        } else {
            error
        }
    }
    fn build_error(&self, error: CoreError) -> CoreError {
        if self.contract == BindingResourceContract::StrictV1 {
            return error;
        }
        // The closed TD/Planning path reports allocator refusal as unqualified
        // Backpressure; the supported Host compiler uses cause 322. Capped
        // ledger refusal has a distinct cause (331). Keep those distinctions
        // and any original binding-private context without guessing a byte row.
        let domain = match &error {
            CoreError::Backpressure(c)
                if c.cause_code().is_none() || c.cause_code() == Some(322) =>
            {
                ResourceFailureDomain::DeploymentFailure
            }
            CoreError::Backpressure(_) | CoreError::LimitExceeded { .. } => {
                ResourceFailureDomain::EngineCapacity
            }
            _ => return error,
        };
        self.resource_error(error, domain)
    }
    fn check(&self, kind: K, n: u64) -> CoreResult<()> {
        check(&self.limits, kind, n).map_err(|error| {
            // This boundary knows the row's owner even when its value is
            // missing. TD/Planning allocation classification is separate.
            if self.contract == BindingResourceContract::StrictV1 {
                error
            } else {
                self.resource_error(error, ResourceFailureDomain::EngineCapacity)
            }
        })
    }
    fn ledger(&self, generation: Generation, runtime: u64) -> AdmissionLedger {
        AdmissionLedger::new(
            SlotIndex::new(0),
            generation,
            0,
            self.scratch / 2,
            0,
            runtime,
            size_of::<CoreError>() as u64,
            self.limit(K::CleanupBytesMax),
        )
    }
    fn limit(&self, k: K) -> u64 {
        self.limits.get(k).expect("checked applicable field")
    }
    fn validate_input(
        &self,
        name: &str,
        options: &InteractionOptions,
        budget: &mut WorkBudget,
        census: Preflight,
    ) -> CoreResult<()> {
        if options.data.is_some()
            || name.len() as u64 > self.input[3]
            || options.uri_variables.len() as u64 > self.input[0]
        {
            return Err(invalid());
        }
        let comparisons = add(add(census.property_count, census.coordinate_count)?, 1)?;
        let bytes = add(
            mul(comparisons, self.input[3])?,
            mul(options.uri_variables.len() as u64, mul(2, self.input[2])?)?,
        )?;
        if !pay(
            budget,
            &[
                (W::QueueOperations, 1),
                (W::CodecInputBytes, bytes),
                (W::BindingPolls, add(self.work[0], self.work[1])?),
            ],
        ) {
            return Err(if self.contract == BindingResourceContract::StrictV1 {
                busy()
            } else {
                self.resource_error(busy(), ResourceFailureDomain::EngineCapacity)
            });
        }
        for (key, value) in &options.uri_variables {
            if [key, value]
                .iter()
                .any(|s| s.len() as u64 > self.input[1] || s.capacity() as u64 > self.input[2])
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    fn deadline(
        &self,
        now: MonotonicInstant,
        clock: &dyn RuntimeClock,
        options: &InteractionOptions,
    ) -> CoreResult<Deadline> {
        now.checked_add_ticks(self.timeouts[1])
            .ok_or_else(invalid)?;
        let ticks = if let Some(t) = options.timeout() {
            let nanos = t.as_nanos();
            let ticks = nanos
                .checked_mul(u128::from(clock.ticks_per_second().get()))
                .ok_or_else(invalid)?
                / 1_000_000_000;
            u64::try_from(ticks)
                .map_err(|_| invalid())?
                .min(self.timeouts[0])
        } else {
            self.timeouts[0]
        };
        now.checked_add_ticks(ticks)
            .map(Deadline::at)
            .ok_or_else(invalid)
    }
    fn startup_time(&self, clock: &dyn RuntimeClock, host: bool) -> CoreResult<MonotonicInstant> {
        let now = clock.now();
        now.checked_add_ticks(self.timeouts[0].max(self.timeouts[1]))
            .ok_or_else(unsupported)?;
        if host {
            let maximum = self
                .limits
                .get(K::HostBindingCancelDrainTimeoutMillisMax)
                .ok_or_else(unsupported)?;
            if u128::from(self.timeouts[1]) * 1000
                > u128::from(maximum) * u128::from(clock.ticks_per_second().get())
            {
                return Err(self.build_error(CoreError::LimitExceeded {
                    resource: K::HostBindingCancelDrainTimeoutMillisMax,
                    limit: maximum,
                    requested: None,
                    observed: None,
                    context: context(),
                }));
            }
        }
        Ok(now)
    }
}

trait Parent: Clone {
    fn edit<T>(&self, f: impl FnOnce(&mut [ResourceAccount; N]) -> T) -> T;
}
#[derive(Clone, Copy)]
struct StaticParent<'a>(&'a RefCell<[ResourceAccount; N]>);
impl Parent for StaticParent<'_> {
    fn edit<T>(&self, f: impl FnOnce(&mut [ResourceAccount; N]) -> T) -> T {
        f(&mut self.0.borrow_mut())
    }
}
struct Charge<P: Parent> {
    parent: P,
    local: [ResourceAccount; N],
    held: [u64; N],
    identity: BindingConfigurationDigest,
    owned: bool,
}
impl<P: Parent> Charge<P> {
    fn is_held(&self) -> bool {
        self.local.iter().zip(self.held).all(|(a, n)| a.used() == n)
            && self.parent.edit(|a| {
                a.iter()
                    .enumerate()
                    .all(|(i, a)| a.kind() == KINDS[i] && a.used() >= self.held[i])
            })
    }
    fn new(parent: P, policy: &ConsumerPropertyReadPolicy) -> CoreResult<Self> {
        let valid = parent.edit(|a| a.iter().zip(KINDS).all(|(a, k)| a.kind() == k));
        if !valid {
            return Err(invalid());
        }
        Ok(Self {
            parent,
            local: policy.parent_accounts(),
            held: [0; N],
            identity: policy.identity,
            owned: policy.contract != BindingResourceContract::StrictV1,
        })
    }
    fn grow(&mut self, amount: [u64; N]) -> CoreResult<()> {
        for (i, n) in amount.iter().copied().enumerate() {
            if self.held[i]
                .checked_add(n)
                .is_none_or(|v| v > self.local[i].limit())
            {
                return Err(self.capacity_error(i, n, None));
            }
        }
        let rejected = self.parent.edit(|a| {
            let mut acquired = 0;
            for i in 0..N {
                if let Some(r) = a[i].try_reserve(amount[i]) {
                    r.commit();
                    acquired += 1;
                } else {
                    for j in 0..acquired {
                        assert!(a[j].release_committed(amount[j]));
                    }
                    let owner = a[i]
                        .try_reserve(0)
                        .expect("zero receipt retains account totals")
                        .owner();
                    return Some((i, owner.0, owner.1, a[i].limit(), a[i].used()));
                }
            }
            None
        });
        if let Some((i, slot, generation, limit, used)) = rejected {
            return Err(self.capacity_error(i, amount[i], Some((slot, generation, limit, used))));
        }
        for (i, n) in amount.into_iter().enumerate() {
            self.held[i] += n;
            // Planning commits its original cleanup account itself.
            if i != CLEANUP_BYTES {
                self.local[i]
                    .try_reserve(n)
                    .expect("local checked")
                    .commit();
            }
        }
        Ok(())
    }
    fn capacity_error(
        &self,
        i: usize,
        n: u64,
        parent: Option<(SlotIndex, Generation, u64, u64)>,
    ) -> CoreError {
        if !self.owned {
            return busy();
        }
        // These are the exact local coordinates minted by parent_accounts.
        let local = Some((SlotIndex::new(i as u32), Generation::INITIAL));
        let (limit, used) = parent
            .map(|(_, _, l, u)| (l, u))
            .unwrap_or((self.local[i].limit(), self.held[i]));
        let error = CoreError::LimitExceeded {
            resource: KINDS[i],
            limit,
            requested: used.checked_add(n),
            observed: Some(used),
            context: context(),
        };
        stamp_resource(
            error,
            self.identity,
            ResourceFailureDomain::EngineCapacity,
            local,
            parent.map(|(s, g, _, _)| (s, g)),
        )
    }
    fn release(&mut self, amount: [u64; N]) {
        for (i, n) in amount.iter().copied().enumerate() {
            self.held[i] = self.held[i]
                .checked_sub(n)
                .expect("original held allowance");
            if i != CLEANUP_BYTES {
                assert!(self.local[i].release_committed(n));
            }
        }
        self.parent.edit(|a| {
            for (a, n) in a.iter_mut().zip(amount) {
                assert!(a.release_committed(n));
            }
        });
    }
}
impl<P: Parent> Drop for Charge<P> {
    fn drop(&mut self) {
        // All physical children must already have been disposed by their owner.
        let cleanup = self.local[CLEANUP_BYTES].used();
        assert!(self.local[CLEANUP_BYTES].release_committed(cleanup));
        self.release(self.held);
    }
}
fn amounts(pairs: &[(usize, u64)]) -> [u64; N] {
    let mut a = [0; N];
    for &(i, n) in pairs {
        a[i] = n;
    }
    a
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Cause {
    Manual,
    Deadline,
    Drop,
    Drain,
}
impl Cause {
    fn error(self) -> CoreError {
        match self {
            Self::Deadline => CoreError::TimedOut(context()),
            _ => CoreError::Cancelled(
                context().with_redacted_cause(401 + self as u16, "Consumer cancellation"),
            ),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CallPhase {
    Vacant,
    Starting,
    Running,
    Cancelling,
    AwaitingResidual,
    #[cfg(feature = "std")]
    Manual,
    Result,
    #[cfg(feature = "std")]
    Acknowledging,
}
struct Kernel {
    phase: CompiledPlanSetState,
    slot: Generation,
    plan: PlanSetGeneration,
    call: Generation,
    nonce: u64,
    first: Option<Cause>,
    call_first: Option<Cause>,
    call_phase: CallPhase,
    interested: bool,
    handle_live: bool,
    ticket_live: bool,
    closed: bool,
    last_now: MonotonicInstant,
    cancel_deadline: Option<Deadline>,
    call_deadline: Option<Deadline>,
    shutdown: bool,
    discard_results: bool,
    residual: bool,
    td_cause: Option<ValidatedThingCause>,
    build_error: Option<CoreError>,
}
impl Kernel {
    fn new(now: MonotonicInstant) -> Self {
        Self {
            phase: CompiledPlanSetState::Reclaimed,
            slot: Generation::INITIAL,
            plan: PlanSetGeneration::INITIAL,
            call: Generation::INITIAL,
            nonce: 0,
            first: None,
            call_first: None,
            call_phase: CallPhase::Vacant,
            interested: false,
            handle_live: false,
            ticket_live: false,
            closed: false,
            last_now: now,
            cancel_deadline: None,
            call_deadline: None,
            shutdown: false,
            discard_results: false,
            residual: false,
            td_cause: None,
            build_error: None,
        }
    }
    fn begin(&mut self) -> CoreResult<()> {
        if self.closed
            || self.phase != CompiledPlanSetState::Reclaimed
            || self.handle_live
            || self.ticket_live
        {
            return Err(busy());
        }
        let slot = self.slot.checked_next().ok_or_else(unsupported)?;
        let plan = self.plan.checked_next().ok_or_else(unsupported)?;
        let nonce = self.nonce.checked_add(1).ok_or_else(unsupported)?;
        self.slot = slot;
        self.plan = plan;
        self.nonce = nonce;
        self.first = None;
        self.td_cause = None;
        self.build_error = None;
        self.phase = CompiledPlanSetState::Building;
        Ok(())
    }
    fn close_generation(&mut self, cause: Cause, ticks: u64) {
        if matches!(
            self.phase,
            CompiledPlanSetState::Building | CompiledPlanSetState::Frozen
        ) {
            self.build_error.get_or_insert_with(|| cause.error());
        }
        if self.first.is_none() {
            self.first = Some(cause);
        }
        if self.phase != CompiledPlanSetState::Reclaimed {
            self.phase = CompiledPlanSetState::Draining;
        }
        self.cancel(cause, ticks);
    }
    fn cancel(&mut self, cause: Cause, _ticks: u64) {
        if self.call_phase != CallPhase::Vacant && self.call_first.is_none() {
            self.call_first = Some(cause);
            // Child Drop records intent only. The next manual control step
            // samples the clock and binds the one independent drain deadline.
            self.cancel_deadline = None;
        }
    }
    fn start_call(&mut self, slot: Generation, deadline: Deadline) -> CoreResult<Generation> {
        if slot != self.slot {
            return Err(stale());
        }
        if self.phase != CompiledPlanSetState::Published
            || self.closed
            || self.call_phase != CallPhase::Vacant
            || self.ticket_live
        {
            return Err(busy());
        }
        self.call = self.call.checked_next().ok_or_else(unsupported)?;
        self.call_phase = CallPhase::Starting;
        self.call_first = None;
        self.cancel_deadline = None;
        self.call_deadline = Some(deadline);
        self.interested = true;
        Ok(self.call)
    }
    fn expire(&mut self, now: MonotonicInstant, ticks: u64) {
        if now.clock_id() != self.last_now.clock_id()
            || now.checked_duration_since(self.last_now).is_none()
        {
            self.cancel(Cause::Deadline, ticks);
            if self.call_first.is_some() {
                self.cancel_deadline = Some(Deadline::at(self.last_now));
            }
            return;
        }
        self.last_now = now;
        if self
            .call_deadline
            .is_some_and(|d| d.checked_is_elapsed_at(now) != Some(false))
        {
            self.cancel(Cause::Deadline, ticks);
        }
        if self.call_first.is_some() && self.cancel_deadline.is_none() {
            self.cancel_deadline = Some(Deadline::at(now.checked_add_ticks(ticks).unwrap_or(now)));
        }
    }
    fn next_deadline(&self) -> Option<Deadline> {
        if self.call_first.is_some() {
            self.cancel_deadline.or(Some(Deadline::at(self.last_now)))
        } else {
            self.call_deadline
        }
    }
    fn cancel_expired(&self) -> bool {
        self.call_first.is_some()
            && self
                .cancel_deadline
                .is_some_and(|d| d.checked_is_elapsed_at(self.last_now) != Some(false))
    }
    fn settle(&mut self) {
        self.call_phase = CallPhase::Result;
    }
    fn acknowledge(&mut self) {
        self.call_phase = CallPhase::Vacant;
        self.call_first = None;
        self.call_deadline = None;
        self.cancel_deadline = None;
        self.interested = false;
    }
    fn final_checks(
        &self,
        nonce: u64,
        slot: Generation,
        plan: PlanSetGeneration,
        registration: BindingRegistrationIdentity,
        actual: BindingRegistrationIdentity,
        capacity: bool,
    ) -> CoreResult<()> {
        if self.nonce != nonce || self.slot != slot || self.plan != plan {
            return Err(stale());
        }
        if self.first.is_some() || self.closed {
            return Err(self.first.unwrap_or(Cause::Drain).error());
        }
        if self.phase != CompiledPlanSetState::Frozen
            || registration != actual
            || !capacity
            || self.call_phase != CallPhase::Vacant
        {
            return Err(invariant());
        }
        Ok(())
    }
    fn record(&self, code: u16) -> CleanupRecord {
        let id = CleanupSlotId::new(SlotIndex::new(0), self.call);
        CleanupRecord::try_new(
            CleanupHandle::new(id),
            id,
            CleanupSlotId::new(SlotIndex::new(1), self.slot),
            CleanupOperation::CancelRequest,
            0,
            RetryClass::Never,
            code,
            0,
        )
        .expect("zero retries")
        .with_diagnostic_identities(
            Some(ThingSlotId::new(SlotIndex::new(0), self.slot)),
            None,
            None,
            None,
        )
    }
    fn cleanup_phase(
        &self,
        policy: &ConsumerPropertyReadPolicy,
        transfer: bool,
    ) -> CleanupPhaseContext {
        let subject = CleanupSlotId::new(SlotIndex::new(0), self.call);
        let reservation = CleanupReservation::new(
            subject,
            BindingLifetimeFootprint::new(
                policy.call.retained_items() + 1,
                policy.call_bytes().expect("checked policy"),
            ),
            1,
            WorkBudget::new()
                .with_remaining(W::BindingPolls, policy.work[4])
                .with_remaining(W::CleanupItems, 1),
        );
        let cause = self.call_first.expect("cancellation committed").error();
        let deadline = self.cancel_deadline.unwrap_or(Deadline::at(self.last_now));
        if transfer {
            CleanupPhaseContext::bind_with_transfer_owner(
                reservation,
                CleanupSlotId::new(SlotIndex::new(1), self.slot),
                CleanupOperation::CancelRequest,
                cause,
                deadline,
            )
        } else {
            CleanupPhaseContext::bind(
                reservation,
                CleanupOperation::CancelRequest,
                cause,
                deadline,
            )
        }
    }
}

struct Terminal {
    work: WorkBudget,
    steps: u64,
    used_steps: u64,
    #[cfg(feature = "std")]
    bridge: WorkBudget,
}
impl Terminal {
    fn empty() -> Self {
        Self {
            work: WorkBudget::new()
                .with_remaining(W::PlanningItems, 4)
                .with_remaining(W::CleanupItems, 4)
                .with_remaining(W::QueueOperations, 4),
            steps: 4,
            used_steps: 0,
            #[cfg(feature = "std")]
            bridge: WorkBudget::new().with_remaining(W::PlanningItems, 2),
        }
    }
    fn from_preflight(p: Preflight) -> CoreResult<Self> {
        let items = add(p.remaining_planning_items, 3)?;
        let steps = add(p.output_capacity_bytes, items)?;
        Ok(Self {
            work: WorkBudget::new()
                .with_remaining(W::PlanningItems, items)
                .with_remaining(W::CleanupItems, items)
                .with_remaining(W::CodecInputBytes, p.output_capacity_bytes)
                .with_remaining(W::QueueOperations, steps),
            steps,
            used_steps: 0,
            #[cfg(feature = "std")]
            bridge: WorkBudget::new().with_remaining(W::PlanningItems, 2),
        })
    }
    fn discard<A: 'static>(&mut self, mut rollback: Rollback<A>) {
        loop {
            assert!(
                self.used_steps < self.steps,
                "prepaid terminal bound exhausted"
            );
            self.used_steps += 1;
            self.work
                .consume(W::QueueOperations, 1)
                .expect("prepaid coordinator");
            match rollback.step(&mut self.work) {
                ReclaimStep::Pending(next) => rollback = next,
                ReclaimStep::Complete(_) => break,
            }
        }
    }
}
struct BuildStorage<A: 'static, P: Parent> {
    root: Option<PlanRoot<A>>,
    draft: Option<Draft<A>>,
    rollback: Option<Rollback<A>>,
    charge: Option<Charge<P>>,
    terminal: Terminal,
    census: Option<Preflight>,
    cause: Option<CoreError>,
    td_cause: Option<ValidatedThingCause>,
    build_amount: [u64; N],
}
impl<A: 'static, P: Parent> BuildStorage<A, P> {
    fn new(parent: P, policy: &ConsumerPropertyReadPolicy, upstream: u64) -> CoreResult<Self> {
        let mut charge = Charge::new(parent, policy)?;
        let root_bytes = PlanRoot::<A>::required_capacity();
        policy.check(K::CleanupItemBytesMax, root_bytes)?;
        let build = add(policy.scratch, size_of::<Self>() as u64)?;
        policy.check(K::AdmissionTemporaryBytesPerOperationMax, build)?;
        let build_amount = amounts(&[(ENGINE, build), (TEMP, build), (PEAK, build)]);
        policy.check(K::PeakLiveBytesPerAdmissionMax, add(build, upstream)?)?;
        let mut initial = build_amount;
        initial[PEAK] = add(initial[PEAK], upstream)?;
        initial[CLEANUP_BYTES] = root_bytes;
        initial[CLEANUP_ITEMS] = 1;
        charge.grow(initial)?;
        let root = PlanRoot::try_new(
            &policy.planning,
            charge.local[CLEANUP_BYTES]
                .try_reserve(root_bytes)
                .expect("held parent and local capacity"),
        )
        .map_err(|(_, e)| policy.build_error(e))?;
        Ok(Self {
            root: Some(root),
            draft: None,
            rollback: None,
            charge: Some(charge),
            terminal: Terminal::empty(),
            census: None,
            cause: None,
            td_cause: None,
            build_amount,
        })
    }
    fn end_scratch(&mut self) {
        let mut amount = core::mem::replace(&mut self.build_amount, [0; N]);
        if let Some(charge) = &mut self.charge {
            // The consumed continuation has already destroyed or aborted its
            // compiler cursor and ended all source/scratch loans.
            amount[CURSOR] = charge.held[CURSOR];
            charge.release(amount);
        }
    }
    fn local_discard(&mut self) {
        if let Some(draft) = self.draft.take() {
            self.rollback = Some(draft.into_rollback());
        }
        if self.rollback.is_none() {
            self.rollback = self.root.as_mut().and_then(PlanRoot::take);
        }
        if let Some(rollback) = self.rollback.take() {
            self.terminal.discard(rollback);
        }
        drop(self.root.take());
        drop(self.charge.take());
    }
}
impl<A: 'static, P: Parent> Drop for BuildStorage<A, P> {
    fn drop(&mut self) {
        self.local_discard();
    }
}

// Inline caller storage deliberately avoids a new allocation at every phase.
#[allow(clippy::large_enum_variant)]
enum Building<'td, 'env, R: ConsumerPropertyReadRegistration> {
    Td(ValidatedThingCursor<'td>, &'env mut PlanRoot<R::Artifact>),
    Planning(PlanningBuild<'td, 'env, R>),
    End,
}
struct Build<'td, 'env, R: ConsumerPropertyReadRegistration, P: Parent> {
    state: Building<'td, 'env, R>,
    registration: &'env R,
    policy: &'env ConsumerPropertyReadPolicy,
    generation: PlanSetGeneration,
    upstream: u64,
    charge: &'env mut Charge<P>,
    terminal: &'env mut Terminal,
    census: &'env mut Option<Preflight>,
    draft: &'env mut Option<Draft<R::Artifact>>,
    rollback: &'env mut Option<Rollback<R::Artifact>>,
    cause: &'env mut Option<CoreError>,
    td_cause: &'env mut Option<ValidatedThingCause>,
}
impl<'td, 'env, R: ConsumerPropertyReadRegistration, P: Parent> Build<'td, 'env, R, P> {
    fn step(&mut self, budget: &mut WorkBudget, cancel: bool) -> StepStatus<()> {
        let state = core::mem::replace(&mut self.state, Building::End);
        match state {
            Building::Td(cursor, root) => match cursor.step(budget, cancel) {
                ValidatedThingProgress::Pending(cursor) => self.state = Building::Td(cursor, root),
                ValidatedThingProgress::Failed(e) => {
                    *self.td_cause = Some(e);
                    *self.cause = Some(self.policy.build_error(td_error(e)));
                }
                ValidatedThingProgress::Complete(proof) => {
                    match build_consumer_property_read_aggregate(
                        proof,
                        self.registration,
                        &self.policy.planning,
                        self.generation,
                        self.policy.ledger(self.generation.get(), 0),
                        root,
                    ) {
                        Ok(build) => self.state = Building::Planning(build),
                        Err(e) => *self.cause = Some(self.policy.build_error(e)),
                    }
                }
            },
            Building::Planning(build) => match build.step(budget, cancel) {
                PlanningStep::Pending(build) => self.state = Building::Planning(build),
                PlanningStep::Preflight {
                    build,
                    requirements: p,
                } => {
                    // Private association: the only waiting build, original
                    // accounts, captured registration, and generations are all
                    // retained here. A public tuple/ledger is never authority.
                    let admitted = (|| {
                        if p.registration != self.registration.identity() {
                            return Err(invariant());
                        }
                        let terminal = Terminal::from_preflight(p)?;
                        let output = p.output_capacity_bytes;
                        let cleanup = add(
                            size_of::<(CoreError, Preflight, Terminal)>() as u64,
                            self.policy.call_bytes()?,
                        )?;
                        self.policy.check(K::CleanupItemBytesMax, cleanup)?;
                        self.policy
                            .check(K::CompiledRuntimeBytesPerThingMax, output)?;
                        let peak = add(
                            self.upstream,
                            add(self.charge.held[ENGINE], add(output, cleanup)?)?,
                        )?;
                        self.policy.check(K::PeakLiveBytesPerAdmissionMax, peak)?;
                        #[cfg(feature = "std")]
                        let artifact = self
                            .registration
                            .support()
                            .host_slot_layout()
                            .unwrap_or(self.registration.support().artifact_layout())
                            .size() as u64;
                        #[cfg(not(feature = "std"))]
                        let artifact = self.registration.support().artifact_layout().size() as u64;
                        self.policy.check(
                            K::BindingArtifactBytesPerThingMax,
                            mul(p.coordinate_count, artifact)?,
                        )?;
                        self.charge.grow(amounts(&[
                            (ENGINE, add(output, cleanup)?),
                            (PEAK, add(output, cleanup)?),
                            (OUTPUT, output),
                            (CURSOR, p.cursor_capacity_bytes),
                            (ARTIFACTS, p.coordinate_count),
                            (ARTIFACT_BYTES, mul(p.coordinate_count, artifact)?),
                            (SETS, 1),
                            (THINGS, 1),
                            (CLEANUP_BYTES, cleanup),
                            (CLEANUP_ITEMS, 1),
                        ]))?;
                        self.charge.local[CLEANUP_BYTES]
                            .try_reserve(cleanup)
                            .expect("paired capacity")
                            .commit();
                        *self.terminal = terminal;
                        *self.census = Some(p);
                        Ok(())
                    })();
                    match admitted {
                        Ok(()) => match build.admit_runtime(
                            self.policy
                                .ledger(self.generation.get(), p.output_capacity_bytes),
                        ) {
                            Ok(build) => self.state = Building::Planning(build),
                            Err((build, ledger, e)) => {
                                drop(build);
                                let _ = ledger;
                                *self.cause = Some(self.policy.build_error(e));
                            }
                        },
                        Err(e) => {
                            drop(build);
                            *self.cause = Some(self.policy.build_error(e));
                        }
                    }
                }
                PlanningStep::Complete(draft) => *self.draft = Some(draft),
                PlanningStep::Failed { cause, cleanup } => {
                    *self.cause = Some(self.policy.build_error(cause));
                    *self.rollback = cleanup;
                }
            },
            Building::End => {}
        }
        if matches!(self.state, Building::End) {
            StepStatus::progress(Some(()), None)
        } else {
            StepStatus::progress(None, Some(PendingWork::new(PendingWorkClass::BindingInput)))
        }
    }
    fn is_finished(&self) -> bool {
        matches!(self.state, Building::End)
    }
}

type StaticRegistration<C> = ConsumerCompilerRegistration<StaticConsumerBindingRegistration<C>>;
type Artifact<C> = <<C as PollClientBinding>::Compiler as BindingCompilerExtension>::Artifact;

/// Caller-owned build adapter; it cannot outlive its `consume` progress scope.
pub struct StaticConsumerPropertyReadBuild<'td, 'env, 'capacity, C: PollClientBinding>
where
    C::Compiler: 'static,
    Artifact<C>: 'static,
{
    build: Build<'td, 'env, StaticRegistration<C>, StaticParent<'capacity>>,
    cancelled: bool,
    error: &'env mut Option<CoreError>,
}
impl<C: PollClientBinding> StaticConsumerPropertyReadBuild<'_, '_, '_, C>
where
    C::Compiler: 'static,
    Artifact<C>: 'static,
{
    pub fn step(&mut self, budget: &mut WorkBudget) -> StepStatus<()> {
        let result = self.build.step(budget, self.cancelled);
        if let Some(error) = self.build.cause.clone() {
            self.error.get_or_insert(error);
        }
        result
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.error.get_or_insert_with(|| {
            self.build
                .cause
                .clone()
                .unwrap_or_else(|| Cause::Manual.error())
        });
    }
    pub fn is_finished(&self) -> bool {
        self.build.is_finished()
    }
}

// Static runtime implementation follows the same kernel and retains the real
// associated state; no erased owner or reference-counted Host graph is used.
mod application_static;
pub use application_static::StaticConsumerPropertyReadRuntime;
