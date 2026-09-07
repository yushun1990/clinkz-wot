//! Typed, non-wrapping work budgets for bounded progress.

use core::fmt;

const WORK_CLASS_COUNT: usize = 12;

/// A class of incremental engine work with its own counter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum WorkClass {
    /// Parsed JSON values and schema nodes visited.
    JsonSchemaNodes,
    /// Exact codec input bytes consumed.
    CodecInputBytes,
    /// Exact codec output bytes produced.
    CodecOutputBytes,
    /// URI-template source and expanded-output bytes processed.
    UriBytes,
    /// Security expression branches inspected.
    SecurityBranches,
    /// Security or credential providers probed.
    ProviderProbes,
    /// Bounded queue enqueue, dequeue, or overflow operations.
    QueueOperations,
    /// Binding progress calls performed.
    BindingPolls,
    /// Cleanup records or cleanup targets processed.
    CleanupItems,
    /// Bounded handler start, step, cancel, or adapter-poll work.
    HandlerSteps = 9,
    /// Generic typed-document validation and census visits not already
    /// owned by a more specific class. Typed schema-node visits remain
    /// `JsonSchemaNodes`, URI-template bytes remain `UriBytes`, and
    /// security branches remain `SecurityBranches`.
    DocumentNodes = 10,
    /// Aggregate planning enumeration, row construction, lookup sealing,
    /// reconciliation, and reclamation work. Admitted as a discriminant
    /// only; no Planning code charges it yet.
    PlanningItems = 11,
}

impl WorkClass {
    /// Every work class in stable counter order.
    pub const ALL: [Self; WORK_CLASS_COUNT] = [
        Self::JsonSchemaNodes,
        Self::CodecInputBytes,
        Self::CodecOutputBytes,
        Self::UriBytes,
        Self::SecurityBranches,
        Self::ProviderProbes,
        Self::QueueOperations,
        Self::BindingPolls,
        Self::CleanupItems,
        Self::HandlerSteps,
        Self::DocumentNodes,
        Self::PlanningItems,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

/// A bounded set of independent remaining-work counters.
///
/// A newly constructed budget is exhausted in every class. Callers must set
/// each permitted counter explicitly; no constructor interprets omission as
/// unbounded work.
#[derive(Debug, Eq, PartialEq)]
pub struct WorkBudget {
    remaining: [u64; WORK_CLASS_COUNT],
}

impl WorkBudget {
    /// Creates an explicitly exhausted budget.
    pub const fn new() -> Self {
        Self {
            remaining: [0; WORK_CLASS_COUNT],
        }
    }

    /// Returns a budget with one counter replaced by `remaining`.
    #[must_use]
    pub const fn with_remaining(mut self, class: WorkClass, remaining: u64) -> Self {
        self.remaining[class.index()] = remaining;
        self
    }

    /// Replaces one remaining counter.
    pub fn set_remaining(&mut self, class: WorkClass, remaining: u64) {
        self.remaining[class.index()] = remaining;
    }

    /// Returns the remaining units for one work class.
    pub const fn remaining(&self, class: WorkClass) -> u64 {
        self.remaining[class.index()]
    }

    /// Charges `units` before work begins.
    ///
    /// The counter is unchanged when it has insufficient capacity.
    pub fn consume(&mut self, class: WorkClass, units: u64) -> Result<(), BudgetExceeded> {
        let remaining = self.remaining(class);
        let Some(next) = remaining.checked_sub(units) else {
            return Err(BudgetExceeded {
                class,
                requested: units,
                remaining,
            });
        };
        self.remaining[class.index()] = next;
        Ok(())
    }

    /// Returns whether every work class is exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.remaining.iter().all(|remaining| *remaining == 0)
    }
}

impl Default for WorkBudget {
    fn default() -> Self {
        Self::new()
    }
}

/// A failed work-budget charge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetExceeded {
    class: WorkClass,
    requested: u64,
    remaining: u64,
}

