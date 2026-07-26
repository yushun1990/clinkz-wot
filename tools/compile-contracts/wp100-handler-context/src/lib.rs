#![no_std]

use clinkz_wot_core::HandlerContext;

/// Proves that the public context is copyable without adding a lifetime bound.
pub const fn copy_context<'a>(context: HandlerContext<'a>) -> HandlerContext<'a> {
    context
}

/// Proves that the canonical handler-module path remains public.
pub const fn copy_module_context<'a>(
    context: clinkz_wot_core::handler::HandlerContext<'a>,
) -> clinkz_wot_core::handler::HandlerContext<'a> {
    context
}
