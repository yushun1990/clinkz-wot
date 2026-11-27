//! Continuing Consumer Actions. Native mappings and opaque continuation backing
//! belong to the selected Binding; neither is logical invocation authority.
use crate::{
    consumer::*, execution::action::ActionReply, host_accounts::HostCapacityReservation, *,
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{
    any::Any,
    task::{Context, Poll},
};

/// Backing limits enforced by the native owner. Together with configuration
/// these delimit what the Host admits for one specific operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumerActionBounds {
    /// Native exchange backing through acknowledged close.
    pub exchange_bytes: u64,
    pub temporary_bytes: u64,
    /// Opaque Invoke continuation. Zero means the operation cannot return one.
    pub continuation_bytes: u64,
    /// Complete owned application result. Zero means the operation cannot return one.
    pub response_bytes: u64,
}

pub type ConsumerActionReply = ActionReply<Box<dyn Any>, ConsumerActionOutput>;

/// Stable result backing lent directly by the Host settlement. A Binding can
/// move an existing output or a byte buffer acquired before the native effect.
pub struct ConsumerActionOutput(ActionOutputBacking);

enum ActionOutputBacking {
    Interaction(InteractionOutput),
    Buffered(Vec<u8>, &'static str),
}

impl From<InteractionOutput> for ConsumerActionOutput {
    fn from(output: InteractionOutput) -> Self {
        Self(ActionOutputBacking::Interaction(output))
    }
}

impl ConsumerActionOutput {
    pub fn buffered(body: Vec<u8>, media: &'static str) -> Self {
        Self(ActionOutputBacking::Buffered(body, media))
    }

    pub fn view(&self) -> InteractionOutputView<'_> {
        static EMPTY: InteractionOutput = InteractionOutput::empty();
        match &self.0 {
            ActionOutputBacking::Interaction(output) => output.view(),
            ActionOutputBacking::Buffered(body, media) => InteractionOutputView::new(
                Some(PayloadView {
                    body,
                    content_type: media,
                    content_coding: None,
                }),
                InteractionStatus::Ok,
                EMPTY.view().metadata(),
            ),
        }
    }

    pub fn backing_bytes(&self) -> Option<usize> {
        match &self.0 {
            ActionOutputBacking::Interaction(output) => interaction_output_backing(output)?
                .checked_sub(core::mem::size_of::<InteractionOutput>())?
                .checked_add(core::mem::size_of::<Self>()),
            ActionOutputBacking::Buffered(body, _) => {
                core::mem::size_of::<Self>().checked_add(body.capacity())
            }
        }
    }
}

