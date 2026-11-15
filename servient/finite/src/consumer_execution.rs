//! Optional borrowed composition of Host Consumer progress. Logical authority,
//! admitted work and native resources keep their existing independent owners.
use crate::{
    consumer::ConsumerCalls, consumer_action::ConsumerActions,
    consumer_notification::ConsumerNotifications,
};
use clinkz_wot_core::notification::ConsumerNotificationBinding;
use clinkz_wot_foundation::{MonotonicInstant, WorkBudget};
use core::task::Context;

/// A capacity's explicit progress turn, independent of Thing cardinality.
/// Implementations retain errors, pending work and results with their original
/// owners. A turn must be bounded/cooperative and must not await another capacity.
/// This interface grants no admission, application-dispatch or native authority.
pub trait ConsumerProgress {
    fn step(&mut self, cx: &mut Context<'_>, now: MonotonicInstant, budget: &mut WorkBudget);

    /// Passive inspection: no admitted work or unread result/report remains in
    /// this capacity. It must not invoke linked callbacks or mutate other work.
    /// A fully retired result retained for a live application is still non-idle.
    fn is_idle(&self) -> bool;
}

/// Run one turn of each supplied capacity with separately supplied work credit.
/// The slice may contain any supported subset, several capacities of one family,
/// or notifications from distinct Bindings. It is borrowed only for this call;
/// no registry, allocation, executor, implicit query/cancel or timer is added.
/// Returns true only when every capacity is idle, including unread reports.
///
/// A deployment must arrange turns on native wakes AND a maintenance source
/// (commands, timer or main-loop turns) covering passive lifecycle/stop changes,
/// deadlines, Ready-to-next-phase transitions and replenished credit. A logical
/// deadline does not install a timer. Zero credit preserves that obligation;
/// this function deliberately does not self-wake into a busy loop. Observe
/// application results in the same driving task/loop: these manual owners do not
/// supply independently awaitable result handles or broadcast task notification.
///
/// Closing/removing the managed Consumer fences its original family work; keep
/// driving the capacities and consuming live handles/reports until retirement.
/// Dropping a capacity remains qualified finite local abort, not a completed drain.
pub fn step_consumers(
    capacities: &mut [&mut dyn ConsumerProgress],
    cx: &mut Context<'_>,
    now: MonotonicInstant,
    mut credit: impl FnMut(usize) -> WorkBudget,
) -> bool {
    for (index, capacity) in capacities.iter_mut().enumerate() {
        capacity.step(cx, now, &mut credit(index));
    }
    capacities.iter().all(|capacity| capacity.is_idle())
}

impl ConsumerProgress for ConsumerCalls {
    fn step(&mut self, cx: &mut Context<'_>, now: MonotonicInstant, budget: &mut WorkBudget) {
        ConsumerCalls::step(self, cx, now, budget);
    }
    fn is_idle(&self) -> bool {
        ConsumerCalls::is_idle(self)
    }
}
impl ConsumerProgress for ConsumerActions {
    fn step(&mut self, cx: &mut Context<'_>, now: MonotonicInstant, budget: &mut WorkBudget) {
        ConsumerActions::step(self, cx, now, budget);
    }
    fn is_idle(&self) -> bool {
        ConsumerActions::is_idle(self)
    }
}
impl<B: ConsumerNotificationBinding + Clone + 'static> ConsumerProgress for ConsumerNotifications<B>
where
    B::Program: Clone + 'static,
    B::Configuration: Clone + 'static,
{
    fn step(&mut self, cx: &mut Context<'_>, _: MonotonicInstant, budget: &mut WorkBudget) {
        ConsumerNotifications::step(self, cx, budget);
    }
    fn is_idle(&self) -> bool {
        self.retained_subscriptions() == 0
    }
}
