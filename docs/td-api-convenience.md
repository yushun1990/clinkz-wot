# TD API Convenience Surface

## Current Shape

`clinkz-wot-td` exposes complete TD/TM data structures and builder APIs, but
some common construction flows still require verbose enum wrapping or manual
reference wiring.

The main ergonomic gaps found in the current API were:

- Security schemes had concrete builders, but callers had to wrap concrete
  schemes into `SecurityScheme` enum variants manually.
- `nosec` required a verbose builder plus enum variant expression.
- `ThingBuilder::security` inferred the security definition name from the
  scheme term, which is convenient for simple examples but does not express the
  TD pattern of custom definition names referenced by `security`.
- Data schema builders produced concrete schema structs, while affordance and
  definition APIs commonly expect `DataSchema`.
- Thing Model had named security helpers but no `nosec` shortcut.

## Added Convenience APIs

Security scheme helpers:

- `SecurityScheme::nosec()`
- `SecurityScheme::auto()`
- `SecurityScheme::basic(name)`
- `SecurityScheme::apikey(name)`
- `SecurityScheme::digest(name)`
- `SecurityScheme::bearer(name)`
- `SecurityScheme::bearer_authorization(name, authorization)`
- `SecurityScheme::psk(identity)`
- `SecurityScheme::combo_one_of(names)`
- `SecurityScheme::combo_all_of(names)`
- `SecurityScheme::oauth2(flow)`
- `SecurityScheme::oauth2_code(authorization, token)`
- `SecurityScheme::oauth2_client()`
- `SecurityScheme::oauth2_device()`
- `From<ConcreteSecurityScheme> for SecurityScheme`

Thing helpers:

- `ThingBuilder::security_name(name)`
- `ThingBuilder::security_definition(name, scheme)`
- `ThingBuilder::security_named(name, scheme)`
- `ThingBuilder::nosec()`
- `ThingBuilder::basic_security(name, parameter)`
- `ThingBuilder::apikey_security(name, parameter)`
- `ThingBuilder::digest_security(name, parameter)`
- `ThingBuilder::bearer_security(name, parameter)`
- `ThingBuilder::bearer_authorization_security(name, parameter, authorization)`
- `ThingBuilder::psk_security(name, identity)`
- `ThingBuilder::combo_one_of_security(name, names)`
- `ThingBuilder::combo_all_of_security(name, names)`
- `ThingBuilder::oauth2_code_security(name, authorization, token)`
- `ThingBuilder::oauth2_client_security(name)`
- `ThingBuilder::oauth2_device_security(name)`

Thing Model helpers:

- `ThingModelBuilder::nosec()`
- `ThingModelBuilder::security_named(name, scheme)`
- `ThingModelBuilder::basic_security(name, parameter)`
- `ThingModelBuilder::apikey_security(name, parameter)`
- `ThingModelBuilder::digest_security(name, parameter)`
- `ThingModelBuilder::bearer_security(name, parameter)`
- `ThingModelBuilder::bearer_authorization_security(name, parameter, authorization)`
- `ThingModelBuilder::psk_security(name, identity)`
- `ThingModelBuilder::combo_one_of_security(name, names)`
- `ThingModelBuilder::combo_all_of_security(name, names)`
- `ThingModelBuilder::oauth2_code_security(name, authorization, token)`
- `ThingModelBuilder::oauth2_client_security(name)`
- `ThingModelBuilder::oauth2_device_security(name)`

Data schema helpers:

- `From<ConcreteSchema> for DataSchema`
- `From<ConcreteSchemaBuilder> for DataSchema`
- Common schema receiver methods now accept `impl Into<DataSchema>`, including
  schema definitions, URI variables, action input/output, event schemas, object
  properties, array items, and `oneOf`.

Form operation helpers:

- `Form::read_property(href)`
- `Form::write_property(href)`
- `Form::invoke_action(href)`
- `Form::subscribe_event(href)`
- `FormBuilder` operation methods for all TD 1.1 operation enum values,
  including property, action, event, and Thing-level meta-interaction
  operations.

## Follow-Up Candidates

Useful next API improvements:

- Add affordance shortcuts for common schemas, for example `PropertyAffordance::string()`.
- Add examples that build a minimal TD, a named-secured TD, and a TM using only
  convenience APIs.
