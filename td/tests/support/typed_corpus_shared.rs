//! Fixed typed input shared by the typed-corpus tests and snapshot prototype.

extern crate alloc;
use self::alloc::vec;

use super::td_crate::{
    context::Context,
    data_schema::{
        ArraySchema, DataSchema, DataSchemaContext, IntegerSchema, NumberSchema, ObjectSchema,
        StringSchema,
    },
    data_type::MultiLanguage,
    security_scheme::{Qop, SecurityLocation, SecurityScheme},
    thing::Thing,
    validate::{Validate, ValidationLevel},
};
use serde_json::Value;

pub const CORPUS: &str = r##"{
    "@context": [
        "https://www.w3.org/2022/wot/td/v1.1",
        { "ex": "https://example.org/ns#" },
        "https://example.org/extra-context"
    ],
    "id": "urn:example:typed-corpus",
    "@type": ["Sensor", "Thermometer"],
    "title": "Typed corpus",
    "titles": { "en": "Typed corpus", "fr": "Corpus typé" },
    "description": "A fixed typed semantic corpus",
    "descriptions": {
        "en": "A fixed typed semantic corpus",
        "fr": "Un corpus sémantique typé fixe"
    },
    "version": {
        "instance": "1.2.3",
        "model": "temperature-model-v2",
        "ex:build": { "channel": "evidence", "number": 17 }
    },
    "created": "2024-02-29t23:59:58.123456789987-05:30:15",
    "modified": "2024-03-01 00:00:01.000000004z",
    "support": "https://example.org/support",
    "base": "https://example.org/things/",
    "profile": [
        "https://www.w3.org/2022/wot/profile/http-basic/v1",
        "https://example.org/profiles/local"
    ],
    "security": ["none"],
    "securityDefinitions": {
        "none": {
            "@type": ["PublicAccess", "AnonymousAccess"],
            "description": "No credentials required",
            "descriptions": {
                "en": "No credentials required",
                "fr": "Aucun justificatif requis"
            },
            "proxy": "https://proxy.example.org/security",
            "scheme": "nosec",
            "ex:securityHint": { "priority": 0 }
        },
        "none_alt": { "scheme": "nosec" },
        "automatic": { "scheme": "auto" },
        "combined": {
            "scheme": "combo",
            "oneOf": ["basic", "bearer"],
            "allOf": ["api_key", "psk"]
        },
        "basic": { "scheme": "basic", "name": "account", "in": "cookie" },
        "digest": {
            "scheme": "digest",
            "name": "digest-account",
            "in": "uri",
            "qop": "auth-int"
        },
        "api_key": { "scheme": "apikey", "name": "X-API-Key", "in": "header" },
        "bearer": {
            "scheme": "bearer",
            "authorization": "https://auth.example.org/authorize",
            "name": "access-token",
            "alg": "RS256",
            "format": "paseto",
            "in": "body"
        },
        "psk": { "scheme": "psk", "identity": "sensor-17" },
        "oauth": {
            "scheme": "oauth2",
            "authorization": "https://auth.example.org/oauth/authorize",
            "token": "https://auth.example.org/oauth/token",
            "refresh": "https://auth.example.org/oauth/refresh",
            "scopes": ["things.read", "things.write"],
            "flow": "code"
        }
    },
    "properties": {
        "zeta": {
            "type": "string",
            "observable": true,
            "uriVariables": {
                "locale": {
                    "type": "string",
                    "enum": ["en", "fr"],
                    "default": "en"
                },
                "sample": { "type": "integer", "minimum": 0 }
            },
            "forms": [
                {
                    "href": "zeta/first",
                    "op": ["readproperty", "writeproperty"],
                    "contentType": "text/plain",
                    "contentCoding": "identity",
                    "security": ["none", "none_alt"],
                    "scopes": ["things.read", "things.audit"],
                    "response": {
                        "contentType": "application/cbor",
                        "ex:responseHint": { "compact": true }
                    },
                    "additionalResponses": [
                        {
                            "contentType": "application/problem+json",
                            "schema": "mode",
                            "success": true,
                            "ex:status": 202
                        },
                        {
                            "schema": "threshold",
                            "ex:status": 400
                        }
                    ],
                    "subprotocol": "longpoll",
                    "ex:formHint": { "priority": 1 }
                },
                { "href": "zeta/second", "op": "readproperty" }
            ]
        },
        "alpha": {
            "type": "boolean",
            "forms": [{ "href": "alpha", "op": "readproperty" }]
        }
    },
    "actions": {
        "zeta": {
            "@type": ["CalibrateAction", "MaintenanceAction"],
            "title": "Calibrate",
            "titles": { "en": "Calibrate", "fr": "Étalonner" },
            "description": "Calibrate the sensor",
            "descriptions": {
                "en": "Calibrate the sensor",
                "fr": "Étalonner le capteur"
            },
            "uriVariables": {
                "attempt": { "type": "integer", "minimum": 1, "maximum": 3 },
                "channel": {
                    "type": "string",
                    "enum": ["primary", "backup"],
                    "default": "primary"
                }
            },
            "input": {
                "type": "object",
                "properties": {
                    "force": { "type": "boolean" },
                    "offset": { "type": "number", "minimum": -5.0, "maximum": 5.0 }
                },
                "required": ["offset"]
            },
            "output": {
                "type": "string",
                "enum": ["accepted", "rejected"]
            },
            "safe": true,
            "idempotent": true,
            "forms": [
                {
                    "href": "actions/calibrate/{channel}",
                    "op": ["invokeaction", "queryaction"],
                    "contentType": "application/json",
                    "security": ["none"],
                    "scopes": ["things.write"]
                },
                { "href": "actions/calibrate/cancel", "op": "cancelaction" }
            ],
            "ex:actionHint": { "priority": 2 }
        },
        "alpha": {
            "forms": [{ "href": "actions/reset", "op": "invokeaction" }]
        }
    },
    "events": {
        "zeta": {
            "@type": ["AlarmEvent", "TelemetryEvent"],
            "title": "Alarm",
            "titles": { "en": "Alarm", "fr": "Alarme" },
            "description": "Reports alarm state changes",
            "descriptions": {
                "en": "Reports alarm state changes",
                "fr": "Signale les changements d’alarme"
            },
            "uriVariables": {
                "severity": {
                    "type": "string",
                    "enum": ["warning", "critical"],
                    "default": "warning"
                },
                "window": { "type": "integer", "minimum": 1, "maximum": 60 }
            },
            "subscription": {
                "type": "object",
                "properties": {
                    "threshold": { "type": "number", "minimum": 0.0 }
                },
                "required": ["threshold"]
            },
            "data": {
                "type": "object",
                "properties": {
                    "active": { "type": "boolean" },
                    "message": { "type": "string", "minLength": 1 }
                },
                "required": ["message"]
            },
            "dataResponse": {
                "type": "string",
                "enum": ["acknowledged", "retry"]
            },
            "cancellation": {
                "type": "object",
                "properties": {
                    "reason": { "type": "string" }
                },
                "required": ["reason"]
            },
            "forms": [
                {
                    "href": "events/alarm/{severity}",
                    "op": ["subscribeevent", "unsubscribeevent"],
                    "contentType": "application/json",
                    "security": ["none"],
                    "scopes": ["things.observe"]
                },
                { "href": "events/alarm/stream", "op": "subscribeevent" }
            ],
            "ex:eventHint": { "priority": 3 }
        },
        "alpha": {
            "forms": [{ "href": "events/status", "op": "subscribeevent" }]
        }
    },
    "links": [
        {
            "href": "docs/manual?edition=2",
            "type": "application/pdf",
            "rel": "service-doc",
            "anchor": "https://example.org/things/typed-corpus",
            "sizes": "16x16 32x32",
            "hreflang": ["en", "fr"],
            "ex:linkHint": { "priority": 4 }
        },
        { "href": "related/item" }
    ],
    "schemaDefinitions": {
        "mode": { "type": "string", "enum": ["auto", "manual"] },
        "threshold": { "type": "number", "minimum": 0.25, "maximum": 9.5 }
    },
    "uriVariables": {
        "tenant": { "type": "string", "minLength": 1, "maxLength": 32 }
    },
    "forms": [],
    "ex:payload": {
        "flag": true,
        "count": 123456789012345678901234567890,
        "empty": null,
        "items": ["first", { "left": "L", "right": "R" }]
    }
}"##;

