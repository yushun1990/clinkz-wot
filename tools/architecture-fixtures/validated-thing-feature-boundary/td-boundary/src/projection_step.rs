//! Shared, non-production atomic precharge used by Number and schema probes.
use clinkz_wot_foundation::{WorkBudget, WorkClass};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectionProgress<T> {
    Pending,
    Limit,
    Cancelled,
    Complete(T),
}

pub fn project<T>(
    lexeme: &str,
    ceiling: usize,
    budget: &mut WorkBudget,
    lifetime: &mut u64,
    mut cancelled: impl FnMut() -> bool,
    projection: impl FnOnce() -> T,
) -> ProjectionProgress<T> {
    if lexeme.len() > ceiling {
        return ProjectionProgress::Limit;
    }
    let charge = lexeme.len() as u64;
    if budget.remaining(WorkClass::CodecInputBytes) < charge {
        return ProjectionProgress::Pending;
    }
    if *lifetime < charge {
        return ProjectionProgress::Limit;
    }
    if cancelled() {
        return ProjectionProgress::Cancelled;
    }
    budget.consume(WorkClass::CodecInputBytes, charge).unwrap();
    *lifetime -= charge;
    let result = projection();
    if cancelled() {
        ProjectionProgress::Cancelled
    } else {
        ProjectionProgress::Complete(result)
    }
}
