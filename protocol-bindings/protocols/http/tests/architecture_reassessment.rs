#![cfg(feature = "prepare")]
//! Non-production diagnostics retained from Architecture Review 08.
//!
//! These assert reviewed limitations or narrow evidence boundaries, not desired
//! general behavior. Invert/retire the relevant assertion when an independently
//! accepted correction lands; do not preserve a defect to keep this test green.
//! Consumer admission below performs no protocol call or wire I/O. The result
//! kernel probe covers identity/shape only, not installed execution or schemas.
//! See docs/reviews/review-08-integrated-architecture-reassessment.md.

#[path = "support/consumer_read.rs"]
mod consumer_read;

#[cfg(test)]
mod tests {

    use super::consumer_read::{http_registration, prepare_consumer};
    use clinkz_wot_core::host_accounts::HostResourceAccounts;
    use clinkz_wot_core::*;
    use clinkz_wot_foundation::*;
    use clinkz_wot_td::{thing::Thing, validate::Validate};
    use serde_json::{Value, json};
    use std::sync::Arc;

    fn td() -> Value {
        json!({"@context":["https://www.w3.org/2022/wot/td/v1.1", {"ex":"https://example.org/"}],
            "title":"independent audit", "id":"urn:audit",
            "securityDefinitions":{"none":{"scheme":"nosec"}}, "security":["none"],
            "properties":{"a":{"type":"number", "readOnly":true,
                "forms":[{"href":"http://127.0.0.1:9/a"}]}}})
    }
    fn consume(value: Value) -> bool {
        let thing: Thing = serde_json::from_value(value).unwrap();
        thing.validate().expect("ordinary TD Basic validation");
        let bank = Arc::new(HostResourceAccounts::new(GatewayDefaultV1::LIMITS));
        let supported =
            prepare_consumer(&thing, &bank, http_registration(), Default::default()).is_ok();
        assert!(
            ResourceKind::ALL
                .into_iter()
                .all(|k| bank.account(k).used() == 0)
        );
        supported
    }

    #[test]
    fn standalone_consumer_uses_the_source_without_an_id_or_nonempty_key() {
        assert!(consume(td()));
        let mut absent = td();
        absent.as_object_mut().unwrap().remove("id");
        assert!(consume(absent));
        let mut empty = td();
        let p = empty["properties"]
            .as_object_mut()
            .unwrap()
            .remove("a")
            .unwrap();
        empty["properties"][""] = p;
        assert!(consume(empty));
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
        let request = OutboundRequest::finite(
            BindingArtifactRef::new(identity, SlotIndex::new(0)),
            Operation::ReadProperty,
            None,
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
