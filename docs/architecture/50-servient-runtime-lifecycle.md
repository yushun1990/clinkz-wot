# Servient logical runtime — v6.0

Servient owns live logical instances and current execution authority, not native
network topology. It composes optional preparation, security/validation services
and Binding capabilities without interpreting protocols.

## Instance and acceptance

A local reference is qualified by its issuer/registry, slot, generation and
particular admitted obligation, or an equivalent lifetime proof. Neither a TD id,
endpoint, Rust type, content digest nor copied generation authorizes a call.
Several instances may share one description or physical runtime independently.
One logical instance can also participate in several independent Binding
runtimes under that same authority and captured behavior owner. Native runtime
retirement is not logical revocation; it cannot revoke a sibling's acceptance.

Consumer use selects a prepared candidate, checks policy/data/capacity and owns
the entire conversation/result/cleanup obligation before effects. Deferred pure
preparation belongs to a separate admitted coordinator. Completed construction
never creates current authority by itself.

Producer setup validates the private draft and actual behavior/providers, reserves
setup/ingress/finalization/cleanup, and may acquire closed native participation.
Actual endpoints finalize the effective TD. Complete final coverage/readiness
and cancellation/dependency checks precede one publication of description,
logical registry and current serving authority. No fallible native call occurs
inside that local commit. A deterministic support gap found later is a defect:
close affected acceptance and requalify, rather than quietly returning Unsupported
for a promise made at publication.

Native candidates retain original logical tokens/captures and backing. Servient
checks current identity, publication, policy, data and capacity independently per
recipient, captures behavior and reply capability, and dispatches outside locks.
Native ACK/control traffic is not application acceptance. Pending/late work never
looks up a replacement handler or maps an old packet into a successor slot.

## Distinct lifetimes

Action invocation state can outlive its initiating exchange. Logical notification
conversations own establishment/control/delivery/stop and reciprocal responses
where promised. Producer sources can exist with zero subscribers. Native broker
subscribers are not necessarily Engine-owned logical subscriptions. Details are
owned by [Interaction Core](../spec/interaction-core.md).

Destroy/replacement revokes new acceptance first, then cancels/drains accepted
owners. Logical settlement, local release, remote residue and infrastructure
failure/retirement are separately observable. No transition label proves another
owner's release. Shared native participants retain their siblings' leases.
Explicit infrastructure shutdown belongs to its authorized physical owner and
revokes/drains all dependents it affects.

[Runtime Safety](../spec/runtime-safety.md) owns cancellation/cleanup/publication;
[state-machines.toml](../state-machines.toml) projects semantic boundaries, not
mandatory public guard families. It intentionally does not prescribe the old
prepared/active/committed route ABI, fixed scheduler lanes or per-family copies
of the same resource framework.
