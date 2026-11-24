//! Host storage erasure retains the Servient operation as well as the native
//! owner. Bindings never receive a dispatcher or logical registry.
use alloc::{boxed::Box, rc::Weak};
use clinkz_wot_core::{
    CoreError, CoreResult, ErrorContext, ErrorPhase, RetryClass,
    action::ProducerActionBinding,
    finite::FiniteIngressBinding,
    publication::{ClosedProducerParticipation, ProducerNotificationBinding},
};
use clinkz_wot_foundation::{MonotonicInstant, WorkBudget};
use clinkz_wot_servient_finite::{ProducerFiniteSettlement, RawFiniteRuntime, RawProducerServient};
use core::{
    any::Any,
    mem::size_of,
    ops::{Deref, DerefMut},
    task::Context,
};

/// Independently funded lanes: native ingress must not spend the allowance
/// needed by an already accepted Action or its cleanup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerWork {
    Participation,
    Actions,
}

type Turn<T> = fn(
    &mut T,
    &mut RawProducerServient,
    &mut Context<'_>,
    MonotonicInstant,
    &mut dyn FnMut(ProducerWork) -> WorkBudget,
) -> CoreResult<()>;

trait StoredParticipation {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn native(&self) -> &(dyn ClosedProducerParticipation + 'static);
    fn native_mut(&mut self) -> &mut (dyn ClosedProducerParticipation + 'static);
    fn take_finite_settlement(&mut self) -> Option<Box<ProducerFiniteSettlement>>;
    fn overhead(&self) -> usize;
    fn step(
        &mut self,
        owner: &mut RawProducerServient,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        credit: &mut dyn FnMut(ProducerWork) -> WorkBudget,
    ) -> CoreResult<()>;
}
struct Stored<T> {
    native: T,
    turn: Turn<T>,
    settlement: fn(&mut T) -> Option<Box<ProducerFiniteSettlement>>,
}
impl<T: ClosedProducerParticipation + Any> StoredParticipation for Stored<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut self.native
    }
    fn native(&self) -> &(dyn ClosedProducerParticipation + 'static) {
        &self.native
    }
    fn native_mut(&mut self) -> &mut (dyn ClosedProducerParticipation + 'static) {
        &mut self.native
    }
    fn take_finite_settlement(&mut self) -> Option<Box<ProducerFiniteSettlement>> {
        (self.settlement)(&mut self.native)
    }
    fn overhead(&self) -> usize {
        size_of::<Self>() - size_of::<T>()
    }
    fn step(
        &mut self,
        owner: &mut RawProducerServient,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        credit: &mut dyn FnMut(ProducerWork) -> WorkBudget,
    ) -> CoreResult<()> {
        (self.turn)(&mut self.native, owner, cx, now, credit)
    }
}

