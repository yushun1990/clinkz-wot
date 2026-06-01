# Clinkz Platform Context

## Platform Vision

Clinkz is a semantic IoT platform built around the Web of Things model.

In Clinkz, devices, databases, APIs, services, gateways, and compute tasks can all be represented as Things. TD and TM documents provide the semantic contract for these Things.

## Role of clinkz-wot

`clinkz-wot` is the WoT engine layer for the platform.

It provides:

- TD and TM modeling.
- Validation and serialization.
- Protocol-neutral operation dispatch.
- Protocol binding integration.
- Discovery integration.
- Servient runtime building blocks.

## Zenoh in Clinkz

Clinkz Platform uses zenoh as its default communication bus.

This platform choice does not make zenoh mandatory for the WoT engine. The engine remains protocol-neutral, and zenoh is implemented as the first optional binding.

Zenoh may be used to connect distributed storage, computation, gateways, and physical devices. It may also replace or front legacy protocols in deployments where that is beneficial.

## Thing Types

The platform should be able to model:

- Physical device Things.
- Gateway Things.
- Database Things.
- API Things.
- Compute Things.
- Virtual composite Things.
- Directory and registry Things.

All of these should use the same TD/TM foundation.

## Extension Policy

Clinkz-specific terms should be expressed through JSON-LD extension vocabulary.

The extension vocabulary should cover platform concepts such as:

- Distributed resource identifiers.
- Zenoh key expressions.
- Storage and query hints.
- Compute placement hints.
- Platform ownership and tenancy metadata.

These extensions must not replace standard WoT TD semantics when a standard term already exists.
