# Executable TD Security and Host HTTPS

Status: MIGRATED

## Boundary challenge

Basis: v6.0, ADR-0025, EXEC-SECURITY-001, PREP-SELECT/REUSE/COVER-001,
SPI-NATIVE/INGRESS-001 and SAFE-AUTH/CANCEL/CLEANUP-001. Source inspected:
TD semantic SecurityFacts, open Consumer common qualification/content, finite
Producer compilation, finite dispatch/delivery, HTTP native transports.

TD already resolves Form override and typed/defaulted scheme operands. It does
not authenticate. Planning currently hardcodes NoSec before consulting actual
services; ConsumerExecutionContent lacks security meaning. Those concrete #260
assumptions cannot safely support authenticated checked reuse. The reusable
boundary remains useful, but must include immutable security requirements and
require actual service compatibility at every bind. Live credentials cannot be
part of a ConsumerDefinition or HttpProducerProgram.

Finite Producer dispatch currently trusts supplied Principal and delivery does
not recheck application authorization. A native reply opportunity is the natural
owner of unverified evidence, distinct from application input. Verification must
occur under paid work before dispatch and be repeated before delivery; identity
changes must refuse the original result. Binding callbacks receive no behavior
handle. Cancellation retains the original reply/evidence until local disposal.

## Selected increment and alternatives

Implement TD basic with default/header Authorization placement, finite Property
Read, HTTP/1.1 over authenticated TLS 1.3, loopback/IP endpoints, JSON. A Basic
verifier maps credentials to an independently established Principal; the handler
applies application policy. Client roots/IP SAN verification, TLS records and
Authorization transport stay in HTTP. No shared credential/session manager.

Core carries a small immutable scheme/parameter requirement and an opaque
per-call Binding capability. It has no Basic, HTTP, TLS or password cases. The
access service qualifies complete prepared requirements and issues a capability
only for the original instance/destination/operation. The Binding uses credentials
after verified TLS and repeats currentness checks. Source/compiler retirement
and independent instances must remain executable.

Reject putting a password/header into native prepared artifacts; reject treating
username or header possession as Principal; reject a universal authentication
registry/framework. A Producer-only raw TLS demo would not test checked Consumer
reuse or actual service pairing. Full WoT security composition is not needed for
this slice: multiple simultaneous requirements, combo allOf/oneOf and other
schemes/placements explicitly refuse; they must never select a weaker branch.

## Falsifiable boundary

Real qualified Producer and Engine Consumer exchange, independent TLS peer,
unsupported/security/service mismatches before effects, missing/wrong/revoked
credentials, untrusted/incorrect identity certificates, authorization policy
change, independent instances, replacement/revocation, cancellation and final
original resource release. No broad production, other role/profile, TLS resource
ceiling or full scheme/conformance claim follows from this evidence.

The selected conclusion is migrated to the Planning/Core/Binding contracts and
[the implementation assessment and scoped evidence](../docs/evidence/td-security-https.md).