/// One original native member and its typed Servient turn, erased together.
/// Replaces storage-only erasure; no work/result registry or scheduler is added.
pub struct OwnedProducerParticipation {
    original: Weak<()>,
    failure: Option<CoreError>,
    native: Box<dyn StoredParticipation>,
}
impl OwnedProducerParticipation {
    /// Extension seam for a linked capability's bounded Servient-side turn.
    /// Construction is pure. The turn must preserve original services, work,
    /// outcomes and cleanup; it must not publish, acknowledge or replay work.
    pub fn new<T: ClosedProducerParticipation + Any>(native: T, turn: Turn<T>) -> Self {
        Self::with_settlement(native, turn, |_| None)
    }
    fn with_settlement<T: ClosedProducerParticipation + Any>(
        native: T,
        turn: Turn<T>,
        settlement: fn(&mut T) -> Option<Box<ProducerFiniteSettlement>>,
    ) -> Self {
        let original = native.participation_id().downgrade();
        Self {
            native: Box::new(Stored {
                native,
                turn,
                settlement,
            }),
            original,
            failure: None,
        }
    }
    pub fn finite<B: FiniteIngressBinding + 'static>(native: RawFiniteRuntime<B>) -> Self {
        Self::with_settlement(
            native,
            |native, owner, cx, now, credit| {
                owner
                    .step(native, cx, &mut credit(ProducerWork::Participation), now)
                    .map(|_| ())
            },
            RawFiniteRuntime::take_retained_settlement,
        )
    }
    pub(crate) fn take_finite_settlement(&mut self) -> Option<Box<ProducerFiniteSettlement>> {
        if self.has_original_native() {
            self.native.take_finite_settlement()
        } else {
            None
        }
    }
    pub fn actions<B: ProducerActionBinding + Any>(native: B) -> Self {
        Self::new(native, |native, owner, _, _, credit| {
            // Both lanes run even when native polling returns an error or has
            // no credit. Original continuations never depend on new ingress.
            let ingress = if native.retired() {
                Ok(())
            } else {
                owner
                    .step_actions(native, &mut credit(ProducerWork::Participation))
                    .map(|_| ())
            };
            let mut work = credit(ProducerWork::Actions);
            let progress = owner
                .progress_actions_closing_revoked(native, &mut work)
                .map(|_| ());
            combine(ingress, progress)
        })
    }
    pub fn notifications<B: ProducerNotificationBinding + ClosedProducerParticipation + Any>(
        native: B,
    ) -> Self {
        Self::new(native, |native, owner, cx, _, credit| {
            owner
                .step_notifications(native, cx, &mut credit(ProducerWork::Participation))
                .map(|_| ())
        })
    }
    pub fn as_any_mut(&mut self) -> &mut dyn Any {
        self.native.as_any_mut()
    }
    pub fn as_participation(&self) -> &dyn ClosedProducerParticipation {
        self.native.native()
    }
    pub fn as_participation_mut(&mut self) -> &mut dyn ClosedProducerParticipation {
        self.native.native_mut()
    }
    pub(crate) fn overhead(&self) -> usize {
        self.native.overhead()
    }
    pub(crate) fn has_original_native(&self) -> bool {
        self.participation_id().matches(&self.original)
    }
    pub(crate) fn failure(&self) -> Option<&CoreError> {
        self.failure
            .as_ref()
            .or_else(|| self.native.native().native_failure())
    }
    pub(crate) fn step(
        &mut self,
        owner: &mut RawProducerServient,
        key: clinkz_wot_core::producer::ProducerInstanceKey,
        cx: &mut Context<'_>,
        now: MonotonicInstant,
        credit: &mut dyn FnMut(ProducerWork) -> WorkBudget,
    ) -> CoreResult<()> {
        let result = if !self.has_original_native()
            || !self.associated_with(&owner.participation_token(), key)
        {
            Err(CoreError::StaleHandle(ErrorContext::new(
                ErrorPhase::Binding,
                RetryClass::Never,
            )))
        } else {
            let result = self.native.step(owner, cx, now, credit);
            // Native failure is evidence; only the logical owner can revoke the
            // committed incarnation and request its participant's retirement.
            let reconcile = owner.reconcile_participation(key, self.as_participation_mut());
            combine(result, reconcile)
        };
        if let Err(error) = &result
            && !matches!(error, CoreError::Backpressure(_))
        {
            self.failure.get_or_insert_with(|| error.clone());
        }
        result
    }
}

fn combine(first: CoreResult<()>, second: CoreResult<()>) -> CoreResult<()> {
    match (first, second) {
        (Ok(()), result) => result,
        (Err(CoreError::Backpressure(_)), Err(error)) => Err(error),
        (Err(error), _) => Err(error),
    }
}
impl Deref for OwnedProducerParticipation {
    type Target = dyn ClosedProducerParticipation;
    fn deref(&self) -> &Self::Target {
        self.native.native()
    }
}
impl DerefMut for OwnedProducerParticipation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.native.native_mut()
    }
}
