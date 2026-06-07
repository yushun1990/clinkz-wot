# TD 1.1 Field Coverage Matrix

This audit compares the current `clinkz-wot-td` data model with the W3C WoT TD
1.1 vocabulary. The reference target is the W3C Recommendation:
<https://www.w3.org/TR/wot-thing-description/>.

Status values:

- `covered`: the field is represented with an appropriate Rust type and serde
  mapping.
- `partial`: the field is represented, but validation, typing, or extension
  preservation needs follow-up work.
- `gap`: the field or expected behavior is missing.

## Thing

| TD term | Rust field | Rust type | Required | OneOrMany | Defaults | Extensions | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `@context` | `Thing::context` | `Context` | Yes | String or array | WoT TD 1.1 default in builder/default | Object context entries preserve extension vocabulary terms | partial | Deserialization accepts string/object array forms, but validation levels still need to check required standard context rules explicitly. |
| `@type` | `Thing::_metadata.tags` | `Option<Vec<String>>` | No | Yes | None | N/A | covered | Flattened through `Metadata`. |
| `title` | `Thing::_metadata.title` | `Option<String>` | Yes | No | None | N/A | partial | Current validation checks non-empty title. Type is optional to keep deserialization tolerant. |
| `titles` | `Thing::_metadata.titles` | `Option<MultiLanguage>` | No | No | None | N/A | covered | Multi-language map is represented. |
| `description` | `Thing::_metadata.description` | `Option<String>` | No | No | None | N/A | covered | Flattened through `Metadata`. |
| `descriptions` | `Thing::_metadata.descriptions` | `Option<MultiLanguage>` | No | No | None | N/A | covered | Multi-language map is represented. |
| `id` | `Thing::id` | `Option<AbsoluteUri>` | No | No | None | N/A | covered | Absolute URI enforced during parse and deserialization. |
| `version` | `Thing::version` | `Option<VersionInfo>` | No | No | None | `VersionInfo::_extra_fields` | covered | `instance` and `model` represented. |
| `created` | `Thing::created` | `Option<OffsetDateTime>` | No | No | None | N/A | covered | RFC3339 serde helper works under `no_std + alloc`. |
| `modified` | `Thing::modified` | `Option<OffsetDateTime>` | No | No | None | N/A | covered | RFC3339 serde helper works under `no_std + alloc`. |
| `support` | `Thing::support` | `Option<AbsoluteUri>` | No | No | None | N/A | covered | Absolute URI enforced. |
| `base` | `Thing::base` | `Option<BaseUri>` | No | No | None | N/A | covered | Type accepts absolute URI and absolute URI template. `resolve_form_href` applies concrete base values to relative form `href` values and preserves URI templates for runtime expansion. |
| `properties` | `Thing::properties` | `Option<BTreeMap<String, PropertyAffordance>>` | No | No | None | Via affordance/schema extension maps | covered | Map of property affordances represented. |
| `actions` | `Thing::actions` | `Option<BTreeMap<String, ActionAffordance>>` | No | No | None | `ActionAffordance::_extra_fields` | covered | Map of action affordances represented. |
| `events` | `Thing::events` | `Option<BTreeMap<String, EventAffordance>>` | No | No | None | `EventAffordance::_extra_fields` | covered | Map of event affordances represented. |
| `links` | `Thing::links` | `Option<Vec<Link>>` | No | No | None | `Link::_extra_fields` | covered | Array of links represented. |
| `forms` | `Thing::forms` | `Option<Vec<Form>>` | No | No | None | `Form::_extra_fields` | covered | Top-level forms represented. |
| `security` | `Thing::security` | `Vec<String>` | Yes | Yes | None | N/A | covered | OneOrMany represented. Basic validation checks non-empty values and resolves names against `securityDefinitions`. |
| `securityDefinitions` | `Thing::security_definitions` | `BTreeMap<String, SecurityScheme>` | Yes | No | None | Via scheme extension maps | covered | Map represented. Basic validation checks security definitions, selected scheme constraints, and combo references against the definition map. |
| `profile` | `Thing::profile` | `Option<Vec<AbsoluteUri>>` | No | Yes | None | N/A | covered | OneOrMany absolute URI list represented. |
| `schemaDefinitions` | `Thing::schema_definitions` | `Option<BTreeMap<String, DataSchema>>` | No | No | None | Via schema extension maps | covered | Map represented. |
| `uriVariables` | `Thing::uri_variables` | `Option<BTreeMap<String, DataSchema>>` | No | No | None | Via schema extension maps | covered | Thing-level URI variables represented. |
| Unknown TD terms | `Thing::_extra_fields` | `ExtensionMap` | No | N/A | N/A | Yes | covered | Unknown root fields are preserved. |

