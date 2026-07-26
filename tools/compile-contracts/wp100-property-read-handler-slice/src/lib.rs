#![no_std]

use clinkz_wot_core::{
    CoreResult, HandlerContext, InteractionInput, InteractionOutput, ReadPropertyHandler,
    StaticHandlerRegistration,
};

/// Proves the exact root-level synchronous handler bound and borrowed call.
pub fn call_read_property<H>(
    handler: &H,
    context: HandlerContext<'_>,
    input: &InteractionInput,
) -> CoreResult<InteractionOutput>
where
    H: ReadPropertyHandler + ?Sized,
{
    handler.handle(context, input)
}

/// Proves the canonical handler-module path composes with static registration.
pub fn call_registered_read_property<H>(
    registration: StaticHandlerRegistration<'_, H>,
    context: HandlerContext<'_>,
    input: &InteractionInput,
) -> CoreResult<InteractionOutput>
where
    H: clinkz_wot_core::handler::ReadPropertyHandler,
{
    registration.handler().handle(context, input)
}
