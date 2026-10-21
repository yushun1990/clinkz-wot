use crate::property_read::HostPropertyReadOwner;
use crate::{ExposedThingHandle, ServientError, ServientResult};
use clinkz_wot_core::*;
use clinkz_wot_foundation::{Generation, SlotIndex, WorkBudget};
use clinkz_wot_td::thing::Thing;
/// Host Producer facade. Preparation is explicit; progress remains caller driven.
#[derive(Clone)]
pub struct Servient {
    pub(crate) property_read: Option<HostPropertyReadOwner>,
}
impl Servient {
    pub fn produce_td(&self, td: Thing) -> ServientResult<ExposedThingHandle> {
        let id = td
            .id
            .as_ref()
            .map(|uri| ThingId::from(uri.as_str()))
            .ok_or(ServientError::MissingThingId)?;
        let owner = self.property_read.as_ref().ok_or_else(|| {
            CoreError::UnsupportedOperation(ErrorContext::new(
                ErrorPhase::Admission,
                RetryClass::Never,
            ))
        })?;
        owner.install(td, ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL))?;
        Ok(ExposedThingHandle {
            id,
            owner: owner.clone(),
        })
    }
    pub fn step(
        &self,
        cx: &mut core::task::Context<'_>,
        budget: &mut WorkBudget,
    ) -> StepStatus<()> {
        self.property_read
            .as_ref()
            .map_or(StepStatus::Idle, |owner| owner.step(cx, budget))
    }
}