## Interaction Affordances

| TD term | Rust field | Rust type | Applies to | Required | OneOrMany | Defaults | Extensions | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `@type` | `_metadata.tags` or schema metadata | `Option<Vec<String>>` | Action, Event, Property via schema flattening | No | Yes | None | N/A | covered | Property metadata comes through flattened `DataSchema`; action/event use `Metadata`. |
| `title` | `_metadata.title` or schema metadata | `Option<String>` | Action, Event, Property via schema flattening | No | No | None | N/A | covered | Optional affordance title represented. |
| `titles` | `_metadata.titles` or schema metadata | `Option<MultiLanguage>` | Action, Event, Property via schema flattening | No | No | None | N/A | covered | Multi-language titles represented. |
| `description` | `_metadata.description` or schema metadata | `Option<String>` | Action, Event, Property via schema flattening | No | No | None | N/A | covered | Optional affordance description represented. |
| `descriptions` | `_metadata.descriptions` or schema metadata | `Option<MultiLanguage>` | Action, Event, Property via schema flattening | No | No | None | N/A | covered | Multi-language descriptions represented. |
| `forms` | `InteractionAffordance::forms` | `Vec<Form>` | Property, Action, Event | No | No | Empty vector | Via forms | partial | Represented. Default operation inference and explicit operation-context validation are implemented; stricter Profile-level form presence rules remain future work. |
| `uriVariables` | `InteractionAffordance::uri_variables` | `Option<BTreeMap<String, DataSchema>>` | Property, Action, Event | No | No | None | Via schema extension maps | covered | Interaction-level URI variables represented. |
| `observable` | `PropertyAffordance::observable` | `bool` | Property | No | No | `false` | N/A | covered | Flexible bool deserializer preserves default behavior. |
| `input` | `ActionAffordance::input` | `Option<DataSchema>` | Action | No | No | None | Via schema extension maps | covered | Represented. |
| `output` | `ActionAffordance::output` | `Option<DataSchema>` | Action | No | No | None | Via schema extension maps | covered | Represented. |
| `safe` | `ActionAffordance::safe` | `bool` | Action | No | No | `false` | N/A | covered | Default represented. |
| `idempotent` | `ActionAffordance::idempotent` | `bool` | Action | No | No | `false` | N/A | covered | Default represented. |
| `synchronous` | `ActionAffordance::synchronous` | `Option<bool>` | Action | No | No | None | N/A | covered | Represented with flexible bool deserializer. |
| `subscription` | `EventAffordance::subscription` | `Option<DataSchema>` | Event | No | No | None | Via schema extension maps | covered | Represented. |
| `data` | `EventAffordance::data` | `Option<DataSchema>` | Event | No | No | None | Via schema extension maps | covered | Represented. |
| `dataResponse` | `EventAffordance::data_response` | `Option<DataSchema>` | Event | No | No | None | Via schema extension maps | covered | Represented. |
| `cancellation` | `EventAffordance::cancellation` | `Option<DataSchema>` | Event | No | No | None | Via schema extension maps | covered | Represented. |
| Unknown affordance terms | `_extra_fields` or schema extension map | `ExtensionMap` | Property, Action, Event | No | N/A | N/A | Yes | covered | Action and Event have dedicated extension maps. Property extension preservation is handled through the flattened `DataSchema` extension map and builder/round-trip coverage. |

## Form

