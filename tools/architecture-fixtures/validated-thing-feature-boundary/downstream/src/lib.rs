#![no_std]

use number_boundary_td_prototype::synchronous_decimal;
use serde_json::Number;

/// One exact predicate to exercise a shared semantic consumer, not a full
/// five-predicate Basic kernel. Exponent size does not change positivity.
#[derive(Default)]
pub struct Positivity {
    negative: bool,
    exponent: bool,
    nonzero: bool,
}
impl Positivity {
    pub fn byte(&mut self, byte: u8) {
        match byte {
            b'-' if !self.exponent => self.negative = true,
            b'e' | b'E' => self.exponent = true,
            b'1'..=b'9' if !self.exponent => self.nonzero = true,
            _ => (),
        }
    }
    pub fn positive(&self) -> bool {
        self.nonzero && !self.negative
    }
}
pub fn synchronous_positive(number: &Number) -> bool {
    let mut kernel = Positivity::default();
    synchronous_decimal(number, |b| kernel.byte(b)).unwrap();
    kernel.positive()
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use alloc::{format, string::ToString, vec::Vec};
    use clinkz_wot_td::{
        data_schema::DataSchema,
        thing::Thing,
        validate::{Validate, ValidationLevel},
    };

    fn corpus() -> Vec<(Number, bool)> {
        let mut values = Vec::new();
        for text in [
            "0",
            "-0",
            "-0.0",
            "1.00",
            "10e-1",
            "-1.25",
            "9007199254740993.0000000000000000000001",
            "5e-324",
            "18446744073709551615",
        ] {
            values.push((text.parse().unwrap(), !text.starts_with('-') && text != "0"));
        }
        // Use only public parsing and Numbers constructible in this graph.
        #[cfg(any(feature = "ap", feature = "validated-thing"))]
        for text in [
            format!("1e+{}1", "0".repeat(65_536)),
            format!("1e-{}1", "9".repeat(4096)),
            format!("1{}e-4096", "0".repeat(4096)),
            format!("0.{}1e4097", "0".repeat(4096)),
            format!("-0e+{}1", "9".repeat(4096)),
        ] {
            values.push((text.parse().unwrap(), !text.starts_with('-')));
        }
        values
    }

    #[test]
    fn capability_is_not_inferred_from_dependency_features() {
        assert_eq!(
            number_boundary_td_prototype::LOCAL_VALIDATED_THING,
            cfg!(feature = "validated-thing")
        );
        let number: Number = "9007199254740993.0000000000000000000001".parse().unwrap();
        #[cfg(any(feature = "ap", feature = "validated-thing"))]
        assert_eq!(number.as_str(), "9007199254740993.0000000000000000000001");
        #[cfg(not(any(feature = "ap", feature = "validated-thing")))]
        assert_ne!(
            number.to_string(),
            "9007199254740993.0000000000000000000001"
        );
    }

    #[test]
    fn sync_source_and_positivity_work_in_every_graph() {
        for (number, positive) in corpus() {
            let mut emitted = Vec::new(); // assertion storage outside adapter
            synchronous_decimal(&number, |b| emitted.push(b)).unwrap();
            assert_eq!(emitted, number.to_string().as_bytes());
            #[cfg(any(feature = "ap", feature = "validated-thing"))]
            assert_eq!(emitted, number.as_str().as_bytes());
            assert_eq!(synchronous_positive(&number), positive);
        }
    }

    #[test]
    fn current_td_basic_exposes_the_required_base_semantic_delta() {
        // All four bound pairings apply even to non-numeric schema variants.
        // Base Numbers can retain distinct integers which as_f64 collapses.
        for lower in ["minimum", "exclusiveMinimum"] {
            for upper in ["maximum", "exclusiveMaximum"] {
                let input = format!(
                    r#"{{"@context":"https://www.w3.org/2022/wot/td/v1.1","title":"probe","security":["none"],"securityDefinitions":{{"none":{{"scheme":"nosec"}}}},"schemaDefinitions":{{"probe":{{"type":"string","{lower}":9007199254740993,"{upper}":9007199254740992}}}}}}"#
                );
                let thing: Thing = serde_json::from_str(&input).unwrap();
                thing.validate_with_level(ValidationLevel::Basic).unwrap();
                let DataSchema::String(schema) =
                    &thing.schema_definitions.as_ref().unwrap()["probe"]
                else {
                    panic!()
                };
                let a = schema._context._extra_fields[lower].as_number().unwrap();
                let b = schema._context._extra_fields[upper].as_number().unwrap();
                // Independent integer oracle for this witness only.
                assert!(a.as_u64().unwrap() > b.as_u64().unwrap());
                assert_eq!(a.to_string(), "9007199254740993");
                assert_eq!(b.to_string(), "9007199254740992");
            }
        }
        let tiny: Number = "1e-4000".parse().unwrap();
        #[cfg(any(feature = "ap", feature = "validated-thing"))]
        assert!(synchronous_positive(&tiny));
        #[cfg(not(any(feature = "ap", feature = "validated-thing")))]
        assert!(!synchronous_positive(&tiny)); // already rounded typed zero
    }

    #[cfg(feature = "validated-thing")]
    #[test]
    fn charged_source_matches_sync_and_keeps_terminal_first_cause() {
        use clinkz_wot_foundation::{WorkBudget, WorkClass};
        use number_boundary_td_prototype::bounded::{Progress, Scan, decimal};
        fn budget(n: u64) -> WorkBudget {
            WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, n)
        }
        for (number, positive) in corpus() {
            let source = decimal(&number);
            assert_eq!(source.as_ptr(), number.as_str().as_ptr());
            for size in [1, 7, 128] {
                let mut scan = Scan::new(&number, source.len() as u64);
                let mut emitted = Vec::new();
                let mut kernel = Positivity::default();
                loop {
                    let before = (scan.position, scan.lifetime);
                    assert_eq!(
                        scan.step(&mut budget(0), false, |_| panic!()),
                        Progress::Pending
                    );
                    assert_eq!(before, (scan.position, scan.lifetime));
                    let mut allowance = budget(size);
                    let outcome = scan.step(&mut allowance, false, |b| {
                        emitted.push(b);
                        kernel.byte(b);
                    });
                    assert_eq!(
                        scan.position - before.0,
                        (size - allowance.remaining(WorkClass::CodecInputBytes)) as usize
                    );
                    if outcome == Progress::Complete {
                        break;
                    }
                    assert_eq!(outcome, Progress::Pending);
                }
                assert_eq!(emitted, source.as_bytes());
                assert_eq!(scan.lifetime, 0);
                assert_eq!(kernel.positive(), positive);
                assert_eq!(
                    scan.step(&mut budget(1), true, |_| panic!()),
                    Progress::Complete
                );
            }
            let mut scan = Scan::new(&number, 0);
            assert_eq!(
                scan.step(&mut budget(1), false, |_| panic!()),
                Progress::Limit
            );
            assert_eq!(
                scan.step(&mut budget(1), true, |_| panic!()),
                Progress::Limit
            );
            let mut scan = Scan::new(&number, 1);
            assert_eq!(
                scan.step(&mut budget(0), true, |_| panic!()),
                Progress::Cancelled
            );
            assert_eq!(
                scan.step(&mut budget(128), false, |_| panic!()),
                Progress::Cancelled
            );
        }
    }
}
