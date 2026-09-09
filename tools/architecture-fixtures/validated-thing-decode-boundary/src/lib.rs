//! Non-production source-boundary probe, not a ValidatedThing implementation.
extern crate alloc;

use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Trace {
    reads: usize,
    furthest: usize,
}

thread_local! {
    static TRACE: Cell<Trace> = Cell::new(Trace::default());
}

fn record_read(position: usize) {
    TRACE.with(|trace| {
        let mut value = trace.get();
        value.reads += 1;
        value.furthest = value.furthest.max(position + 1);
        trace.set(value);
    });
}

#[allow(dead_code)]
mod observed {
    include!(concat!(env!("OUT_DIR"), "/observed.rs"));
}

#[cfg(test)]
mod probe {
    use super::*;
    use clinkz_wot_foundation::{WorkBudget, WorkClass};
    use clinkz_wot_td::{
        thing::Thing,
        validate::{Validate, ValidationLevel},
    };
    use serde::de::value::{Error, StrDeserializer};

    fn parse(input: &str) -> (Result<time::OffsetDateTime, Error>, Trace) {
        TRACE.with(|trace| trace.set(Trace::default()));
        let value = observed::deserialize(StrDeserializer::<Error>::new(input));
        (value, TRACE.with(Cell::get))
    }

    fn timestamp(digits: usize) -> String {
        format!("2026-09-09T00:00:00.{}Z", "1".repeat(digits))
    }

    #[test]
    fn existing_thing_decode_accepts_long_fraction_and_basic_accepts_the_result() {
        for digits in [1, 9, 10, 4096, 65536] {
            let input = timestamp(digits);
            // Production decoding is the independent semantic oracle. Input
            // construction and Thing ownership are outside the observed call.
            let json = format!(
                r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"clock","security":["none"],"securityDefinitions":{{"none":{{"scheme":"nosec"}}}},"created":"{input}","modified":"{input}"}}"#
            );
            let thing: Thing = serde_json::from_str(&json).unwrap();
            thing.validate_with_level(ValidationLevel::Basic).unwrap();
            let (parsed, trace) = parse(&input);
            let parsed = parsed.unwrap();
            assert_eq!(Some(parsed), thing.created);
            assert_eq!(Some(parsed), thing.modified);
            assert_eq!(trace.furthest, input.len());
            // Exact trace for the current parser: each fractional digit is
            // peeked once by the loop and once by bump; fixed syntax adds 24.
            assert_eq!(trace.reads, 2 * digits + 24);
            println!(
                "fraction={digits} input={} reads={} furthest={} synchronous_calls=1",
                input.len(),
                trace.reads,
                trace.furthest
            );
        }
    }

    #[test]
    fn late_invalid_fraction_is_not_a_valid_prefix() {
        let valid = timestamp(4096);
        let invalid = valid.replace('Z', "xZ");
        let (result, trace) = parse(&invalid);
        assert!(result.is_err());
        assert_eq!(trace.furthest, invalid.len() - 1);
        assert_eq!(trace.reads, 2 * 4096 + 23);
        // Returning the truncated valid prefix would miss this rejection.
        assert!(parse(&timestamp(9)).0.is_ok());
        println!(
            "late-invalid input={} reads={} furthest={}",
            invalid.len(),
            trace.reads,
            trace.furthest
        );
    }

    #[test]
    fn charging_one_node_does_not_bound_shared_parser_byte_work() {
        let input = timestamp(4096);
        let mut budget = WorkBudget::new().with_remaining(WorkClass::DocumentNodes, 1);
        budget.consume(WorkClass::DocumentNodes, 1).unwrap();
        let (result, trace) = parse(&input);
        assert!(result.is_ok());
        assert_eq!(budget.remaining(WorkClass::CodecInputBytes), 0);
        assert_eq!(trace.reads, 8216);
        println!(
            "node-only adapter: byte_budget=0 actual_reads={}",
            trace.reads
        );
    }

    #[test]
    fn atomic_precharge_cannot_turn_the_parser_into_a_resumable_kernel() {
        let input = timestamp(4096);
        let required = (2 * 4096 + 24) as u64;
        TRACE.with(|trace| trace.set(Trace::default()));
        // Even granting the adapter exact foreknowledge of the work cost,
        // repeated small step budgets cannot resume any internal parser state.
        for allowance in [0, 1, 9, 128, required - 1] {
            for _ in 0..3 {
                let mut budget =
                    WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, allowance);
                if budget.consume(WorkClass::CodecInputBytes, required).is_ok() {
                    let _ = parse(&input);
                    panic!("insufficient budget unexpectedly admitted the call");
                }
                assert_eq!(TRACE.with(Cell::get), Trace::default());
                assert_eq!(budget.remaining(WorkClass::CodecInputBytes), allowance);
            }
            println!("atomic adapter: step_allowance={allowance} steps=3 reads=0");
        }
        let mut budget = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, required);
        budget
            .consume(WorkClass::CodecInputBytes, required)
            .unwrap();
        let (result, trace) = parse(&input);
        assert!(result.is_ok());
        assert_eq!(trace.reads as u64, required);
        assert_eq!(budget.remaining(WorkClass::CodecInputBytes), 0);
    }
}
