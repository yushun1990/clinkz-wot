use crate::property_read::{HostPropertyReadConfig, HostPropertyReadOwner};
use crate::{Servient, ServientResult};
use clinkz_wot_core::*;
use clinkz_wot_foundation::ResourceLimits;
use clinkz_wot_planning::HostBindingPreparation;
/// Explicit composition of the manually progressed Producer capability.
#[derive(Default)]
pub struct ServientBuilder {
    limits: Option<ResourceLimits>,
    binding: Option<HostBindingPreparation>,
}
impl ServientBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = Some(limits);
        self
    }
    pub fn binding_registration(mut self, binding: HostBindingPreparation) -> Self {
        self.binding = Some(binding);
        self
    }
    pub fn build(self) -> ServientResult<Servient> {
        let owner = match (self.limits, self.binding) {
            (Some(limits), Some(binding)) => Some(HostPropertyReadOwner::new(
                HostPropertyReadConfig::new(limits, binding),
            )),
            (None, Some(_)) => {
                return Err(CoreError::Validation(ErrorContext::new(
                    ErrorPhase::Admission,
                    RetryClass::Never,
                ))
                .into());
            }
            (_, None) => None,
        };
        Ok(Servient {
            property_read: owner,
        })
    }
}
