# W3C WoT Compliance Notes

## Baseline

The default compliance target is W3C Web of Things TD 1.1, Architecture 1.1, Discovery, and Profile.

TD 2.0 is tracked as experimental work only. Do not make TD 2.0 behavior part of the default public API until the specification stabilizes.

## Thing Description

A Thing Description is the machine-readable contract for a Thing. It describes identity, metadata, interaction affordances, security metadata, protocol forms, links, schemas, and extension vocabulary.

TD parsing should be tolerant enough to preserve unknown terms. Validation should be a separate explicit step.

## Thing Model

Thing Model should be used to reduce authoring repetition and define reusable templates.

The intended workflow is:

1. Authors define reusable TM templates.
2. Platform tooling instantiates concrete TDs from those templates.
3. Consumers receive fully usable TDs with concrete forms.

This keeps TDs precise for consumers while avoiding repetitive hand-authored JSON.

## Forms and `base`

In TD, a form describes how to perform an operation. A form target is provided by `href`.

Using a Thing-level `base` reduces repetition:

```json
{
  "base": "zenoh://clinkz/gateways/gw001/",
  "properties": {
    "temperature": {
      "type": "number",
      "forms": [
        {
          "href": "properties/temperature",
          "op": ["readproperty", "observeproperty"]
        }
      ]
    }
  }
}
```

The runtime or binding layer should resolve the target as:

```text
zenoh://clinkz/gateways/gw001/properties/temperature
```

The TD crate should store both `base` and `href`. Shared resolution logic should be added so bindings do not each implement their own resolution behavior.

## URI Templates

URI templates are useful when one affordance exposes parameterized resources.

For fully enumerated properties, each property should still have an explicit form, but the form can use a short relative `href` and default metadata.

## Defaults

TD authors and generators should avoid redundant fields when defaults are clear.

Examples:

- `contentType` defaults to `application/json`.
- Thing-level `security` can be inherited by forms unless a form overrides it.
- Default operation behavior should be resolved according to TD rules and interaction type.

## JSON-LD Contexts

The `@context` field defines the semantic vocabulary used by a TD.

The standard WoT TD context should always be present. Extension vocabularies should be explicitly declared with their own namespace prefixes.