| TD term | Rust field | Rust type | Required | OneOrMany | Defaults | Extensions | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `href` | `Form::href` | `FormHref` | Yes | No | None | N/A | covered | URI references and URI templates represented. |
| `contentType` | `Form::content_type` | `String` | No | No | `application/json` | N/A | covered | Default is applied and skipped during serialization when unchanged. |
| `contentCoding` | `Form::content_coding` | `Option<String>` | No | No | None | N/A | covered | Represented. |
| `security` | `Form::security` | `Option<Vec<String>>` | No | Yes | Inherit from Thing/interactions | N/A | covered | OneOrMany represented. Basic validation checks form-level references, and `td_defaults::effective_form_security` resolves form overrides versus Thing-level inheritance. |
| `scopes` | `Form::scopes` | `Option<Vec<String>>` | No | Yes | None | N/A | covered | OneOrMany represented. |
| `response` | `Form::response` | `Option<ExpectedResponse>` | No | No | None | `ExpectedResponse::_extra_fields` | covered | Primary response metadata represented. |
| `additionalResponses` | `Form::additional_responses` | `Option<Vec<AdditionalExpectedResponse>>` | No | No | None | `AdditionalExpectedResponse::_extra_fields` | covered | Additional response metadata represented. |
| `subprotocol` | `Form::subprotocol` | `Option<String>` | No | No | None | N/A | covered | Represented. |
| `op` | `Form::op` | `Option<Vec<Operation>>` | No | Yes | Context-dependent | N/A | covered | Operations are typed. Basic validation rejects operations outside the affordance context, and `td_defaults::effective_form_operations` implements TD 1.1 default inference. |
| Unknown form terms | `Form::_extra_fields` | `ExtensionMap` | No | N/A | N/A | Yes | covered | Unknown form fields are preserved. |

## Link

| TD term | Rust field | Rust type | Required | OneOrMany | Defaults | Extensions | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `href` | `Link::href` | `UriReference` | Yes | No | None | N/A | covered | URI references represented; URI templates are rejected. |
| `type` | `Link::content_type` | `Option<String>` | No | No | None | N/A | covered | Serialized as `type`. |
| `rel` | `Link::rel` | `Option<String>` | No | No | None | N/A | covered | Represented. |
| `anchor` | `Link::anchor` | `Option<UriReference>` | No | No | None | N/A | covered | URI references represented. |
| `sizes` | `Link::sizes` | `Option<String>` | No | No | None | N/A | covered | Represented. |
| `hreflang` | `Link::hreflang` | `Option<Vec<String>>` | No | Yes | None | N/A | covered | OneOrMany represented. |
| Unknown link terms | `Link::_extra_fields` | `ExtensionMap` | No | N/A | N/A | Yes | covered | Unknown link fields are preserved. |

## DataSchema