pub fn typed_corpus() -> Thing {
    let mut thing: Thing = serde_json::from_str(CORPUS).expect("fixed TD input must decode");
    #[cfg(feature = "td2-preview")]
    {
        thing
            .actions
            .as_mut()
            .unwrap()
            .get_mut("zeta")
            .unwrap()
            .synchronous = Some(true);
    }
    thing
        ._extra_fields
        .insert("ex:long".into(), Value::String("λ".repeat(2_048)));
    thing
        .validate_with_level(ValidationLevel::Basic)
        .expect("fixed typed corpus must pass Basic");
    thing
}

pub fn assert_complete_security_corpus(thing: &Thing) {
    let definitions = &thing.security_definitions;
    assert_eq!(definitions.len(), 10);

    let SecurityScheme::NoSec(none) = &definitions["none"] else {
        panic!("none must retain its NoSec variant");
    };
    assert_eq!(
        none._context.tags.as_deref(),
        Some(["PublicAccess".into(), "AnonymousAccess".into()].as_slice())
    );
    assert_eq!(
        none._context.description.as_deref(),
        Some("No credentials required")
    );
    assert_eq!(
        none._context
            .descriptions
            .as_ref()
            .unwrap()
            .get("fr")
            .unwrap(),
        "Aucun justificatif requis"
    );
    assert_eq!(
        none._context.proxy.as_ref().unwrap().as_str(),
        "https://proxy.example.org/security"
    );
    assert_eq!(
        none._context._extra_fields["ex:securityHint"]["priority"],
        serde_json::json!(0)
    );

    assert!(matches!(definitions["none_alt"], SecurityScheme::NoSec(_)));
    assert!(matches!(definitions["automatic"], SecurityScheme::Auto(_)));

    let SecurityScheme::Combo(combined) = &definitions["combined"] else {
        panic!("combined must retain its Combo variant");
    };
    assert_eq!(combined.one_of, ["basic", "bearer"]);
    assert_eq!(combined.all_of, ["api_key", "psk"]);

    let SecurityScheme::Basic(basic) = &definitions["basic"] else {
        panic!("basic must retain its Basic variant");
    };
    assert_eq!(basic.name.as_deref(), Some("account"));
    assert_eq!(basic.location, SecurityLocation::Cookie);

    let SecurityScheme::Digest(digest) = &definitions["digest"] else {
        panic!("digest must retain its Digest variant");
    };
    assert_eq!(digest.name.as_deref(), Some("digest-account"));
    assert_eq!(digest.location, SecurityLocation::Uri);
    assert_eq!(digest.qop, Qop::AuthInt);

    let SecurityScheme::APIKey(api_key) = &definitions["api_key"] else {
        panic!("api_key must retain its APIKey variant");
    };
    assert_eq!(api_key.name.as_deref(), Some("X-API-Key"));
    assert_eq!(api_key.location, SecurityLocation::Header);

    let SecurityScheme::Bearer(bearer) = &definitions["bearer"] else {
        panic!("bearer must retain its Bearer variant");
    };
    assert_eq!(
        bearer.authorization.as_ref().unwrap().as_str(),
        "https://auth.example.org/authorize"
    );
    assert_eq!(bearer.name.as_deref(), Some("access-token"));
    assert_eq!(bearer.alg, "RS256");
    assert_eq!(bearer.format, "paseto");
    assert_eq!(bearer.location, SecurityLocation::Body);

    let SecurityScheme::PSK(psk) = &definitions["psk"] else {
        panic!("psk must retain its PSK variant");
    };
    assert_eq!(psk.identity.as_deref(), Some("sensor-17"));

    let SecurityScheme::OAuth2(oauth) = &definitions["oauth"] else {
        panic!("oauth must retain its OAuth2 variant");
    };
    assert_eq!(
        oauth.authorization.as_ref().unwrap().as_str(),
        "https://auth.example.org/oauth/authorize"
    );
    assert_eq!(
        oauth.token.as_ref().unwrap().as_str(),
        "https://auth.example.org/oauth/token"
    );
    assert_eq!(
        oauth.refresh.as_ref().unwrap().as_str(),
        "https://auth.example.org/oauth/refresh"
    );
    assert_eq!(
        oauth.scopes.as_deref(),
        Some(["things.read".into(), "things.write".into()].as_slice())
    );
    assert_eq!(oauth.flow, "code");
}

