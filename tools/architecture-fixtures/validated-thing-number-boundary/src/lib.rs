//! Non-production blocking evidence for the stable typed Number query path.
//! The observed lexical parser is a lower bound, not a replacement as_f64.

use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Trace {
    reads: usize,
    furthest: usize,
}

thread_local! {
    static TRACE: Cell<Trace> = Cell::new(Trace::default());
    static BASE: Cell<usize> = const { Cell::new(0) };
}

fn record(input: &[u8], count: usize) {
    if count == 0 {
        return;
    }
    assert!(count <= input.len());
    let offset = (input.as_ptr() as usize)
        .checked_sub(BASE.with(Cell::get))
        .unwrap();
    TRACE.with(|trace| {
        let mut value = trace.get();
        value.reads += count;
        value.furthest = value.furthest.max(offset + count);
        trace.set(value);
    });
}

fn split_first(input: &[u8]) -> Option<(&u8, &[u8])> {
    record(input, usize::from(!input.is_empty()));
    input.split_first()
}

fn first(input: &[u8]) -> Option<&u8> {
    record(input, usize::from(!input.is_empty()));
    input.first()
}

#[allow(dead_code)]
mod observed {
    pub mod common {
        include!(concat!(env!("OUT_DIR"), "/common.rs"));
    }
    pub mod decimal {
        include!(concat!(env!("OUT_DIR"), "/decimal.rs"));
    }
    pub mod parse {
        include!(concat!(env!("OUT_DIR"), "/parse.rs"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinkz_wot_foundation::{WorkBudget, WorkClass};
    use clinkz_wot_td::{
        data_schema::DataSchema,
        thing::Thing,
        validate::{Validate, ValidateError, ValidationLevel},
    };
    use serde_json::{Number, Value};

    fn lexical(number: &Number) -> (observed::decimal::Decimal, Trace) {
        let bytes = number.as_str().as_bytes();
        BASE.with(|base| base.set(bytes.as_ptr() as usize));
        TRACE.with(|trace| trace.set(Trace::default()));
        let parsed = observed::parse::parse_number(bytes).unwrap();
        (parsed, TRACE.with(Cell::get))
    }

    fn number(zeros: usize, last: char) -> Number {
        // Use only supported parsing, never from_string_unchecked or private
        // Number storage. The exponent remains arbitrarily long after parsing.
        let input = format!("1e+{}{last}", "0".repeat(zeros));
        let number: Number = serde_json::from_str(&input).unwrap();
        assert_eq!(number.as_str(), input);
        number
    }

    fn thing(number: Number) -> Thing {
        let mut thing: Thing = serde_json::from_str(
            r#"{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"numeric probe","security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},"schemaDefinitions":{"probe":{"type":"string"}}}"#,
        ).unwrap();
        let DataSchema::String(schema) = thing
            .schema_definitions
            .as_mut()
            .unwrap()
            .get_mut("probe")
            .unwrap()
        else {
            panic!("fixture shape changed");
        };
        // Existing Basic rules deliberately inspect numeric extension fields
        // on non-numeric schemas as well as dedicated numeric-schema fields.
        schema
            ._context
            ._extra_fields
            .insert("minimum".into(), Value::Number(number));
        schema
            ._context
            ._extra_fields
            .insert("maximum".into(), Value::from(2));
        thing
    }

    #[test]
    fn final_exponent_digit_changes_real_basic_result_after_arbitrarily_long_prefix() {
        for (zeros, expected_reads) in [(1, 8), (9, 24), (4096, 4111), (65536, 65551)] {
            let lower = number(zeros, '0');
            let higher = number(zeros, '1');
            assert_eq!(lower.as_f64(), Some(1.0));
            assert_eq!(higher.as_f64(), Some(10.0));
            let (decimal, trace) = lexical(&lower);
            assert_eq!(decimal.exponent, 0);
            assert_eq!(decimal.mantissa, 1);
            assert_eq!(trace.furthest, lower.as_str().len());
            assert_eq!(trace.reads, expected_reads);
            let (decimal_high, trace_high) = lexical(&higher);
            assert_eq!(decimal_high.exponent, 1);
            assert_eq!(trace_high, trace);
            let accepted = thing(lower);
            let rejected = thing(higher);
            accepted
                .validate_with_level(ValidationLevel::Basic)
                .unwrap();
            assert!(matches!(
                rejected.validate_with_level(ValidationLevel::Basic),
                Err(ValidateError::InvalidSchema(_))
            ));
            println!(
                "zeros={zeros} input={} lexical_reads={} furthest={} explicit_queries_per_number=1 results=1,10 basic=accept,reject",
                zeros + 4,
                trace.reads,
                trace.furthest
            );
        }
    }

    #[test]
    fn schema_node_charge_does_not_charge_the_stable_querys_byte_work() {
        let number = number(4096, '0');
        let mut budget = WorkBudget::new().with_remaining(WorkClass::JsonSchemaNodes, 1);
        budget.consume(WorkClass::JsonSchemaNodes, 1).unwrap();
        // Actual public query followed by an independently observed lexical
        // lower bound from this toolchain's source; these are separate calls.
        assert_eq!(number.as_f64(), Some(1.0));
        let (_, trace) = lexical(&number);
        assert_eq!(budget.remaining(WorkClass::CodecInputBytes), 0);
        assert_eq!(trace.furthest, 4100);
        assert!(trace.reads >= 4100);
        println!(
            "node-only: schema_nodes=1 byte_budget=0 lexical_reads={} furthest={}",
            trace.reads, trace.furthest
        );
    }

    #[test]
    fn atomic_query_precharge_has_no_continuation_for_small_steps() {
        let number = number(4096, '0');
        let (_, trace) = lexical(&number);
        // Give the attempted adapter even a lower-bound cost oracle. Failure
        // to start this indivisible call cannot be repaired by a larger total
        // lifetime allowance or repeated fresh small per-step allowances.
        let lower_bound = number.as_str().len() as u64;
        for allowance in [0, 1, 9, 128, lower_bound - 1] {
            let mut calls = 0;
            for _ in 0..5 {
                let mut budget =
                    WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, allowance);
                if budget
                    .consume(WorkClass::CodecInputBytes, lower_bound)
                    .is_ok()
                {
                    calls += 1;
                    let _ = number.as_f64();
                }
                assert_eq!(budget.remaining(WorkClass::CodecInputBytes), allowance);
            }
            assert_eq!(calls, 0);
            println!(
                "atomic: allowance={allowance} steps=5 queries={calls} lexical_position=0 lower_bound={lower_bound}"
            );
        }
        assert!(trace.reads as u64 >= lower_bound);
    }

    #[test]
    fn lossless_text_capture_does_not_supply_the_basic_numeric_projection() {
        // A resumable Display-to-arena copy can preserve lossless bytes. It
        // cannot alone implement Basic's existing as_f64 projection: this
        // fixture includes rounding, underflow, and finite filtering.
        for (input, expected) in [
            ("9007199254740993.0", Some(9007199254740992.0)),
            ("1e-4000", Some(0.0)),
            ("1e+4000", None),
        ] {
            let number: Number = serde_json::from_str(input).unwrap();
            assert_eq!(number.as_str(), input);
            assert_eq!(number.to_string(), input);
            assert_eq!(number.as_f64(), expected);
        }
    }
}
