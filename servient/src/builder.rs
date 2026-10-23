use crate::{Servient, host_accounts::HostResourceAccounts};
use alloc::sync::Arc;
use clinkz_wot_core::{producer::*, *};
use clinkz_wot_foundation::ResourceLimits;
use clinkz_wot_planning::producer_execution::PreparedProducer;

/// Explicit source-free Host composition. Pass the same original account bank
/// to Consumer and Producer roots to enforce their shared physical capacity.
pub struct ServientBuilder {
    limits: ResourceLimits,
    parent: Arc<HostResourceAccounts>,
}
impl ServientBuilder {
    pub fn new(limits: ResourceLimits, parent: Arc<HostResourceAccounts>) -> Self {
        Self { limits, parent }
    }
    /// Preparation already checked the entire advertised draft and exact
    /// application handler. No TD, compiler or public setup stages survive.
    #[allow(clippy::type_complexity)]
    pub fn produce<E: ProducerExecutor, H: ReadPropertyHandler + Send + 'static>(
        self,
        prepared: PreparedProducer<E, H>,
        authority: ProducerInstanceAuthority,
        deadline: Deadline,
    ) -> Result<
        Servient<E, H>,
        BindingInputRejection<(
            PreparedProducer<E, H>,
            ProducerInstanceAuthority,
            ResourceLimits,
            Arc<HostResourceAccounts>,
        )>,
    > {
        Servient::try_new(prepared, authority, self.limits, self.parent, deadline)
    }
}
