use clinkz_wot_td::{
    data_schema::{ContextHelper, DataSchema},
    security_scheme::{SecurityLocation, SecurityScheme},
    thing::Thing,
    validate::{Validate, ValidateError, ValidationLevel},
};
use std::{fs, path::PathBuf};

#[test]
fn test_thing_roundtrip_fidelity() {
    // Use CARGO_MANIFEST_DIR so the fixture path is stable from any workspace cwd.
    let mut fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixtures_path.push("tests/fixtures");

    let paths = fs::read_dir(&fixtures_path)
        .unwrap_or_else(|_| panic!("Failed to read fixtures at {:?}", fixtures_path));

    for entry in paths {
        let path_buf = entry.unwrap().path();

        // Only process JSON and JSON-LD fixtures.
        let ext = path_buf.extension().and_then(|s| s.to_str());
        if ext != Some("json") && ext != Some("jsonld") {
            continue;
        }

        // Parse the original fixture into a generic JSON value for structural comparison.
        let raw_json = fs::read_to_string(&path_buf).expect("Read failed");
        let mut original_value: serde_json::Value = serde_json::from_str(&raw_json)
            .unwrap_or_else(|_| panic!("Original JSON is invalid: {:?}", path_buf));
        sanitize_json(&mut original_value);

        // Deserialize the fixture into the Thing model.
        let thing: Thing = serde_json::from_str(&raw_json)
            .unwrap_or_else(|_| panic!("Failed to deserialize into Thing: {:?}", path_buf));

        // Run explicit TD validation.
        thing
            .validate()
            .unwrap_or_else(|_| panic!("Logic validation failed: {:?}", path_buf));

        // Serialize the Thing back to JSON.
        let serialized_json = serde_json::to_string(&thing)
            .unwrap_or_else(|_| panic!("Failed to serialize: {:?}", path_buf));

        // Parse the serialized JSON for semantic comparison.
        let mut serialized_value: serde_json::Value =
            serde_json::from_str(&serialized_json).unwrap();
        sanitize_json(&mut serialized_value);

        // Compare JSON values to ignore field order and whitespace while checking fidelity.
        assert_json_eq(&original_value, &serialized_value, &path_buf);
    }
}

fn sanitize_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            // Recursively sanitize child values.
            map.values_mut().for_each(sanitize_json);
            // Remove empty objects, empty arrays, and null values.
            map.retain(|_, v| {
                !(v.is_null()
                    || v.is_object() && v.as_object().unwrap().is_empty()
                    || v.is_array() && v.as_array().unwrap().is_empty())
            });
        }
        serde_json::Value::Array(arr) => {
            arr.iter_mut().for_each(sanitize_json);
        }
        _ => {}
    }
}

/// Compares JSON values semantically.
fn assert_json_eq(
    original: &serde_json::Value,
    serialized: &serde_json::Value,
    path: &std::path::Path,
) {
    // Unknown Thing fields should be stored in extension maps and serialized back out.

    if !is_semantic_eq(original, serialized) {
        // Print detailed values when a fixture does not round-trip.
        panic!(
            "Round-trip fidelity check failed for {:?}.\nOriginal: {}\nSerialized: {}",
            path,
            serde_json::to_string_pretty(original).unwrap(),
            serde_json::to_string_pretty(serialized).unwrap()
        );
    }
}

// Attempts a loose ISO 8601 date-time comparison.
fn try_compare_dates(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    if let (Some(s1), Some(s2)) = (a.as_str(), b.as_str()) {
        // Keep this dependency-free by trimming common serialized precision differences.
        return s1.trim_end_matches('Z').trim_end_matches('0')
            == s2.trim_end_matches('Z').trim_end_matches('0');
    }
    false
}

