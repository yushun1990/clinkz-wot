//! Deterministic, bounded compilation of validated Thing Descriptions.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use clinkz_wot_core::{
    BindingArtifactEnvelope, BindingArtifactRef, CoreError, CoreResult, LogicalInteractionPlan,
    PlanSetGeneration,
};
use clinkz_wot_foundation::WorkBudget;
use clinkz_wot_td::thing::Thing;

// WP-200 owns the algorithm now; WP-300 will provide the first product
// consumer through a complete registration. Inline tests are its current
// executable consumer.
#[allow(dead_code)]
mod property_read;
pub use property_read::{
    PropertyReadBuildCursor, PropertyReadPlanCompiler, select_consumer_property_read,
};

/// Borrowed immutable inputs for one plan-build transaction.
#[derive(Clone, Copy)]
pub struct PlanBuildInput<'a, R: ?Sized> {
    validated_td: &'a Thing,
    registrations: &'a R,
    plan_set_generation: PlanSetGeneration,
}

impl<'a, R: ?Sized> PlanBuildInput<'a, R> {
    /// Captures one validated TD, registration snapshot, and plan-set generation.
    pub const fn new(
        validated_td: &'a Thing,
        registrations: &'a R,
        plan_set_generation: PlanSetGeneration,
    ) -> Self {
        Self {
            validated_td,
            registrations,
            plan_set_generation,
        }
    }

    /// Returns the validated TD borrowed for this build transaction.
    pub const fn validated_td(&self) -> &'a Thing {
        self.validated_td
    }

    /// Returns the immutable registration snapshot.
    pub const fn registrations(&self) -> &'a R {
        self.registrations
    }

    /// Returns the generation reserved for the resulting immutable plan set.
    pub const fn plan_set_generation(&self) -> PlanSetGeneration {
        self.plan_set_generation
    }
}

/// Owned immutable output of one completed plan-build transaction.
#[derive(Debug, Eq, PartialEq)]
pub struct PlanBuildOutput<A> {
    logical_plans: Vec<LogicalInteractionPlan>,
    artifacts: Vec<BindingArtifactEnvelope<A>>,
    artifact_refs: Vec<BindingArtifactRef>,
}

impl<A> PlanBuildOutput<A> {
    /// Creates an owned output from its bounded plan-set projections.
    pub fn new(
        logical_plans: Vec<LogicalInteractionPlan>,
        artifacts: Vec<BindingArtifactEnvelope<A>>,
        artifact_refs: Vec<BindingArtifactRef>,
    ) -> Self {
        Self {
            logical_plans,
            artifacts,
            artifact_refs,
        }
    }

    /// Returns the owned immutable logical plans.
    pub fn logical_plans(&self) -> &[LogicalInteractionPlan] {
        &self.logical_plans
    }

    /// Returns the admitted immutable artifact envelopes.
    pub fn artifacts(&self) -> &[BindingArtifactEnvelope<A>] {
        &self.artifacts
    }

    /// Returns the compact plan-to-artifact references.
    pub fn artifact_refs(&self) -> &[BindingArtifactRef] {
        &self.artifact_refs
    }

    /// Consumes the output and returns every owned collection.
    pub fn into_parts(
        self,
    ) -> (
        Vec<LogicalInteractionPlan>,
        Vec<BindingArtifactEnvelope<A>>,
        Vec<BindingArtifactRef>,
    ) {
        (self.logical_plans, self.artifacts, self.artifact_refs)
    }
}

/// Build failure that preserves its caller-owned resumable cursor.
#[derive(Debug, Eq, PartialEq)]
pub struct PlanBuildFailure<C> {
    error: CoreError,
    cursor: C,
}

impl<C> PlanBuildFailure<C> {
    /// Creates an ownership-preserving build failure.
    pub const fn new(error: CoreError, cursor: C) -> Self {
        Self { error, cursor }
    }

    /// Borrows the structured failure.
    pub const fn error(&self) -> &CoreError {
        &self.error
    }

    /// Borrows the returned cursor.
    pub const fn cursor(&self) -> &C {
        &self.cursor
    }

    /// Consumes the failure and returns both owned values.
    pub fn into_parts(self) -> (CoreError, C) {
        (self.error, self.cursor)
    }
}

/// One bounded step of a shared plan compiler.
#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub enum PlanBuildStep<C, A> {
    /// More work remains and the owned cursor is returned.
    Pending(C),
    /// The transaction completed with a TD-lifetime-free output.
    Complete(PlanBuildOutput<A>),
    /// The transaction failed and the owned cursor is returned.
    Failed(PlanBuildFailure<C>),
}

/// Shared incremental planning contract over one registration snapshot type.
pub trait PlanCompiler<R: ?Sized> {
    /// Pure caller-owned resumable build state.
    type Cursor;
    /// Immutable protocol-specific artifact payload.
    type Artifact;

    /// Creates pure build cursor state.
    fn start(&self, input: &PlanBuildInput<'_, R>) -> CoreResult<Self::Cursor>;

    /// Performs bounded deterministic progress.
    fn step(
        &self,
        input: &PlanBuildInput<'_, R>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> PlanBuildStep<Self::Cursor, Self::Artifact>;

    /// Consumes pure in-memory build state.
    fn abort(&self, cursor: Self::Cursor);
}
