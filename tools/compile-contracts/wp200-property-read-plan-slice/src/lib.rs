#![no_std]

extern crate alloc;

use alloc::boxed::Box;

use clinkz_wot_core::{
    BindingArtifact, BindingArtifactCompatibility, BindingArtifactFootprint, BindingCompilerBounds,
    BindingCompilerExtension, BindingCompilerFailure, BindingCompilerInput, BindingCompilerOutput,
    BindingCompilerStep, CoreError, ErrorContext, ErrorPhase, RetryClass,
    StaticBindingCompilerRegistration,
};
use clinkz_wot_foundation::{WorkBudget, WorkClass};

/// Cursor owned by a third-party constrained compiler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThirdPartyCursor {
    remaining_steps: u8,
}

/// Immutable protocol-specific data compiled by the third-party binding.
#[derive(Debug, Eq, PartialEq)]
pub struct ThirdPartyArtifact {
    target: Box<str>,
}

impl ThirdPartyArtifact {
    /// Returns the compiled target without consulting a TD or form.
    pub fn target(&self) -> &str {
        &self.target
    }
}

/// A compiler authored outside the engine workspace.
#[derive(Clone, Copy)]
pub struct ThirdPartyCompiler {
    compatibility: BindingArtifactCompatibility,
}

impl ThirdPartyCompiler {
    /// Creates the compiler with its stable artifact compatibility identity.
    pub const fn new(compatibility: BindingArtifactCompatibility) -> Self {
        Self { compatibility }
    }
}

impl BindingCompilerExtension for ThirdPartyCompiler {
    type Cursor = ThirdPartyCursor;
    type Artifact = ThirdPartyArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn bounds(
        &self,
        input: &BindingCompilerInput<'_>,
    ) -> clinkz_wot_core::CoreResult<BindingCompilerBounds> {
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, input.logical_plan().resolved_target().len() as u64),
            core::mem::size_of::<ThirdPartyCursor>() as u64,
            0,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 2),
        ))
    }

    fn start(
        &self,
        _input: &BindingCompilerInput<'_>,
    ) -> clinkz_wot_core::CoreResult<Self::Cursor> {
        Ok(ThirdPartyCursor { remaining_steps: 2 })
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        mut cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return BindingCompilerStep::Pending(cursor);
        }
        cursor.remaining_steps -= 1;
        if cursor.remaining_steps != 0 {
            return BindingCompilerStep::Pending(cursor);
        }
        let target: Box<str> = input.logical_plan().resolved_target().into();
        let footprint = BindingArtifactFootprint::new(1, target.len() as u64);
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            self.compatibility,
            footprint,
            ThirdPartyArtifact { target },
        )))
    }

    fn abort(&self, _cursor: Self::Cursor) {}
}

/// A second concrete compiler proves application-level heterogeneity.
#[derive(Clone, Copy)]
pub struct FixedCompiler {
    compatibility: BindingArtifactCompatibility,
}

/// Cursor for the second concrete compiler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedCursor(u16);

/// Artifact for the second concrete compiler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedArtifact(u16);

impl FixedArtifact {
    /// Creates the fixture artifact from its compact representation.
    pub const fn from_raw(value: u16) -> Self {
        Self(value)
    }
}

impl FixedCompiler {
    /// Creates the fixed compiler.
    pub const fn new(compatibility: BindingArtifactCompatibility) -> Self {
        Self { compatibility }
    }
}

impl BindingCompilerExtension for FixedCompiler {
    type Cursor = FixedCursor;
    type Artifact = FixedArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        self.compatibility
    }

    fn bounds(
        &self,
        _input: &BindingCompilerInput<'_>,
    ) -> clinkz_wot_core::CoreResult<BindingCompilerBounds> {
        Ok(BindingCompilerBounds::new(
            BindingArtifactFootprint::new(1, 2),
            core::mem::size_of::<FixedCursor>() as u64,
            0,
            WorkBudget::new().with_remaining(WorkClass::BindingPolls, 1),
        ))
    }

    fn start(
        &self,
        _input: &BindingCompilerInput<'_>,
    ) -> clinkz_wot_core::CoreResult<Self::Cursor> {
        Ok(FixedCursor(7))
    }

    fn step(
        &self,
        _input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        if budget.consume(WorkClass::BindingPolls, 1).is_err() {
            return BindingCompilerStep::Pending(cursor);
        }
        BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
            self.compatibility,
            BindingArtifactFootprint::new(1, 2),
            FixedArtifact(cursor.0),
        )))
    }

    fn abort(&self, _cursor: Self::Cursor) {}
}

/// Application-owned closed compiler universe.
pub enum AppCompiler {
    /// Third-party binding variant.
    ThirdParty(ThirdPartyCompiler),
    /// A different binding variant.
    Fixed(FixedCompiler),
}

/// Application-owned closed cursor universe.
#[derive(Debug, Eq, PartialEq)]
pub enum AppCursor {
    /// Third-party cursor variant.
    ThirdParty(ThirdPartyCursor),
    /// Fixed cursor variant.
    Fixed(FixedCursor),
}

