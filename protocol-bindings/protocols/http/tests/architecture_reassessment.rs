#![cfg(feature = "prepare")]
//! Non-production diagnostics retained from Architecture Review 08.
//!
//! These assert reviewed limitations or narrow evidence boundaries, not desired
//! general behavior. Invert/retire the relevant assertion when an independently
//! accepted correction lands; do not preserve a defect to keep this test green.
//! Consumer admission below performs no protocol call or wire I/O. The result
//! kernel probe covers identity/shape only, not installed execution or schemas.
//! See docs/reviews/review-08-integrated-architecture-reassessment.md.

#[cfg(test)]
mod tests {
    use clinkz_wot_binding_http::{HttpConsumerConfig, try_consumer_registration};
    use clinkz_wot_core::*;
    use clinkz_wot_foundation::*;
    use clinkz_wot_servient::consumer_property_read::*;
    use clinkz_wot_td::{thing::Thing, validate::Validate};
    use serde_json::{Value, json};
    use std::{
        num::NonZeroU64,
        sync::Arc,
        task::{Context, Waker},
    };

    fn td() -> Value {
        json!({"@context":["https://www.w3.org/2022/wot/td/v1.1", {"ex":"https://example.org/"}],
            "title":"independent audit", "id":"urn:audit",
            "securityDefinitions":{"none":{"scheme":"nosec"}}, "security":["none"],
            "properties":{"a":{"type":"number", "readOnly":true,
                "forms":[{"href":"http://127.0.0.1:9/a"}]}}})
    }
    fn work() -> WorkBudget {
        WorkClass::ALL
            .into_iter()
            .fold(WorkBudget::new(), |b, w| b.with_remaining(w, 1_000_000))
    }
    struct Clock;
    impl RuntimeClock for Clock {
        fn now(&self) -> MonotonicInstant {
            MonotonicInstant::new(ClockId::new(800), 0)
        }
        fn ticks_per_second(&self) -> NonZeroU64 {
            NonZeroU64::new(1000).unwrap()
        }
    }
    fn consume(value: Value) -> bool {
        let thing: Thing = serde_json::from_value(value).unwrap();
        thing.validate().expect("ordinary TD Basic validation");
        let policy = ConsumerPropertyReadPolicy::try_new_owned(
            BindingResourceContract::ConsumerOwnedHostV2,
            ResourceProfileId::GATEWAY_DEFAULT_V1,
            GatewayDefaultV1::LIMITS.clone(),
            BindingLifetimeFootprint::new(8, 8192),
            BindingLifetimeFootprint::new(8, 4096),
            [1, 64, 64, 64],
            [4096, 16],
            [2, 2, 1, 1, 1, 1, 1],
            [30_000, 1000],
            65536,
        )
        .unwrap();
        let parent = Arc::new(
            clinkz_wot_servient::host_accounts::HostResourceAccounts::try_from_accounts(
                policy.parent_accounts(),
            )
            .unwrap(),
        );
        let registration = try_consumer_registration(
            HttpConsumerConfig::for_cleartext_ip().unwrap(),
            BindingId::new(800),
            BindingGeneration::INITIAL,
            800,
        )
        .unwrap();
        let (runtime, observer) = ConsumerPropertyReadRuntime::try_new(
            registration,
            policy,
            Arc::new(Clock),
            parent.clone(),
        )
        .unwrap_or_else(|_| panic!("runtime construction"));
        let result = runtime.consume(&thing, &mut work(), |b| {
            for _ in 0..300_000 {
                b.step(&mut work());
                if b.is_finished() {
                    return;
                }
            }
            panic!("build stalled")
        });
        let supported = match result {
            Ok(StartStatus::Ready(handle)) => {
                drop(handle);
                true
            }
            Err(e) => {
                println!("Basic-valid input rejected: {e:?}");
                false
            }
            _ => panic!("unexpected pending publication"),
        };
        drop(thing);
        let mut done = false;
        for _ in 0..10_000 {
            if let StartStatus::Ready(outcome) =
                runtime.shutdown(&mut Context::from_waker(Waker::noop()), &mut work())
            {
                assert!(outcome.is_complete());
                done = true;
                break;
            }
        }
        assert!(done);
        drop(runtime);
        drop(observer);
        assert!(
            parent
                .snapshot(ConsumerPropertyReadPolicy::PARENT_KINDS)
                .iter()
                .all(|a| a.used() == 0)
        );
        supported
    }

    #[test]
    fn consumer_still_requires_source_id_and_nonempty_property_key() {
        assert!(consume(td()));
        let mut absent = td();
        absent.as_object_mut().unwrap().remove("id");
        assert!(!consume(absent));
        let mut empty = td();
        let p = empty["properties"]
            .as_object_mut()
            .unwrap()
            .remove("a")
            .unwrap();
        empty["properties"][""] = p;
        assert!(!consume(empty));
    }

    #[test]
    fn unused_extension_security_survives_td_but_selected_scheme_needs_binding_support() {
        let mut value = td();
        value["securityDefinitions"]["unused"] =
            json!({"scheme":"ex:nonce", "ex:nonceHeader":"X-Nonce"});
        let thing: Thing = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&thing).unwrap()["securityDefinitions"]["unused"],
            value["securityDefinitions"]["unused"]
        );
        assert!(consume(value.clone()));
        value["security"] = json!(["unused"]);
        assert!(!consume(value));
    }

    #[test]
    fn result_identity_seal_does_not_validate_json_or_payload_schema() {
        let generation = PlanSetGeneration::new(Generation::INITIAL);
        let plan = PlanId::new(SlotIndex::new(0), Generation::INITIAL);
        let identity = BindingArtifactIdentity::new(
            generation,
            plan,
            BindingId::new(800),
            BindingGeneration::INITIAL,
            BindingConfigurationDigest::new([1; 32]),
            BindingArtifactCompatibility::new([2; 16]),
            BindingArtifactRole::ConsumerCall,
        );
        let request = OutboundRequest::property_read(
            BindingArtifactRef::new(identity, SlotIndex::new(0)),
            Default::default(),
            None,
        )
        .unwrap();
        let invalid_json =
            InteractionOutput::with_data(Payload::new(&b"not JSON"[..], "application/json"))
                .try_with_metadata(
                    InteractionOutputMetadata::default().with_untrusted_binding_response(
                        BindingResponseMetadata::primary(
                            BindingId::new(800),
                            BindingGeneration::INITIAL,
                            plan,
                            200,
                        ),
                    ),
                )
                .unwrap();
        assert!(validate_untrusted_binding_output(&request, invalid_json).is_ok());
    }
}
