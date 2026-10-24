use clinkz_wot_td::affordance::ActionAffordance;
use serde_json::json;

#[test]
fn td11_action_synchronicity_is_optional_without_a_preview_feature() {
    for claim in [None, Some(false), Some(true)] {
        let mut value = json!({"forms": []});
        if let Some(claim) = claim {
            value["synchronous"] = json!(claim);
        }
        let action: ActionAffordance = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(action.synchronous, claim);
        assert!(!action._extra_fields.contains_key("synchronous"));
        let output = serde_json::to_value(action).unwrap();
        assert_eq!(output.get("synchronous"), value.get("synchronous"));
    }
}
