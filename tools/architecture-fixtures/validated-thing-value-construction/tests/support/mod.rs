extern crate alloc;
use alloc::vec::Vec;
use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use serde_json::Value;
use validated_thing_value_construction_probe::{Cursor, Kind, OwnedValue, Progress, View};

pub(crate) fn budget(n: u64) -> WorkBudget {
    [
        W::DocumentNodes,
        W::CodecInputBytes,
        W::CodecOutputBytes,
        W::CleanupItems,
    ]
    .into_iter()
    .fold(WorkBudget::new(), |budget, class| {
        budget.with_remaining(class, n)
    })
}

// Allocation observations and terminal diagnostics stay inline; boxing this
// fixture record would contaminate the measured interval.
#[allow(clippy::result_large_err)]
pub(crate) fn drive(
    mut cursor: Cursor<'_>,
    n: u64,
) -> Result<OwnedValue, validated_thing_value_construction_probe::Failure> {
    for _ in 0..5_000_000 {
        let before = cursor.trace();
        let lifetime = cursor.lifetime_remaining();
        let mut work = budget(n);
        let progress = cursor.step(&mut work, false);
        let trace = match &progress {
            Progress::Pending(cursor) => cursor.trace(),
            Progress::Complete(value) => value.trace(),
            Progress::Failed(failure) => failure.trace,
        };
        let classes = [
            W::DocumentNodes,
            W::CodecInputBytes,
            W::CodecOutputBytes,
            W::CleanupItems,
        ];
        let spent: u64 = classes
            .iter()
            .enumerate()
            .map(|(i, &class)| {
                let debit = n - work.remaining(class);
                assert_eq!(trace.work[i] - before.work[i], debit);
                debit
            })
            .sum();
        match progress {
            Progress::Pending(next) => {
                assert_eq!(next.lifetime_remaining(), lifetime - spent);
                cursor = next;
            }
            Progress::Complete(value) => return Ok(value),
            Progress::Failed(failure) => {
                assert_eq!(failure.live_after_rollback, 0);
                assert_eq!(failure.allocations, failure.releases);
                return Err(failure);
            }
        }
    }
    panic!("construction did not make bounded progress");
}

pub(crate) fn equivalent(view: View<'_>, value: &Value) {
    match value {
        Value::Null => assert_eq!(view.kind(), Kind::Null),
        Value::Bool(value) => {
            assert_eq!(view.kind(), if *value { Kind::True } else { Kind::False })
        }
        Value::Number(number) => {
            assert_eq!(view.kind(), Kind::Number);
            assert_eq!(view.text(), Some(number.as_str()));
        }
        Value::String(text) => {
            assert_eq!(view.kind(), Kind::String);
            assert_eq!(view.text(), Some(text.as_str()));
        }
        Value::Array(values) => {
            assert_eq!(view.kind(), Kind::Array);
            assert_eq!(view.len(), values.len());
            for (index, value) in values.iter().enumerate() {
                equivalent(view.child(index).unwrap(), value);
            }
        }
        Value::Object(values) => {
            assert_eq!(view.kind(), Kind::Object);
            assert_eq!(view.len(), values.len());
            let mut sorted: Vec<_> = values.iter().collect();
            sorted.sort_by_key(|&(key, _)| key);
            for (index, (key, value)) in sorted.into_iter().enumerate() {
                let (name, child) = view.member(index).unwrap();
                assert_eq!(name, key);
                equivalent(child, value);
            }
        }
    }
}
