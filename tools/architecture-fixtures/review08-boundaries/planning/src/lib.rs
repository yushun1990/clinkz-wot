//! Public compiler authoring contracts belong on the PREPARATION side.
#![no_std]
extern crate alloc;
use alloc::rc::Rc;
use review08_core::{Bound, Definition};
use review08_td::{ResolvedInstance, ResolvedRead};

pub enum Step<C, P> {
    Pending(C),
    Ready(P),
}

/// Third-party associated types; no compiler recognition or Copy requirement.
pub trait ReadCompiler {
    type Program: 'static;
    type Operands: 'static;
    type Cursor: 'static;
    fn start(&self, input: &ResolvedRead<'_>) -> Self::Cursor;
    fn step(&self, cursor: Self::Cursor, credits: &mut u8) -> Step<Self::Cursor, Self::Program>;
    fn bind(
        &self,
        program: &Self::Program,
        input: &ResolvedInstance<'_>,
    ) -> Result<Self::Operands, &'static str>;
}

pub fn finish<C: ReadCompiler>(
    input: &ResolvedRead<'_>,
    native: C::Program,
) -> Rc<Definition<C::Program>> {
    Rc::new(Definition {
        operation: input.operation,
        result: input.result,
        native,
    })
}

pub fn bind<C: ReadCompiler>(
    compiler: &C,
    definition: Rc<Definition<C::Program>>,
    input: &ResolvedInstance<'_>,
) -> Result<Bound<C::Program, C::Operands>, &'static str> {
    if definition.operation != input.definition.operation
        || definition.result != input.definition.result
    {
        return Err("different common semantics");
    }
    let operands = compiler.bind(&definition.native, input)?;
    Ok(Bound {
        definition,
        operands,
    })
}