pub fn serializer_failure_thing() -> Thing {
    let mut thing = typed_corpus();
    thing.context =
        serde_json::from_str::<Context>(r#"["https://example.org/extension-only"]"#).unwrap();
    thing
        .validate_with_level(ValidationLevel::Basic)
        .expect("Basic accepts extension-only Context");
    thing
}

pub fn nested_schema_corpus() -> Thing {
    let mut thing = typed_corpus();
    let label = DataSchema::String(StringSchema {
        _context: DataSchemaContext {
            data_type: Some("string".into()),
            enumerate: Some(vec![
                Value::String("warm".into()),
                Value::String("cold".into()),
            ]),
            ..Default::default()
        },
        min_length: Some(1),
        max_length: Some(24),
        pattern: Some("^[a-z]+$".into()),
        content_encoding: Some("utf-8".into()),
        content_media_type: Some("text/plain".into()),
    });
    let reading = DataSchema::Object(ObjectSchema {
        _context: DataSchemaContext::default(),
        properties: Some(
            [
                ("label".into(), label),
                (
                    "level".into(),
                    DataSchema::Integer(IntegerSchema {
                        minimum: Some(-10),
                        maximum: Some(10),
                        multiple_of: Some(2),
                        ..Default::default()
                    }),
                ),
                (
                    "ratio".into(),
                    DataSchema::Number(NumberSchema {
                        minimum: Some(0.25),
                        exclusive_maximum: Some(9.5),
                        ..Default::default()
                    }),
                ),
            ]
            .into(),
        ),
        required: Some(vec!["label".into(), "level".into()]),
    });
    let mut context = DataSchemaContext {
        data_type: Some("object".into()),
        constant: Some(serde_json::json!({"nested": [null, true]})),
        default: Some(serde_json::json!({"nested": []})),
        unit: Some("sample".into()),
        one_of: Some(vec![
            DataSchema::Boolean(Default::default()),
            DataSchema::Null(Default::default()),
        ]),
        enumerate: Some(vec![
            serde_json::json!({"a": 1}),
            serde_json::json!(["x", "y"]),
        ]),
        read_only: true,
        format: Some("custom-object".into()),
        ..Default::default()
    };
    context._metadata.tags = Some(vec!["Sensor".into(), "Sample".into()]);
    context._metadata.title = Some("Readings".into());
    context._metadata.titles = Some(
        MultiLanguage::new()
            .with("en", "Readings")
            .with("fr", "Mesures"),
    );
    context._metadata.description = Some("Nested readings".into());
    context._metadata.descriptions = Some(MultiLanguage::new().with("en", "Nested readings"));
    context
        ._extra_fields
        .insert("ex:opaque".into(), serde_json::from_str("1e309").unwrap());
    context._extra_fields.insert(
        "ex:deep".into(),
        serde_json::json!({"items": [false, {"key": "value"}]}),
    );
    thing
        .properties
        .as_mut()
        .unwrap()
        .get_mut("alpha")
        .unwrap()
        ._schema = DataSchema::Object(ObjectSchema {
        _context: context,
        properties: Some(
            [(
                "samples".into(),
                DataSchema::Array(ArraySchema {
                    items: Some(vec![reading, DataSchema::Null(Default::default())]),
                    min_items: Some(1),
                    max_items: Some(8),
                    ..Default::default()
                }),
            )]
            .into(),
        ),
        required: Some(vec!["samples".into()]),
    });
    thing
        .validate_with_level(ValidationLevel::Basic)
        .expect("nested typed schema must pass Basic");
    thing
}
