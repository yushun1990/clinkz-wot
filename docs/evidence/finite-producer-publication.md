# Finite Producer publication and reply retirement

The Host application registers finite services once, then passes its TD, behaviors
and exposure policy to `HostProducer::expose`. The application polls publication
with its clock. Before acquiring an assigned endpoint, the candidate validates
the draft TD and matches its advertised operations to the application behaviors.
At the actual base, Planning selects a unique registration for every Form and
operation and checks its definition against all competitors. The candidate pays
for each provider visit across these passes and retries. Only selected services consume
member positions. The existing Servient commit checks the captured facts and
opens the ready members together. A refused candidate releases partial native
setup and returns its original inputs. [Host implementation](../../servient/src/host_producer.rs) ·
[HTTP application](../../protocol-bindings/protocols/http/examples/serve_properties.rs)

A periodic Host turn visits one member. Each exposure carries the last accepted
clock observation from publication through serving and retirement. A backwards
tick or clock-domain change fences that exposure before dispatch or delivery.
Each interrupted reply keeps its original result and any earlier cause.
Taking an outcome transfers its original
record and data. Disposing it frees a record for the next admission. A deployment
can share its outstanding-handler quota across compatible existing callers by
supplying the same bank in `HostProducerLimits::shared_handler_quota`. The HTTP participant owns one active connection per
lease. Its response shares the handler’s immutable payload and its JSON validator
traverses escaped keys and values in bounded borrowed storage.
[Finite result ownership](../../servient/finite/src/producer_capacity.rs) ·
[Native reply](../../protocol-bindings/protocols/http/src/producer_transport.rs) ·
[Installed validator measurements](../../protocol-bindings/protocols/http/tests/producer_installation.rs)

For a trusted final TD with one NoSec Property Read, `ReadPublication` borrows
the mapping, application behavior, logical slot and supplied exchange slots.
Each accepted offer keeps its original reply, deadline, validator scratch and
exclusive output loan. A failed admission returns the original offer. The
application polls each exchange by its key. Successful native retirement frees
that exchange’s slot, and the returned settlement holds its output loan through
disposal. The caller supplies time and drives pending work. Core carries the
same first-cause and cleanup-error decision for both deployments.
[Caller-owned Producer](../../servient/finite/src/borrowed_producer.rs) ·
[Shared retirement](../../core/src/execution/producer.rs)

| Case | Executable observation |
| --- | --- |
| Two assigned candidates contend for the last member | Both endpoints are acquired closed. The winner serves its advertised Read alongside the incumbent. The losing port can be rebound while its refusal is held, and its recovered handler can publish later. |
| Final membership and callback changes | One application serves HTTP, independently registered UDP, or both. Two Reads fit one selected member. A port-zero draft with two matches can resolve to one serving registration. Final ambiguity releases the candidate. Split membership, provider drift, stale or foreign drivers, cancellation and expiry are tested at their acquisition boundaries. |
| A held result exhausts capacity | An outcome taken from an assigned Host retains the last shared permit. A fixed Host can dispatch its waiting HTTP peer after disposal. Three nested Hosts defer sibling drives while revocation takes effect immediately. |
| Two accepted datagrams on one member | The first reply times out and its local retirement fails. The second delivers to its own peer first. Retrying the first releases the original reply exactly once and preserves both errors and its output. |
| Loss, replacement and late handoff | Explicit close records cancellation on outstanding Reads immediately and preserves any earlier timeout. A committed Host reports idle ingress failure through `publication_failure()`. Required ingress loss or loss of reply isolation fences the affected publication. Its pending replies drain and an independent endpoint serves. A prior result outlives replacement and keeps its original buffer. Revocation during delivery authorization suppresses the wire send. |

The Host cases use real loopback sockets and a separately implemented test
registration. The manual cases use a real UDP peer and typed per-exchange
storage. Constrained compilation covers the no-pointer-atomics target.
[Host cases](../../protocol-bindings/protocols/http/tests/producer_exposure.rs) ·
[Independent Host registration](../../protocol-bindings/protocols/http/tests/support/exposure_udp.rs) ·
[Manual cases](../../servient/finite/tests/borrowed_producer.rs)

```sh
cargo test --locked -p clinkz-wot-binding-http --test producer_exposure --test producer_installation
cargo test --locked -p clinkz-wot-binding-http --no-default-features
cargo test --locked -p clinkz-wot-servient-finite --no-default-features --test borrowed_producer
cargo check --locked -p clinkz-wot-servient-finite --no-default-features --target thumbv6m-none-eabi
```

The public Host offers ordinary admission. Advance availability depends on a
deployment requesting and supplying a reservation. Continuing Actions, MQTT
emissions and the mixed Host credit factory retain their own work-package
boundaries. [Servient work package](../work-packages/WP-400-servient.md#public-finite-publication-acceptance-and-retirement)
