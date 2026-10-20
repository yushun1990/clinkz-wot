//! Tiny semantic input stand-ins, NOT a second TD validator/resolver.
#![no_std]
use review08_core::{Operation, ResultShape};

/// Production counterpart: TD-owned paid, scoped resolved semantic loans.
pub struct ResolvedRead<'a> {
    pub operation: Operation,
    pub result: ResultShape,
    pub native_prefix: &'a str,
}

/// An explicit instantiation request, not a complete independent TD or authority.
pub struct ResolvedInstance<'a> {
    pub definition: ResolvedRead<'a>,
    pub suffix: &'a str,
}