fn is_semantic_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value::*;

    match (a, b) {
        // Handle WoT OneOrMany shorthand, such as "op": ["read"] vs "op": "read".
        (Array(arr), other) if arr.len() == 1 => is_semantic_eq(&arr[0], other),
        (other, Array(arr)) if arr.len() == 1 => is_semantic_eq(other, &arr[0]),

        // Compare objects recursively and allow missing default values.
        (Object(map_a), Object(map_b)) => {
            let all_keys: std::collections::HashSet<_> = map_a.keys().chain(map_b.keys()).collect();
            for key in all_keys {
                let val_a = map_a.get(key).unwrap_or(&Null);
                let val_b = map_b.get(key).unwrap_or(&Null);

                // If values differ, check whether this is a missing-vs-default case.
                if val_a != val_b {
                    // Account for date-time precision differences.
                    if (key == "created"
                        || key == "modified"
                        || key == "last_changed"
                        || key == "last_updated")
                        && try_compare_dates(val_a, val_b)
                    {
                        continue;
                    }

                    if is_default_value(key, val_a) && val_b.is_null() {
                        continue;
                    }
                    // if is_default_value(key, val_b) && val_a.is_null() { continue; }
                    if !is_semantic_eq(val_a, val_b) {
                        return false;
                    }
                }
            }
            true
        }

        // Compare arrays deeply.
        (Array(arr_a), Array(arr_b)) => {
            if arr_a.len() != arr_b.len() {
                return false;
            }
            arr_a
                .iter()
                .zip(arr_b.iter())
                .all(|(ia, ib)| is_semantic_eq(ia, ib))
        }

        // Treat JSON numbers with the same numeric value as equivalent.
        (Number(_), Number(_)) => a.as_f64() == b.as_f64(),

        // Compare primitive values directly.
        (v1, v2) => v1 == v2,
    }
}

/// Checks whether a serialized value is a TD default.
fn is_default_value(key: &str, value: &serde_json::Value) -> bool {
    match key {
        // Boolean defaults are false.
        "readOnly" | "writeOnly" | "observable" | "safe" | "idempotent" | "success" => {
            value == &serde_json::Value::Bool(false)
        }
        "contentType" => value == "application/json",
        "in" => value == "header",
        "qop" => value == "auth",
        "alg" => value == "ES256",
        "format" => value == "jwt",
        _ => false,
    }
}

#[test]
fn minimal_validation_accepts_deserialized_shape_without_basic_requirements() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "security": [],
        "securityDefinitions": {}
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD shape should deserialize");
    thing
        .validate_with_level(ValidationLevel::Minimal)
        .expect("minimal validation should accept serde-valid TD shape");

    let err = thing
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject missing title");
    assert!(matches!(err, ValidateError::MissingRequiredField(field) if field == "title"));
}

#[test]
fn basic_validation_rejects_unknown_thing_security_reference() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Unknown Security",
        "security": "missing_sc",
        "securityDefinitions": {
            "nosec_sc": { "scheme": "nosec" }
        }
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD shape should deserialize");
    let err = thing
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("unknown security reference should fail basic validation");

    assert!(matches!(
        err,
        ValidateError::InvalidReference { context, reference }
            if context == "Thing.security" && reference == "missing_sc"
    ));
}

#[test]
fn basic_validation_rejects_unknown_form_security_reference() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Unknown Form Security",
        "security": "nosec_sc",
        "securityDefinitions": {
            "nosec_sc": { "scheme": "nosec" }
        },
        "properties": {
            "status": {
                "type": "string",
                "forms": [
                    {
                        "href": "/properties/status",
                        "op": "readproperty",
                        "security": "missing_sc"
                    }
                ]
            }
        }
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD shape should deserialize");
    let err = thing
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("unknown form security reference should fail basic validation");

    assert!(matches!(
        err,
        ValidateError::InvalidReference { context, reference }
            if context == "Property 'status'.forms[0].security" && reference == "missing_sc"
    ));
}

