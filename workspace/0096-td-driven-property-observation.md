# TD-driven Property observation

Status: MIGRATED

## Boundary and evidence

Property observation is still absent from joined preparation/publication after
Producer and Consumer Event integration. The raw Consumer already distinguishes
PropertyObservation and forbids Event response/control payloads. The composed
Consumer definition, preparation and policy adapter nevertheless hardcode Event;
Producer source registration, captured services and native publication are also
Event-only. Reusing those APIs unchanged would misattribute Property values and
policy operations.

## Alternatives and selected direction

1. Pretend a Property is an Event: rejected; wrong schema and authorization roles.
2. Add an independent observation kernel or duplicate publication ownership:
   rejected; establishment/stop and publication effect/retirement already have
   useful executable owners.
3. Extend those narrow capabilities with explicit Property/Event discriminants:
   selected. Consumer schemas distinguish the Property value from all four Event
   roles. Producer notification sources/data retain their family through actual
   mapping and captured services; the publication owner shares only admission,
   effect and retirement mechanics. No finite Read or session facade is involved.

The concrete MQTT mapping follows the W3C MQTT binding's observeproperty ->
SUBSCRIBE and unobserveproperty -> UNSUBSCRIBE mapping. That binding is evolving
guidance, not a W3C Recommendation. Non-retained QoS 0 supports future values;
it supplies neither a Read nor an initial value. Exact-topic native sharing is
compatible across families but cannot change either logical target or schema.
The initial subset reports bounded loss or terminates; it does not coalesce.
HTTP SSE, retained snapshots, collections, QoS 1/2, TLS and general schema/security
qualification are outside this increment.

## Affected authority

`EXEC-NOTIFY-001`, Planning, Binding SPI, runtime safety, WP-200/300/400/600,
API ownership and PLAN capability coverage. Existing Event and raw notification
evidence must continue passing; real Engine roles must exercise Property values,
empty establishment, mapped unobserve, authority, failure and complete cleanup.

## Disposition

The selected correction is implemented in Consumer schema/preparation/policy,
Producer source/mapping/publication and the MQTT capability, with affected domain
contracts and API ownership updated. MQTT Property Form extensions use Property
semantic coordinates; the former Event-only scope lookup refused real Property
TDs and is corrected. Real broker fixtures exercise Property values in both Engine
roles and reaffirm Event behavior; the external Consumer fixture also prepares a
Property through the public SPI. Current evidence is linked from PLAN and WP-600.
No package/gate/milestone closure or broader production claim is made.
