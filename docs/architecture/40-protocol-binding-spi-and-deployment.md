# Binding execution and deployment — v6.0

[Binding SPI](../spec/binding-spi.md) owns detailed contracts. The common boundary
expresses WoT intent, a selected mapping, current permission, owned work and
observable outcomes. It prescribes no common network request/session/task model.

Finite interactions, notification conversations, Producer exposure and Discovery
have distinct controls. A private native driver may implement several facets;
a registration need not supply dummy roles, compilers or servers. Third-party
Cargo crates can supply complete conforming registrations without Engine edits
or workspace membership. The application links and registers them at startup;
one Servient's registration set is immutable after construction. Deployment
owns a new-instance drain/cutover; no live code unloading is selected.

Bindings own native protocol vocabulary interpretation, resource acquisition,
ingress, addressing/matching, captured variables, authentication integration,
correlation, flow control, distribution and physical retirement. Native receiving
and matching progress independently of semantic acceptance. Servient independently
checks each logical candidate and alone chooses application behavior.

A logical role does not dictate network direction. A Producer can connect to a
broker; a Consumer conversation can require a callback endpoint. A Thing is not
one device, listener, connection or native subscription. Shared infrastructure
has one physical owner and explicit compatible leases. Logical close releases
only its participation. Final destruction waits for all actual packets/replies/
callbacks/jobs; idle pools and reconnect are separately bounded owner policies.
A participation clone or a poll method grants no infrastructure shutdown right.

There is no Engine-native universal RPC, session manager, topic matcher, message
broadcast/copy requirement or unique-replier/aggregation protocol. Exclusive
request endpoint ambiguity must be refused; native multicast can have independent
recipients. The actual mapping declares response opportunities/cardinality and
must support the promised semantic topology before acceptance.

Host and constrained profiles share supported semantic guarantees, using their
own storage, synchronization and progress. Host erasure/Arc/executors and static
associated state/caller-owned slots are choices. `std`, async syntax, allocation,
thread mobility, atomics, dynamic preparation and backend/hardware support are
independent axes. No protocol quota proves architectural generality; concrete
capability evidence proves only the tested mapping/profile.