| TD term | Rust field | Rust type | Applies to | Required | OneOrMany | Defaults | Extensions | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `@type` | `DataSchemaContext::_metadata.tags` | `Option<Vec<String>>` | All schemas | No | Yes | None | N/A | covered | Flattened metadata. |
| `title` | `DataSchemaContext::_metadata.title` | `Option<String>` | All schemas | No | No | None | N/A | covered | Represented. |
| `titles` | `DataSchemaContext::_metadata.titles` | `Option<MultiLanguage>` | All schemas | No | No | None | N/A | covered | Represented. |
| `description` | `DataSchemaContext::_metadata.description` | `Option<String>` | All schemas | No | No | None | N/A | covered | Represented. |
| `descriptions` | `DataSchemaContext::_metadata.descriptions` | `Option<MultiLanguage>` | All schemas | No | No | None | N/A | covered | Represented. |
| `const` | `DataSchemaContext::constant` | `Option<serde_json::Value>` | All schemas | No | No | None | N/A | covered | Represented as JSON value. |
| `default` | `DataSchemaContext::default` | `Option<serde_json::Value>` | All schemas | No | No | None | N/A | covered | Represented as JSON value. |
| `unit` | `DataSchemaContext::unit` | `Option<String>` | All schemas | No | No | None | N/A | covered | Represented. |
| `oneOf` | `DataSchemaContext::one_of` | `Option<Vec<DataSchema>>` | All schemas | No | No | None | Via nested schemas | covered | Represented. |
| `enum` | `DataSchemaContext::enumerate` | `Option<Vec<serde_json::Value>>` | All schemas | No | No | None | N/A | covered | Represented as JSON values. |
| `readOnly` | `DataSchemaContext::read_only` | `bool` | All schemas | No | No | `false` | N/A | covered | Flexible bool deserializer. |
| `writeOnly` | `DataSchemaContext::write_only` | `bool` | All schemas | No | No | `false` | N/A | covered | Flexible bool deserializer. |
| `format` | `DataSchemaContext::format` | `Option<String>` | All schemas | No | No | None | N/A | covered | Represented. |
| `type` | `DataSchemaContext::data_type` | `Option<String>` | All schemas | No | No | None | N/A | covered | Represented as string. Deserialization prefers the explicit `type` field for concrete variant selection, and Basic validation rejects type-to-variant mismatches. |
| `items` | `ArraySchema::items` | `Option<Vec<DataSchema>>` | Array | No | Yes | None | Via nested schemas | covered | OneOrMany represented. |
| `minItems` | `ArraySchema::min_items` | `Option<u32>` | Array | No | No | None | N/A | covered | Represented. |
| `maxItems` | `ArraySchema::max_items` | `Option<u32>` | Array | No | No | None | N/A | covered | Represented. |
| `minimum` | `NumberSchema::minimum`, `IntegerSchema::minimum` | `Option<f64>`, `Option<i64>` | Number, Integer | No | No | None | N/A | covered | Represented. |
| `exclusiveMinimum` | `NumberSchema::exclusive_minimum`, `IntegerSchema::exclusive_minimum` | `Option<f64>`, `Option<i64>` | Number, Integer | No | No | None | N/A | covered | Represented. |
| `maximum` | `NumberSchema::maximum`, `IntegerSchema::maximum` | `Option<f64>`, `Option<i64>` | Number, Integer | No | No | None | N/A | covered | Represented. |
| `exclusiveMaximum` | `NumberSchema::exclusive_maximum`, `IntegerSchema::exclusive_maximum` | `Option<f64>`, `Option<i64>` | Number, Integer | No | No | None | N/A | covered | Represented. |
| `multipleOf` | `NumberSchema::multiple_of`, `IntegerSchema::multiple_of` | `Option<f64>`, `Option<i64>` | Number, Integer | No | No | None | N/A | covered | Builders preserve the provided value, and Basic validation rejects non-positive typed or preserved extension-map `multipleOf` values. |
| `properties` | `ObjectSchema::properties` | `Option<BTreeMap<String, DataSchema>>` | Object | No | No | None | Via nested schemas | covered | Represented. |
| `required` | `ObjectSchema::required` | `Option<Vec<String>>` | Object | No | No | None | N/A | covered | Represented. |
| `minLength` | `StringSchema::min_length` | `Option<u32>` | String | No | No | None | N/A | covered | Represented. |
| `maxLength` | `StringSchema::max_length` | `Option<u32>` | String | No | No | None | N/A | covered | Represented. |
| `pattern` | `StringSchema::pattern` | `Option<String>` | String | No | No | None | N/A | covered | Represented. |
| `contentEncoding` | `StringSchema::content_encoding` | `Option<String>` | String | No | No | None | N/A | covered | Represented. |
| `contentMediaType` | `StringSchema::content_media_type` | `Option<String>` | String | No | No | None | N/A | covered | Represented. |
| Unknown schema terms | `DataSchemaContext::_extra_fields` | `ExtensionMap` | All schemas | No | N/A | N/A | Yes | covered | Unknown schema fields are preserved. |

## SecurityScheme

