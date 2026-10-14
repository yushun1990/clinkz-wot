use clinkz_wot_consumer_property_read_binding_fixture::{Clock, StaticClient, Stats};
use clinkz_wot_core::{InteractionOutput, Payload};
use std::sync::Arc;
#[test]
fn downstream_application_destroys_input_then_uses_two_real_compiled_coordinates() {
    let thing = serde_json::from_str(r#"{
      "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"external","id":"urn:external",
      "security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},
      "properties":{"a-read":{"forms":[{"href":"https://peer/skip","op":[]},{"href":"https://peer/first"},{"href":"https://peer/second"}]},
      "c-温度":{"forms":[{"href":"https://peer/skip","op":["writeproperty"]},{"href":"https://peer/last"}]}} }"#).unwrap();
    let outputs = core::array::from_fn(|_| {
        Some(InteractionOutput::with_data(Payload::new(
            &b"17"[..],
            "application/json",
        )))
    });
    let result = clinkz_wot_consumer_property_read_runtime_runner::application(
        thing,
        StaticClient::new(Arc::new(Stats::default()), outputs),
        &Clock::default(),
    )
    .unwrap();
    assert_eq!(result, [599, 599]);
}

#[test]
#[cfg(feature = "std")]
fn downstream_host_application_destroys_input_then_uses_two_real_compiled_coordinates() {
    let thing = serde_json::from_str(r#"{
      "@context":"https://www.w3.org/2022/wot/td/v1.1","title":"external","id":"urn:external",
      "security":["none"],"securityDefinitions":{"none":{"scheme":"nosec"}},
      "properties":{"a-read":{"forms":[{"href":"https://peer/skip","op":[]},{"href":"https://peer/first"},{"href":"https://peer/second"}]},
      "c-温度":{"forms":[{"href":"https://peer/skip","op":["writeproperty"]},{"href":"https://peer/last"}]}} }"#).unwrap();
    let outputs = core::array::from_fn(|_| {
        Some(InteractionOutput::with_data(Payload::new(
            &b"17"[..],
            "application/json",
        )))
    });
    let result = clinkz_wot_consumer_property_read_runtime_runner::host_application(
        thing,
        clinkz_wot_consumer_property_read_binding_fixture::HostClient::new(
            Arc::new(Stats::default()),
            outputs,
        ),
        Arc::new(Clock::default()),
    )
    .unwrap();
    assert_eq!(result, [599, 599]);
}
