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
- `From<ConcreteSecurityScheme> for SecurityScheme`

Thing helpers:

- `ThingBuilder::security_name(name)`
- `ThingBuilder::security_definition(name, scheme)`
- `ThingBuilder::security_named(name, scheme)`
- `ThingBuilder::nosec()`
- `ThingBuilder::basic_security(name, parameter)`
- `ThingBuilder::apikey_security(name, parameter)`

Thing Model helpers:

- `ThingModelBuilder::nosec()`

Data schema helpers:

- `From<ConcreteSchema> for DataSchema`
- `From<ConcreteSchemaBuilder> for DataSchema`
- Selected schema receiver methods now accept `impl Into<DataSchema>`.

## Follow-Up Candidates

Useful next API improvements:

- Add `SecurityScheme` constructors for bearer, digest, PSK, combo, and OAuth2
  flows where the required fields are clear.
- Accept `impl Into<DataSchema>` consistently for all schema receiver methods,
  including interaction URI variables and batch object properties.
- Add form operation shortcuts such as `Form::read_property`, `Form::invoke_action`,
  or builder methods like `.read_property()` and `.invoke_action()`.
- Add affordance shortcuts for common schemas, for example `PropertyAffordance::string()`.
- Add examples that build a minimal TD, a named-secured TD, and a TM using only
  convenience APIs.
