//! An isolated normal graph: Planning must request TD's validated-thing itself.
#![no_std]

pub use clinkz_wot_planning::{
    ConsumerPropertyReadBuild, ConsumerPropertyReadBuildConfig, ConsumerPropertyReadBuildStep,
    ConsumerPropertyReadCleanupSlot, ConsumerPropertyReadDraft, ConsumerPropertyReadPreflight,
    InlineConsumerPropertyReadRegistration, ConsumerPropertyReadRollback,
    ConsumerPropertyReadRollbackStep, PlanFootprint, build_consumer_property_read_aggregate,
    select_consumer_property_read_aggregate,
};