#[test]
fn security_scheme_deserialization_uses_scheme_to_select_the_concrete_variant() {
    let raw = r#"{
        "scheme": "apikey",
        "name": "X-API-Key",
        "in": "header",
        "cz:credentialSource": "platform"
    }"#;

    let scheme: SecurityScheme =
        serde_json::from_str(raw).expect("security scheme should deserialize");

    let SecurityScheme::APIKey(scheme) = scheme else {
        panic!("scheme-based deserialization should select the API key variant");
    };
    assert_eq!(scheme._context.scheme, "apikey");
    assert_eq!(scheme.name.as_deref(), Some("X-API-Key"));
    assert_eq!(scheme._context._extra_fields.get("name"), None);
    assert_eq!(
        scheme._context._extra_fields.get("cz:credentialSource"),
        Some(&serde_json::json!("platform"))
    );
}

#[test]
fn security_scheme_deserialization_rejects_unknown_scheme_values() {
    let raw = r#"{
        "scheme": "custom-scheme"
    }"#;

    let err = serde_json::from_str::<SecurityScheme>(raw)
        .expect_err("unknown security schemes should fail during deserialization");

    assert!(err.to_string().contains("unsupported security scheme"));
}

#[test]
fn security_scheme_deserialization_accepts_uri_api_key_locations() {
    let raw = r#"{
        "scheme": "apikey",
        "in": "uri",
        "name": "urlKey"
    }"#;

    let scheme: SecurityScheme =
        serde_json::from_str(raw).expect("API key security scheme should deserialize");

    let SecurityScheme::APIKey(scheme) = scheme else {
        panic!("scheme-based deserialization should preserve the API key variant");
    };
    assert_eq!(scheme.location, SecurityLocation::Uri);
    assert_eq!(scheme.name.as_deref(), Some("urlKey"));
}

#[test]
fn basic_validation_rejects_apikey_without_name() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Invalid API Key",
        "security": "apikey_sc",
        "securityDefinitions": {
            "apikey_sc": {
                "scheme": "apikey",
                "in": "header"
            }
        }
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD shape should deserialize");
    let err = thing
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject apikey schemes without name");

    assert!(
        matches!(err, ValidateError::MissingRequiredField(field) if field.contains("securityDefinitions.apikey_sc") && field.contains("name"))
    );
}

#[test]
fn basic_validation_rejects_combo_security_unknown_references() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Invalid Combo",
        "security": "combo_sc",
        "securityDefinitions": {
            "basic_sc": { "scheme": "basic", "name": "Authorization" },
            "combo_sc": {
                "scheme": "combo",
                "oneOf": ["basic_sc", "missing_sc"]
            }
        }
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD shape should deserialize");
    let err = thing
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject unknown combo references");

    assert!(
        matches!(err, ValidateError::InvalidReference { context, reference }
            if context == "securityDefinitions.combo_sc.oneOf" && reference == "missing_sc")
    );
}

#[test]
fn basic_validation_rejects_oauth2_code_without_token_endpoint() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Invalid OAuth2",
        "security": "oauth_sc",
        "securityDefinitions": {
            "oauth_sc": {
                "scheme": "oauth2",
                "flow": "code",
                "authorization": "https://example.com/oauth/authorize"
            }
        }
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD shape should deserialize");
    let err = thing
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject code flow without token endpoint");

    assert!(
        matches!(err, ValidateError::MissingRequiredField(field) if field.contains("securityDefinitions.oauth_sc") && field.contains("token"))
    );
}

#[test]
fn validation_levels_control_affordance_operation_checks() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Invalid Property Operation",
        "security": "nosec_sc",
        "securityDefinitions": {
            "nosec_sc": { "scheme": "nosec" }
        },
        "properties": {
            "status": {
                "type": "string",
                "forms": [
                    {
                        "href": "/properties/status",
                        "op": "invokeaction"
                    }
                ]
            }
        }
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD shape should deserialize");
    thing
        .validate_with_level(ValidationLevel::Minimal)
        .expect("minimal validation should not run operation context checks");

    let err = thing
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject invalid property operation");

    assert!(
        matches!(err, ValidateError::InvalidOperation { context, .. } if context == "Property 'status'")
    );
}

#[test]
fn basic_validation_rejects_direct_data_schema_constraint_conflicts() {
    let schema = DataSchema::String(DataSchema::string().min_length(8).max_length(4).build());

    let err = schema
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject minLength greater than maxLength");

    assert!(matches!(err, ValidateError::InvalidSchema(message) if message.contains("minLength")));
}

