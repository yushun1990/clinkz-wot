# Protocol Bindings

## Protocol-Neutral Engine

`clinkz-wot` is a protocol-neutral WoT engine.

The engine core must not prefer zenoh, HTTP, CoAP, MQTT, Modbus, BLE, or any other protocol. Protocol choice is deployment policy and should be expressed through TD forms and binding configuration.

## Binding Model

Every protocol binding consumes TD forms and maps them to concrete transport behavior.

Relevant form fields include:

- `href`
- `op`
- `contentType`
- `contentCoding`
- `subprotocol`
- `security`
- `scopes`
- protocol-specific extension terms

Bindings must use the same protocol-neutral trait surface.

## Crate Organization

Protocol binding crates are grouped under `protocol-bindings/`:

- `core`: shared protocol binding utilities published as
  `clinkz-wot-protocol-bindings`.
- `protocols/zenoh`: the concrete zenoh binding published as
  `clinkz-wot-protocol-bindings-zenoh`.

The shared protocol binding crate owns form selection, affordance form lookup,
and target resolution helpers. Concrete protocol crates own transport-specific
metadata parsing and operation mapping.

## Zenoh Binding

Zenoh is the first implemented binding because Clinkz Platform uses zenoh as its default communication bus.

Zenoh is not a required dependency of the engine. It belongs in
`clinkz-wot-protocol-bindings-zenoh` or an equivalent optional crate.

Expected operation mapping:

- Property read maps to zenoh query or get behavior.
- Property write maps to zenoh put or query-with-reply behavior.
- Property observe maps to zenoh subscribe behavior.
- Action invoke maps to request/reply behavior.
- Event subscribe maps to zenoh subscribe behavior.
- Bulk operations map to key-expression based group operations where appropriate.

## Clinkz Extension Namespace

Clinkz-specific binding terms should use a JSON-LD namespace such as:

```json
{
  "cz": "https://clinkz.io/wot#"
}
```

Zenoh-specific terms may use a more specific namespace if needed:

```json
{
  "cz-zenoh": "https://clinkz.io/wot/zenoh#"
}
```

Examples of extension terms:

- `cz-zenoh:keyExpr`
- `cz-zenoh:qos`
- `cz-zenoh:encoding`
- `cz-zenoh:priority`
- `cz-zenoh:congestionControl`

The exact vocabulary should be versioned and documented before being used in stable TDs.

## Future Bindings

Future bindings should use the same core traits:

- HTTP
- CoAP
- MQTT
- Modbus TCP
- Modbus RTU
- BLE
- OPC UA
- Custom industrial protocols

Zenoh may also be used as a bridge or replacement transport for constrained or legacy environments when the deployment makes that appropriate. This is a platform choice, not an engine-level assumption.
