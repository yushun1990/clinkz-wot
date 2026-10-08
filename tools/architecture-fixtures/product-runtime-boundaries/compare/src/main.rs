use clinkz_wot_core as old;
use clinkz_wot_foundation::{
    GatewayDefaultV1, Generation, ResourceKind, SlotIndex, StaticResourceProfile,
};
use runtime_contracts_probe as new;
use std::collections::BTreeMap;
#[path = "../../plan_builder.rs"]
mod plan_builder;
mod selection;
fn main() {
    let expected_old_plan = old::PlanId::new(SlotIndex::new(3), Generation::INITIAL);
    let expected_new_plan = new::PlanId::new(SlotIndex::new(3), Generation::INITIAL);
    let limits = GatewayDefaultV1::LIMITS
        .clone()
        .try_with_limit(ResourceKind::AdditionalResponsesPerFormMax, Some(1))
        .unwrap();
    let mut cases = 0;
    let mut constructor_rejections = 0;
    let mut successes = 0;
    // Eight independent shape/identity bits, six opaque native statuses and
    // all three currently constructible normalized success statuses.
    for flags in 0..256u16 {
        for native_status in [0, 100, 200, 418, 599, u16::MAX] {
            for status_index in 0..3 {
                let binding = if flags & 1 != 0 { 18 } else { 17 };
                let generation = if flags & 2 != 0 {
                    Generation::INITIAL.checked_next().unwrap()
                } else {
                    Generation::INITIAL
                };
                let plan_slot = if flags & 4 != 0 { 4 } else { 3 };
                let old_response = if flags & 8 != 0 {
                    old::BindingResponseMetadata::try_additional(
                        old::BindingId::new(binding),
                        old::BindingGeneration::new(generation),
                        old::PlanId::new(SlotIndex::new(plan_slot), Generation::INITIAL),
                        0,
                        native_status,
                        &limits,
                    )
                    .unwrap()
                } else {
                    old::BindingResponseMetadata::primary(
                        old::BindingId::new(binding),
                        old::BindingGeneration::new(generation),
                        old::PlanId::new(SlotIndex::new(plan_slot), Generation::INITIAL),
                        native_status,
                    )
                };
                let new_response = if flags & 8 != 0 {
                    new::BindingResponseMetadata::try_additional(
                        new::BindingId::new(binding),
                        new::BindingGeneration::new(generation),
                        new::PlanId::new(SlotIndex::new(plan_slot), Generation::INITIAL),
                        0,
                        native_status,
                        &limits,
                    )
                    .unwrap()
                } else {
                    new::BindingResponseMetadata::primary(
                        new::BindingId::new(binding),
                        new::BindingGeneration::new(generation),
                        new::PlanId::new(SlotIndex::new(plan_slot), Generation::INITIAL),
                        native_status,
                    )
                };
                let mut om = old::InteractionOutputMetadata::default();
                let mut nm = new::InteractionOutputMetadata::default();
                if flags & 16 == 0 {
                    om = om.with_untrusted_binding_response(old_response);
                    nm = nm.with_untrusted_binding_response(new_response);
                }
                if flags & 64 != 0 {
                    om = om.with_payload_role(old::ResponsePayloadRole::OperationStatus);
                    nm = nm.with_payload_role(new::ResponsePayloadRole::OperationStatus);
                }
                if flags & 128 != 0 {
                    om = om.with_action_invocation(old::ActionInvocationRef::new(
                        SlotIndex::new(4),
                        Generation::INITIAL,
                    ));
                    nm = nm.with_action_invocation(new::ActionInvocationRef::new(
                        SlotIndex::new(4),
                        Generation::INITIAL,
                    ));
                }
                let old_status = [
                    old::InteractionStatus::Ok,
                    old::InteractionStatus::Created,
                    old::InteractionStatus::Accepted,
                ][status_index];
                let new_status = [
                    new::InteractionStatus::Ok,
                    new::InteractionStatus::Created,
                    new::InteractionStatus::Accepted,
                ][status_index];
                let oo = if flags & 32 == 0 {
                    old::InteractionOutput::with_data(old::Payload::new(
                        b"42".to_vec(),
                        "application/json",
                    ))
                } else {
                    old::InteractionOutput::empty()
                };
                let oo = oo.with_status(old_status).try_with_metadata(om);
                assert_eq!(oo.is_none(), flags & (32 | 64) == (32 | 64));
                let Some(oo) = oo else {
                    constructor_rejections += 1;
                    continue;
                };
                let no = new::InteractionOutput::new(
                    if flags & 32 == 0 { Some(b"42") } else { None },
                    new_status,
                    nm,
                );
                let oi = old::BindingArtifactIdentity::new(
                    old::PlanSetGeneration::INITIAL,
                    expected_old_plan,
                    old::BindingId::new(17),
                    old::BindingGeneration::INITIAL,
                    old::BindingConfigurationDigest::new([0; 32]),
                    old::BindingArtifactCompatibility::new([0; 16]),
                    old::BindingArtifactRole::ConsumerCall,
                );
                let request = old::OutboundRequest::property_read(
                    old::BindingArtifactRef::new(oi, SlotIndex::new(0)),
                    BTreeMap::new(),
                    None,
                )
                .unwrap();
                let original = oo.clone();
                let ores = old::validate_untrusted_binding_output(&request, oo);
                let nres = new::validate_property_read_binding_output(
                    new::BindingId::new(17),
                    new::BindingGeneration::INITIAL,
                    expected_new_plan,
                    no,
                );
                // Parity alone can pass a shared bug (even with six successes).
                // Pin which corpus inputs are valid independently of either body.
                assert_eq!(
                    ores.is_ok(),
                    flags == 0 && status_index == 0,
                    "unexpected acceptance: flags={flags} native={native_status} normalized={status_index}"
                );
                assert_eq!(
                    ores.is_ok(),
                    nres.is_ok(),
                    "flags={flags} status={native_status}"
                );
                if let (Err(a), Err(b)) = (&ores, &nres) {
                    assert_eq!(
                        a,
                        &old::CoreError::Validation(
                            old::ErrorContext::new(
                                old::ErrorPhase::Validate,
                                old::RetryClass::Never
                            )
                            .with_operation(clinkz_wot_td::data_type::Operation::ReadProperty)
                            .with_plan(expected_old_plan)
                            .with_binding(old::BindingId::new(17), old::BindingGeneration::INITIAL)
                        )
                    );
                    assert_eq!(
                        b,
                        &new::CoreError::Validation(
                            new::ErrorContext::new(
                                new::ErrorPhase::Validate,
                                new::RetryClass::Never
                            )
                            .with_operation(new::Operation::ReadProperty)
                            .with_plan(expected_new_plan)
                            .with_binding(new::BindingId::new(17), new::BindingGeneration::INITIAL)
                        )
                    );
                }
                if let (Ok(a), Ok(b)) = (&ores, &nres) {
                    assert_eq!(a, &original);
                    assert_eq!(b.data(), Some(&b"42"[..]));
                    assert_eq!(b.status(), new_status);
                    assert_eq!(b.metadata(), &nm);
                }
                successes += usize::from(ores.is_ok());
                cases += 1;
            }
        }
    }
    assert_eq!((cases, constructor_rejections, successes), (3456, 1152, 6));
    let output = plan_builder::build();
    selection::run(&output);
    let plan = &output.logical_plans()[0];
    let image = &static_plan_probe::IMAGE;
    assert_eq!(plan.property_name(), image.property);
    assert_eq!(plan.form_index(), image.form_index);
    assert_eq!(plan.resolved_target(), image.resolved_target);
    assert_eq!(plan.content_type(), image.content_type);
    assert_eq!(
        output.artifacts()[0].artifact().payload().target().unwrap(),
        image.binding_target
    );
    assert_eq!(plan.operation().as_str(), image.operation.as_str());
    assert_eq!(image.form_index, 1);
    assert_eq!(
        image.resolved_target,
        "zenoh+tcp://127.0.0.1:7447/sensor/temperature?unit=C"
    );
    println!("response cases={cases}; identical acceptance and diagnostics");
    println!(
        "unreachable output combinations rejected by existing constructor={constructor_rejections}"
    );
    println!(
        "shared build/runtime leaf plan: {} form={} target={}",
        image.operation.as_str(),
        image.form_index,
        image.resolved_target
    );
    println!(
        "Host sizes (error,borrowed output,image)={:?}",
        static_plan_probe::PROBE_INLINE_SIZES
    );
}