#[test]
fn basic_validation_rejects_explicit_data_schema_type_mismatches() {
    let schema = DataSchema::String(DataSchema::string().data_type("integer").build());

    schema
        .validate_with_level(ValidationLevel::Minimal)
        .expect("minimal validation should keep tolerant data schema parsing");

    let err = schema
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject mismatched explicit data schema types");

    assert!(
        matches!(err, ValidateError::InvalidSchema(message) if message.contains("type 'integer'") && message.contains("string schema"))
    );
}

#[test]
fn basic_validation_contextualizes_explicit_data_schema_type_mismatches() {
    let err = Thing::builder("Invalid Schema Type")
        .nosec()
        .schema_definition(
            "badString",
            DataSchema::String(DataSchema::string().data_type("integer").build()),
        )
        .build()
        .expect_err("thing validation should reject mismatched schema definition types");

    assert!(
        matches!(err, ValidateError::InvalidSchema(message) if message.contains("schemaDefinitions.badString") && message.contains("type 'integer'") && message.contains("string schema"))
    );
}

#[test]
fn basic_validation_rejects_deserialized_property_schema_constraint_conflicts() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Invalid Property Schema",
        "security": "nosec_sc",
        "securityDefinitions": {
            "nosec_sc": { "scheme": "nosec" }
        },
        "properties": {
            "label": {
                "type": "string",
                "minLength": 8,
                "maxLength": 4,
                "forms": [
                    {
                        "href": "/properties/label",
                        "op": "readproperty"
                    }
                ]
            }
        }
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD shape should deserialize");
    thing
        .validate_with_level(ValidationLevel::Minimal)
        .expect("minimal validation should not run schema constraint checks");

    let err = thing
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject invalid property schema constraints");

    assert!(
        matches!(err, ValidateError::InvalidSchema(message) if message.contains("Property 'label'") && message.contains("minLength"))
    );
}

#[test]
fn basic_validation_rejects_schema_definition_multiple_of_zero() {
    let raw = r#"{
        "@context": "https://www.w3.org/2022/wot/td/v1.1",
        "title": "Invalid Schema Definition",
        "security": "nosec_sc",
        "securityDefinitions": {
            "nosec_sc": { "scheme": "nosec" }
        },
        "schemaDefinitions": {
            "badNumber": {
                "type": "number",
                "multipleOf": 0
            }
        }
    }"#;

    let thing: Thing = serde_json::from_str(raw).expect("TD shape should deserialize");
    let err = thing
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject non-positive multipleOf");

    assert!(
        matches!(err, ValidateError::InvalidSchema(message) if message.contains("schemaDefinitions.badNumber") && message.contains("multipleOf"))
    );
}

#[test]
fn basic_validation_rejects_builder_number_schema_multiple_of_zero() {
    let schema = DataSchema::Number(DataSchema::number().multiple_of(0.0).build());

    let err = schema
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject non-positive builder multipleOf");

    assert!(matches!(err, ValidateError::InvalidSchema(message) if message.contains("multipleOf")));
}

#[test]
fn basic_validation_rejects_builder_integer_schema_multiple_of_zero() {
    let schema = DataSchema::Integer(DataSchema::integer().multiple_of(0).build());

    let err = schema
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject non-positive builder multipleOf");

    assert!(matches!(err, ValidateError::InvalidSchema(message) if message.contains("multipleOf")));
}

#[test]
fn basic_validation_rejects_nested_data_schema_constraint_conflicts() {
    let schema = DataSchema::Object(
        DataSchema::object()
            .property(
                "items",
                DataSchema::Array(DataSchema::array().min_items(3).max_items(1).build()),
            )
            .build(),
    );

    let err = schema
        .validate_with_level(ValidationLevel::Basic)
        .expect_err("basic validation should reject nested schema conflicts");

    assert!(
        matches!(err, ValidateError::InvalidSchema(message) if message.contains("properties.items") && message.contains("minItems"))
    );
}