impl BudgetExceeded {
    /// Returns the exhausted work class.
    pub const fn class(&self) -> WorkClass {
        self.class
    }

    /// Returns the units requested by the rejected operation.
    pub const fn requested(&self) -> u64 {
        self.requested
    }

    /// Returns the units that remained before the rejected operation.
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }
}

impl fmt::Display for BudgetExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "work budget exhausted for {:?}: requested {}, remaining {}",
            self.class, self.requested, self.remaining
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BudgetExceeded {}

#[cfg(test)]
mod tests {
    use super::{WorkBudget, WorkClass};

    #[test]
    fn charges_are_exact_and_do_not_wrap() {
        let mut budget = WorkBudget::new().with_remaining(WorkClass::CodecInputBytes, 4);
        assert_eq!(budget.consume(WorkClass::CodecInputBytes, 3), Ok(()));
        let error = budget
            .consume(WorkClass::CodecInputBytes, 2)
            .expect_err("the second charge exceeds the remaining byte");
        assert_eq!(error.requested(), 2);
        assert_eq!(error.remaining(), 1);
        assert_eq!(budget.remaining(WorkClass::CodecInputBytes), 1);
    }

    #[test]
    fn zero_charge_is_allowed_on_an_exhausted_counter() {
        let mut budget = WorkBudget::new();
        assert_eq!(budget.consume(WorkClass::CleanupItems, 0), Ok(()));
        assert!(budget.is_exhausted());
    }

    #[test]
    fn handler_steps_are_appended_and_independently_budgeted() {
        assert_eq!(WorkClass::HandlerSteps as u8, 9);
        assert_eq!(WorkClass::ALL[9], WorkClass::HandlerSteps);

        let mut budget = WorkBudget::new().with_remaining(WorkClass::HandlerSteps, 2);
        assert_eq!(budget.consume(WorkClass::HandlerSteps, 1), Ok(()));
        assert_eq!(budget.remaining(WorkClass::HandlerSteps), 1);
        assert_eq!(budget.remaining(WorkClass::CleanupItems), 0);
    }

    #[test]
    fn consumer_document_and_planning_classes_are_append_only() {
        // The pre-existing ten discriminants and the first ten `ALL` entries
        // stay exactly stable; DocumentNodes then PlanningItems are appended.
        assert_eq!(WorkClass::DocumentNodes as u8, 10);
        assert_eq!(WorkClass::PlanningItems as u8, 11);
        assert_eq!(WorkClass::ALL.len(), 12);
        assert_eq!(WorkClass::ALL[10], WorkClass::DocumentNodes);
        assert_eq!(WorkClass::ALL[11], WorkClass::PlanningItems);
        let prefix = [
            WorkClass::JsonSchemaNodes,
            WorkClass::CodecInputBytes,
            WorkClass::CodecOutputBytes,
            WorkClass::UriBytes,
            WorkClass::SecurityBranches,
            WorkClass::ProviderProbes,
            WorkClass::QueueOperations,
            WorkClass::BindingPolls,
            WorkClass::CleanupItems,
            WorkClass::HandlerSteps,
        ];
        for (index, class) in prefix.into_iter().enumerate() {
            assert_eq!(WorkClass::ALL[index], class);
            assert_eq!(class as u8, index as u8);
        }

        // Both appended counters default to zero in a fresh budget.
        let mut budget = WorkBudget::new();
        assert_eq!(budget.remaining(WorkClass::DocumentNodes), 0);
        assert_eq!(budget.remaining(WorkClass::PlanningItems), 0);
        assert!(budget.is_exhausted());
        budget.set_remaining(WorkClass::DocumentNodes, 3);
        assert_eq!(budget.remaining(WorkClass::DocumentNodes), 3);
        assert_eq!(
            budget.consume(WorkClass::DocumentNodes, 4),
            Err(super::BudgetExceeded {
                class: WorkClass::DocumentNodes,
                requested: 4,
                remaining: 3
            })
        );
        assert_eq!(budget.remaining(WorkClass::PlanningItems), 0);
    }
}
