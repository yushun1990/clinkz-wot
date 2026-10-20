//! Sketch execution contracts. No TD, compiler, protocol, executor or handler lookup.
#![no_std]
extern crate alloc;

use alloc::{boxed::Box, rc::Rc};
use core::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    ReadProperty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultShape {
    Boolean,
}

/// Data, never an installation/validation certificate. Native P is binding-owned.
pub struct Definition<P> {
    pub operation: Operation,
    pub result: ResultShape,
    pub native: P,
}

/// Explicit sharing, with independently owned operands. No live authority.
pub struct Bound<P, O> {
    pub definition: Rc<Definition<P>>,
    pub operands: O,
}

/// Engine treats a binding's canonical native selector as an opaque key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Endpoint(pub Box<str>);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteId {
    pub slot: u32,
    pub generation: u32,
}

/// Constructor creates a NEW authority; it cannot impersonate an existing root.
#[derive(Default)]
pub struct Authority(Rc<()>);

#[derive(Clone)]
pub struct Gate {
    issuer: Rc<()>,
    route: RouteId,
    serving: Rc<Cell<bool>>,
}

/// Non-Clone, non-retained, bound to the real issuer AND exact route generation.
///
/// ```compile_fail
/// use review08_core::*;
/// let authority = Authority::default();
/// let gate = authority.gate(RouteId { slot: 1, generation: 1 });
/// authority.set_serving(&gate, true);
/// let retained = authority.with_permit(&gate, |permit| permit);
/// ```
///
/// ```compile_fail
/// use review08_core::*;
/// let authority = Authority::default();
/// let gate = authority.gate(RouteId { slot: 1, generation: 1 });
/// let forged = AcceptPermit { gate: &gate };
/// ```
pub struct AcceptPermit<'a> {
    gate: &'a Gate,
}

impl Authority {
    pub fn gate(&self, route: RouteId) -> Gate {
        Gate {
            issuer: self.0.clone(),
            route,
            serving: Rc::new(Cell::new(false)),
        }
    }
    pub fn set_serving(&self, gate: &Gate, serving: bool) {
        assert!(Rc::ptr_eq(&self.0, &gate.issuer));
        gate.serving.set(serving);
    }
    pub fn with_permit<T>(
        &self,
        gate: &Gate,
        f: impl for<'p> FnOnce(AcceptPermit<'p>) -> T,
    ) -> Option<T> {
        (Rc::ptr_eq(&self.0, &gate.issuer) && gate.serving.get()).then(|| f(AcceptPermit { gate }))
    }
}

impl AcceptPermit<'_> {
    pub fn authorizes(self, target: &Gate) -> bool {
        Rc::ptr_eq(&self.gate.issuer, &target.issuer)
            && self.gate.route == target.route
            && Rc::ptr_eq(&self.gate.serving, &target.serving)
            && self.gate.serving.get()
            && target.serving.get()
    }
}

pub trait ObservedRead {
    fn endpoint(&self) -> &Endpoint;
}

/// A deliberately narrow one-operation sketch, not a proposed universal SPI.
pub trait ReadRoute {
    type Observation: ObservedRead;
    type Delivery: ReadDelivery;
    fn endpoint(&self) -> &Endpoint;
    fn install(&mut self, gate: Gate) -> Result<(), &'static str>;
    fn accept(
        &mut self,
        permit: AcceptPermit<'_>,
        input: Self::Observation,
    ) -> Result<Self::Delivery, Self::Observation>;
    fn close(&mut self);
}

pub trait ReadDelivery {
    /// Pending/error retains the complete delivery in the caller.
    fn poll_reply(&mut self, result: bool) -> Result<bool, &'static str>;
}
