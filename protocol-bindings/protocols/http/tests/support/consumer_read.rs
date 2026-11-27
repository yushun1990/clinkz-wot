use clinkz_wot_core::{
    BindingGeneration, BindingId, CoreResult, ThingSlotId, consumer::ConsumerInstance,
    host_accounts::HostResourceAccounts,
};
use clinkz_wot_foundation::{
    GatewayDefaultV1, Generation, SlotIndex, StaticResourceProfile, WorkBudget, WorkClass,
};
use clinkz_wot_planning::open_consumer::{
    ConsumerRegistrationInput, ConsumerRegistry, ConsumerSelector,
};
use clinkz_wot_servient_finite::consumer::StandaloneConsumerAssociation;
use clinkz_wot_td::thing::Thing;
use std::sync::Arc;

pub fn work() -> WorkBudget {
    WorkClass::ALL
        .into_iter()
        .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
}

pub fn http_registration() -> ConsumerRegistrationInput {
    use clinkz_wot_binding_http::{HttpConsumerConfig, try_consumer_registration};
    try_consumer_registration(
        HttpConsumerConfig::for_cleartext_ip().unwrap(),
        BindingId::new(600),
        BindingGeneration::INITIAL,
        600,
    )
    .unwrap()
}

pub fn prepare_consumer(
    thing: &Thing,
    bank: &Arc<HostResourceAccounts>,
    registration: ConsumerRegistrationInput,
    selector: ConsumerSelector,
) -> CoreResult<(ConsumerInstance, StandaloneConsumerAssociation)> {
    let registry = ConsumerRegistry::try_new(vec![registration], bank)
        .map_err(|r| r.error().error().clone())?;
    let prepared = registry.prepare(
        thing,
        GatewayDefaultV1::LIMITS,
        bank.clone(),
        selector,
        &mut work(),
        |job| {
            while !job.is_finished() {
                job.step(&mut work(), false);
            }
        },
    )?;
    let (definitions, funding) = prepared.into_execution();
    let instance = ConsumerInstance::new(ThingSlotId::new(SlotIndex::new(0), Generation::INITIAL));
    let association = StandaloneConsumerAssociation::try_new(
        instance.reference(),
        definitions,
        funding,
        bank.clone(),
    )
    .map_err(|r| r.error().error().clone())?;
    Ok((instance, association))
}