/// Owns a finite native exchange, not the continuing invocation. Ready/error
/// never disposes this owner. Close may return an original late observation;
/// pending/error retains every owner and requires further cleanup progress.
/// A poll error does not waive late response validation or retention: the
/// invocation keeps that error independently of an observation from close.
pub trait ConsumerActionExchange {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<CoreResult<ConsumerActionReply>>;
    fn poll_close(&mut self, cx: &mut Context<'_>)
    -> Poll<CoreResult<Option<ConsumerActionReply>>>;
    fn abort_local(self: Box<Self>);
}

/// One immutable, actually compiled native operation. An external Binding can
/// supply this facet without a finite-call adapter or a protocol whitelist.
/// `start` refusal has no native effects; success captures all borrowed input.
/// Controls explicitly check compatibility with the original invoke mapping;
/// different registrations/Bindings are allowed when they implement it.
/// Completed values retain this operation's representation, paired with its
/// captured output data service. Native status/ack envelopes stay Binding-owned;
/// no conversion to the original Invoke representation is required or implied.
/// Metadata and continuation-compatibility inspection are bounded, allocation-
/// free passive operations; execution checks them without entering native I/O.
pub trait ConsumerActionMapping {
    fn identity(&self) -> BindingRegistrationIdentity;
    fn operation(&self) -> Operation;
    fn bounds(&self) -> ConsumerActionBounds;
    fn native_definition(&self) -> &dyn Any;
    /// An incompatible original Invoke declines this pair with `UnsupportedOperation`.
    /// Other errors abort selection. This passive predicate must be stable for
    /// immutable mappings under the effective selection configuration.
    fn supports_continuation(&self, invoke: &dyn ConsumerActionMapping) -> CoreResult<()>;
    /// Pure bounded actual destination calculation, before fresh policy checks.
    /// Start MUST use that same destination; redirects/proxies are unsupported
    /// unless the concrete mapping supplies equivalent policy coverage.
    fn destination(&self, continuation: Option<&dyn Any>) -> CoreResult<Box<str>>;
    /// Secure original exchange, continuation/result and retirement backing
    /// before native effects. Use the same actual destination checked by access.
    fn start(
        &self,
        destination: &str,
        input: Option<&Payload>,
        continuation: Option<&dyn Any>,
    ) -> CoreResult<Box<dyn ConsumerActionExchange>>;
}

pub struct ConsumerActionOperation {
    pub mapping: Box<dyn ConsumerActionMapping>,
    /// Original selected Form target, including any native URI template. Only
    /// the Binding expands it, under the captured destination/security policy.
    pub endpoint: Box<str>,
    pub form: u32,
    pub data: Arc<ConsumerServiceOwner<dyn ConsumerDataService>>,
    /// Original qualification, captured by preparation and never refreshed by
    /// association or execution construction.
    pub body_bytes: u64,
    pub validation_scratch_bytes: u64,
    pub content_type: Box<str>,
    pub identity: BindingRegistrationIdentity,
    pub operation: Operation,
    pub bounds: ConsumerActionBounds,
}
pub struct ConsumerActionDefinitionInput {
    pub target: AffordanceTarget,
    pub schemas: ValueSchemas,
    pub invoke: ConsumerActionOperation,
    pub query: Option<ConsumerActionOperation>,
    pub cancel: Option<ConsumerActionOperation>,
    // Preparation allocation charges. No runtime capacity is acquired here.
    pub funding: Vec<HostCapacityReservation>,
}
/// Reusable source-free definition. Contains no Consumer, permission, vacant
/// invocation slot or native continuation. Endpoints/configuration stay specialized.
pub struct ConsumerActionDefinition(ConsumerActionDefinitionInput);

/// Shared Action meaning, qualified only for application-value roles actually
/// carried by this operation. Native status/acknowledgement envelopes belong to
/// the Binding; CancelAction carries no application input or completed output.
pub fn action_data_supported(
    operation: Operation,
    schemas: &ValueSchemas,
    data: &dyn ConsumerDataService,
) -> bool {
    let supports = |schema: &crate::data_schema::PreparedSchema| {
        data.action_validator().is_some_and(|validator| {
            validator.available()
                && validator.content_type() == data.content_type()
                && validator.body_bytes() as u64 == data.policy().body_bytes
                && validator.supports(schema)
        })
    };
    match operation {
        Operation::InvokeAction => schemas
            .input
            .iter()
            .chain(schemas.output.iter())
            .all(supports),
        Operation::QueryAction => schemas.output.as_ref().is_none_or(supports),
        Operation::CancelAction => true,
        _ => false,
    }
}
impl ConsumerActionDefinition {
    /// Source-free construction checks pairing, not TD support. Planning alone
    /// admits TD operation applicability and complete common/native semantics.
    pub fn try_new(
        input: ConsumerActionDefinitionInput,
    ) -> Result<Self, BindingInputRejection<ConsumerActionDefinitionInput>> {
        let definition = Self(input);
        match definition.check_dependencies() {
            Ok(()) => Ok(definition),
            Err(e) => Err(BindingInputRejection::new(
                definition.0,
                BindingOperationalError::new(e),
            )),
        }
    }
    /// Complete selected-operation qualification for new associations/admission.
    /// Admitted execution checks the dependencies of its original work and the
    /// operation owning its current exchange or unread result.
    pub fn check_dependencies(&self) -> CoreResult<()> {
        let input = &self.0;
        if !matches!(input.target, AffordanceTarget::Action(_))
            || input.target.name().is_none_or(|n| n.len() > 256)
        {
            return Err(invalid());
        }
        for (operation, selected) in [
            (Operation::InvokeAction, Some(&input.invoke)),
            (Operation::QueryAction, input.query.as_ref()),
            (Operation::CancelAction, input.cancel.as_ref()),
        ] {
            let Some(selected) = selected else { continue };
            selected.check_native_dependencies()?;
            if (operation == Operation::InvokeAction && input.schemas.input.is_some())
                || (operation != Operation::CancelAction && input.schemas.output.is_some())
            {
                selected.check_data_dependencies()?;
            }
            if selected.operation != operation
                || !action_data_supported(operation, &input.schemas, selected.data.service())
            {
                return Err(invalid());
            }
            if operation != Operation::InvokeAction {
                selected
                    .mapping
                    .supports_continuation(input.invoke.mapping.as_ref())?;
            }
        }
        Ok(())
    }
    pub fn input(&self) -> &ConsumerActionDefinitionInput {
        &self.0
    }
    pub fn into_input(self) -> ConsumerActionDefinitionInput {
        self.0
    }
}
impl ConsumerActionOperation {
    /// Original native mapping/continuation qualification, independent of a
    /// codec whose application-value role may already have retired.
    pub fn check_native_dependencies(&self) -> CoreResult<()> {
        let b = self.bounds;
        if self.mapping.identity() != self.identity
            || self.mapping.operation() != self.operation
            || self.mapping.bounds() != b
            || b.exchange_bytes == 0
            || self.endpoint.len() > 256
        {
            return Err(invalid());
        }
        Ok(())
    }
    /// Original representation and resource declaration, never a new snapshot.
    pub fn check_data_dependencies(&self) -> CoreResult<()> {
        let policy = self.data.policy();
        if !self.data.available()
            || policy.retained_bytes != self.data.backing_bytes()
            || policy.body_bytes != self.body_bytes
            || policy.temporary_bytes != self.validation_scratch_bytes
            || self.data.content_type() != &*self.content_type
            || !self.data.action_validator().is_some_and(|validator| {
                validator.available()
                    && validator.content_type() == &*self.content_type
                    && validator.body_bytes() as u64 == policy.body_bytes
            })
        {
            return Err(invalid());
        }
        Ok(())
    }
}
fn invalid() -> CoreError {
    CoreError::Validation(ErrorContext::new(ErrorPhase::Validate, RetryClass::Never))
}
