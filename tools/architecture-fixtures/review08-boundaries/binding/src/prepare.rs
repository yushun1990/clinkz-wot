use crate::{Counts, MappingPolicy, Operands, Program};
use review08_core::Endpoint;
use review08_planning::{ReadCompiler, Step};
use review08_td::{ResolvedInstance, ResolvedRead};
use std::rc::Rc;

pub struct Compiler(pub Rc<Counts>, pub MappingPolicy);
pub struct Cursor {
    prefix: Box<str>,
    counts: Rc<Counts>,
    policy: MappingPolicy,
    advanced: bool,
}

impl ReadCompiler for Compiler {
    type Program = Program;
    type Operands = Operands;
    type Cursor = Cursor;
    fn start(&self, input: &ResolvedRead<'_>) -> Cursor {
        self.0.starts.set(self.0.starts.get() + 1);
        Cursor {
            prefix: input.native_prefix.into(),
            counts: self.0.clone(),
            policy: self.1,
            advanced: false,
        }
    }
    fn step(&self, mut cursor: Cursor, credits: &mut u8) -> Step<Cursor, Program> {
        if *credits == 0 {
            return Step::Pending(cursor);
        }
        *credits -= 1;
        if !cursor.advanced {
            cursor.advanced = true;
            return Step::Pending(cursor);
        }
        cursor.counts.compiles.set(cursor.counts.compiles.get() + 1);
        Step::Ready(Program {
            prefix: cursor.prefix,
            counts: cursor.counts,
            policy: cursor.policy,
        })
    }
    fn bind(
        &self,
        program: &Program,
        input: &ResolvedInstance<'_>,
    ) -> Result<Operands, &'static str> {
        // Actual dependencies, not a source id/model/config digest comparison.
        if program.prefix.as_ref() != input.definition.native_prefix || program.policy != self.1 {
            return Err("native dependency changed");
        }
        // An explicit native operand constraint; this is not a TD Property key.
        if input.suffix.is_empty()
            || input.suffix.len() > 8
            || !input.suffix.bytes().all(|c| c.is_ascii_alphanumeric())
        {
            return Err("invalid native operand");
        }
        let key = format!("{}{}", program.prefix, input.suffix);
        if key.len() > 32 {
            return Err("native key too long");
        }
        Ok(Operands {
            endpoint: Endpoint(key.into_boxed_str()),
        })
    }
}