/// Application-owned closed artifact universe.
#[derive(Debug, Eq, PartialEq)]
pub enum AppArtifact {
    /// Third-party artifact variant.
    ThirdParty(ThirdPartyArtifact),
    /// Fixed artifact variant.
    Fixed(FixedArtifact),
}

impl BindingCompilerExtension for AppCompiler {
    type Cursor = AppCursor;
    type Artifact = AppArtifact;

    fn compatibility(&self) -> BindingArtifactCompatibility {
        match self {
            Self::ThirdParty(compiler) => compiler.compatibility(),
            Self::Fixed(compiler) => compiler.compatibility(),
        }
    }

    fn bounds(
        &self,
        input: &BindingCompilerInput<'_>,
    ) -> clinkz_wot_core::CoreResult<BindingCompilerBounds> {
        match self {
            Self::ThirdParty(compiler) => compiler.bounds(input),
            Self::Fixed(compiler) => compiler.bounds(input),
        }
    }

    fn start(&self, input: &BindingCompilerInput<'_>) -> clinkz_wot_core::CoreResult<Self::Cursor> {
        match self {
            Self::ThirdParty(compiler) => compiler.start(input).map(AppCursor::ThirdParty),
            Self::Fixed(compiler) => compiler.start(input).map(AppCursor::Fixed),
        }
    }

    fn step(
        &self,
        input: &BindingCompilerInput<'_>,
        cursor: Self::Cursor,
        budget: &mut WorkBudget,
    ) -> BindingCompilerStep<Self::Cursor, Self::Artifact> {
        match (self, cursor) {
            (Self::ThirdParty(compiler), AppCursor::ThirdParty(cursor)) => {
                map_third_party(compiler.step(input, cursor, budget))
            }
            (Self::Fixed(compiler), AppCursor::Fixed(cursor)) => {
                map_fixed(compiler.step(input, cursor, budget))
            }
            (_, cursor) => BindingCompilerStep::Failed(BindingCompilerFailure::new(
                compiler_cursor_mismatch(),
                cursor,
            )),
        }
    }

    fn abort(&self, cursor: Self::Cursor) {
        match (self, cursor) {
            (Self::ThirdParty(compiler), AppCursor::ThirdParty(cursor)) => {
                compiler.abort(cursor);
            }
            (Self::Fixed(compiler), AppCursor::Fixed(cursor)) => compiler.abort(cursor),
            _ => {}
        }
    }
}

/// Constructs a heterogeneous application-static compiler table.
pub fn static_compiler_table(
    third_party: BindingArtifactCompatibility,
    fixed: BindingArtifactCompatibility,
) -> [StaticBindingCompilerRegistration<AppCompiler>; 2] {
    [
        StaticBindingCompilerRegistration::new(AppCompiler::ThirdParty(ThirdPartyCompiler::new(
            third_party,
        ))),
        StaticBindingCompilerRegistration::new(AppCompiler::Fixed(FixedCompiler::new(fixed))),
    ]
}

fn map_third_party(
    step: BindingCompilerStep<ThirdPartyCursor, ThirdPartyArtifact>,
) -> BindingCompilerStep<AppCursor, AppArtifact> {
    match step {
        BindingCompilerStep::Pending(cursor) => {
            BindingCompilerStep::Pending(AppCursor::ThirdParty(cursor))
        }
        BindingCompilerStep::Complete(output) => {
            let (compatibility, footprint, payload) = output.into_artifact().into_parts();
            BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                compatibility,
                footprint,
                AppArtifact::ThirdParty(payload),
            )))
        }
        BindingCompilerStep::Failed(failure) => {
            let (error, cursor) = failure.into_parts();
            BindingCompilerStep::Failed(BindingCompilerFailure::new(
                error,
                AppCursor::ThirdParty(cursor),
            ))
        }
    }
}

fn map_fixed(
    step: BindingCompilerStep<FixedCursor, FixedArtifact>,
) -> BindingCompilerStep<AppCursor, AppArtifact> {
    match step {
        BindingCompilerStep::Pending(cursor) => {
            BindingCompilerStep::Pending(AppCursor::Fixed(cursor))
        }
        BindingCompilerStep::Complete(output) => {
            let (compatibility, footprint, payload) = output.into_artifact().into_parts();
            BindingCompilerStep::Complete(BindingCompilerOutput::new(BindingArtifact::new(
                compatibility,
                footprint,
                AppArtifact::Fixed(payload),
            )))
        }
        BindingCompilerStep::Failed(failure) => {
            let (error, cursor) = failure.into_parts();
            BindingCompilerStep::Failed(BindingCompilerFailure::new(
                error,
                AppCursor::Fixed(cursor),
            ))
        }
    }
}

fn compiler_cursor_mismatch() -> CoreError {
    CoreError::InternalInvariant(ErrorContext::new(ErrorPhase::Admission, RetryClass::Never))
}