| TD term | Rust field | Rust type | Applies to | Required | OneOrMany | Defaults | Extensions | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `@type` | `SecuritySchemeContext::tags` | `Option<Vec<String>>` | All schemes | No | Yes | None | N/A | covered | Represented. |
| `description` | `SecuritySchemeContext::description` | `Option<String>` | All schemes | No | No | None | N/A | covered | Represented. |
| `descriptions` | `SecuritySchemeContext::descriptions` | `Option<MultiLanguage>` | All schemes | No | No | None | N/A | covered | Represented. |
| `proxy` | `SecuritySchemeContext::proxy` | `Option<AbsoluteUri>` | All schemes | No | No | None | N/A | covered | Absolute URI enforced. |
| `scheme` | `SecuritySchemeContext::scheme` | `String` | All schemes | Yes | No | Scheme-specific builder default | N/A | covered | Represented and used as the discriminator for scheme-directed deserialization into the concrete security scheme variant. |
| `oneOf` | `ComboSecurityScheme::one_of` | `Vec<String>` | Combo | Conditional | No | Empty vector | N/A | covered | Represented. Basic validation checks minimum cardinality, empty references, and references against `securityDefinitions`. |
| `allOf` | `ComboSecurityScheme::all_of` | `Vec<String>` | Combo | Conditional | No | Empty vector | N/A | covered | Represented. Basic validation checks minimum cardinality, empty references, and references against `securityDefinitions`. |
| `name` | `name` fields | `Option<String>` | Basic, Digest, APIKey, Bearer | Conditional | No | None | N/A | partial | Represented. Basic validation checks API key `name`; additional scheme-specific name requirements remain future work. |
| `in` | `location` fields | `SecurityLocation` | Basic, Digest, APIKey, Bearer | Conditional | No | `header` | N/A | covered | Represented with default. |
| `qop` | `DigestSecurityScheme::qop` | `Qop` | Digest | No | No | `auth` | N/A | covered | Represented. |
| `authorization` | `BearerSecurityScheme::authorization`, `OAuth2SecurityScheme::authorization` | `Option<AbsoluteUri>` | Bearer, OAuth2 | Conditional | No | None | N/A | covered | Absolute URI enforced. Basic validation requires OAuth2 code-flow authorization endpoints. |
| `alg` | `BearerSecurityScheme::alg` | `String` | Bearer | No | No | `ES256` | N/A | covered | Represented with default. |
| `format` | `BearerSecurityScheme::format` | `String` | Bearer | No | No | `jwt` | N/A | covered | Represented with default. |
| `identity` | `PSKSecurityScheme::identity` | `Option<String>` | PSK | No | No | None | N/A | covered | Represented. |
| `token` | `OAuth2SecurityScheme::token` | `Option<AbsoluteUri>` | OAuth2 | Conditional | No | None | N/A | covered | Absolute URI enforced. Basic validation requires OAuth2 code-flow token endpoints. |
| `refresh` | `OAuth2SecurityScheme::refresh` | `Option<AbsoluteUri>` | OAuth2 | No | No | None | N/A | covered | Absolute URI enforced. |
| `scopes` | `OAuth2SecurityScheme::scopes` | `Option<Vec<String>>` | OAuth2 | No | Yes | None | N/A | covered | OneOrMany represented. |
| `flow` | `OAuth2SecurityScheme::flow` | `String` | OAuth2 | Yes | No | Builder requires value | N/A | covered | Represented. Basic validation accepts `code`, `client`, and `device`, and rejects unsupported flow names. |
| Unknown security terms | `SecuritySchemeContext::_extra_fields` | `ExtensionMap` | All schemes | No | N/A | N/A | Yes | covered | Unknown security fields are preserved. |

## ExpectedResponse and AdditionalExpectedResponse

| TD term | Rust field | Rust type | Required | OneOrMany | Defaults | Extensions | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `contentType` | `ExpectedResponse::content_type` | `String` | Yes | No | None | N/A | covered | Primary response content type represented. |
| Unknown expected response terms | `ExpectedResponse::_extra_fields` | `ExtensionMap` | No | N/A | N/A | Yes | covered | Unknown fields are preserved. |
| `contentType` | `AdditionalExpectedResponse::content_type` | `Option<String>` | No | No | Form content type | N/A | partial | Default inheritance from parent form is not resolved yet. |
| `schema` | `AdditionalExpectedResponse::schema` | `Option<String>` | No | No | None | N/A | partial | Represented as schema definition name. Validation against `schemaDefinitions` is pending. |
| `success` | `AdditionalExpectedResponse::success` | `bool` | No | No | `false` | N/A | covered | Flexible bool deserializer. |
| Unknown additional response terms | `AdditionalExpectedResponse::_extra_fields` | `ExtensionMap` | No | N/A | N/A | Yes | covered | Unknown fields are preserved. |

## Follow-Up Tasks

- Add Profile and Full validation rules beyond the current Basic checks,
  including standard-context requirements and Profile-specific form
  constraints.
- Validate remaining scheme-specific security details that are not covered by
  the current API key, combo, and OAuth2 checks.
- Add default inheritance helpers for `additionalResponses.contentType` and
  validate `additionalResponses.schema` references against schema definitions
  when a strict validation level requires it.
- Keep fixtures aligned with downstream runtime contracts, especially
  property-level extension preservation, Clinkz JSON-LD context aliases,
  multiple forms per affordance, and compact `OneOrMany` round trips.
