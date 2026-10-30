# TD-driven Producer Events over native distribution

Status: MIGRATED

The #244 roadmap identifies no qualified Producer Event path. Existing finite
requests, continuing Actions and Consumer notification conversations cannot
represent an application emitting an Event independently of subscribers.

Keep v6 responsibility boundaries and extend whole-Thing qualification with
registered Event sources and publication-captured mappings, validation and
security services. Give each admitted emission its own bounded native
publication, cancellation, settlement and acknowledgement lifetime. Do not
translate Events into Actions, finite requests or Engine subscriber fan-out.

MQTT tests the unproved distribution boundary: one QoS 0 PUBLISH can reach zero
or many peers through broker-owned matching. A native service may share its one
connection across independently published Things. A deliberately limited Host
mapping accepts JSON/type-only data, NoSec loopback, exact topics and native
subscribe/unsubscribe without application control payloads. Reject dataResponse,
unsupported Forms, response semantics and security instead of weakening the TD.
Local publication success says nothing about subscriber delivery.

HTTP SSE would exercise another individual stream adapter and is unnecessary
for this selected capability. A bespoke UDP broadcaster would add vocabulary
and matching rules without an interoperability need. MQTT QoS 1/2, reconnect,
replay and request/response correlations add commitments this first slice does
not need. Use MQTT 3.1.1 framing with a bounded single publication slot, and
Mosquitto plus independent wire subscribers as executable evidence.

Final contracts belong in the existing Core/Binding/Planning/runtime owners;
the MQTT README owns the concrete supported subset. Independent review must
check real broker delivery, complete final qualification, source-independent
execution, shared connection lifetime, refusal, cancellation, replacement,
uncertain partial writes, unread results and final release. No gate or milestone
acceptance is implied by author validation.
