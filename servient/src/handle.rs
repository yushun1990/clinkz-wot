use crate::property_read::HostPropertyReadOwner;
use alloc::boxed::Box;
use clinkz_wot_core::*;
/// A handle to the admitted manually progressed Producer generation.
pub struct ExposedThingHandle {
    pub(crate) id: ThingId,
    pub(crate) owner: HostPropertyReadOwner,
}
impl ExposedThingHandle {
    pub fn id(&self) -> &ThingId {
        &self.id
    }
    pub fn set_read_property_handler<H>(
        &self,
        name: impl Into<Box<str>>,
        handler: H,
        footprint: HandlerFootprint,
    ) -> CoreResult<()>
    where
        H: ReadPropertyHandler + Send + Sync + 'static,
    {
        self.owner
            .set_read_property_handler(name, handler, footprint)
    }
    pub fn begin_expose(&self) -> CoreResult<()> {
        self.owner.begin_expose()
    }
    pub fn begin_destroy(&self) -> CoreResult<()> {
        self.owner.begin_destroy()
    }
}
