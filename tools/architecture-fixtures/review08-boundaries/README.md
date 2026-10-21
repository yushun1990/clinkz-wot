# Review 08 constructibility sketches

Non-production, isolated workspace for the three disputed boundaries in
[Review 08](../../../docs/reviews/review-08-integrated-architecture-reassessment.md).
The review owns the selected candidate, alternatives and authority impact.
These small stand-ins do not alter the repository's active dependency graph,
SPI, admission, gate or work-package contracts. There are no registry dependencies,
build scripts, cache, universal SPI, production Binding or CI gate here.

| Crate | What reviewers can inspect |
| --- | --- |
| [core](core/src/lib.rs) | TD-free vocabulary, generic definition/bound data, read-only registration tokens, scoped permits and narrow runtime/candidate interfaces. |
| [td](td/src/lib.rs) | TD-owned semantic input stand-ins using Core operations; deliberately no parser or second semantic kernel. |
| [planning](planning/src/lib.rs) | Public associated-type compiler/cursor/bind contract importing TD views; runtime output types remain Core-owned and native program types remain Binding-owned. |
| [binding](binding/src/lib.rs) | Owned non-Copy program; shared UDP socket/retirement. [Preparation](binding/src/prepare.rs) is optional. The separate [matcher](binding/src/matching.rs) owns native filters, captures, zero/many candidates and shared decoded packets. |
| [engine](engine/src/lib.rs) | Exact-unicast adapter plus [recipient dispatcher](engine/src/matching.rs). Owns logical record/handler indexing, current authorization, publication, per-recipient delivery and generation checks. Depends only on Core; Binding is a test dependency. |

From the repository root:

```sh
cargo test --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml
cargo check --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml -p review08-core -p review08-td -p review08-planning --target thumbv6m-none-eabi
cargo tree --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml -p review08-engine --edges normal --no-default-features
cargo tree --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml -p review08-third-party-binding --edges normal --no-default-features
cargo tree --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml -p review08-third-party-binding --edges normal --features prepare
cargo fmt --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml --all --check
```

Expected: six scenario tests plus two compile-fail permit doctests pass. Normal
runtime trees are `Engine -> Core` and `Binding -> Core`. Preparation adds
`Binding -> Planning -> TD -> Core` and direct Core/TD imports. Cargo checks the
acyclic graph in actual distinct crates; no module-level assumption substitutes
for a dependency edge. The target check requires the installed Cortex-M0 stdlib.

[The scenario corpus](engine/tests/scenarios.rs) exercises:

- One compiler start, zero-credit retention, two credited steps and one owned
  definition; many checked binds, source/compiler destruction, actual dependency
  and policy mismatch refusal, illegal operand refusal and independent grants.
- One socket for two Producer routes; no prepublication or revoked dispatch;
  A's stop preserves B's actual receive/response; A's accepted reply outlives
  both route records and keeps program data alive; final program drop occurs once.
- Pending native retirement until the final observation/reply/job releases;
  injected send failure retaining the complete delivery; exactly one socket
  acquisition/retirement despite extra inert root references.
- An early poll from a clone after A stops preserves B's actual ingress/reply
  and replacement-route attachment. Only final route release initiates ordinary
  retirement; polls cannot request a whole-infrastructure shutdown.
- Foreign issuer, cross-route and old-generation permit rejection; generation
  pinning prevents buffered input from targeting a replacement route.
- Private permit construction and scoped, non-retained lending checked by rustdoc.

The separate [matching corpus](engine/tests/matching.rs) covers overlapping
`meters/+/watts` / `meters/m1/#` and identical filters, captures and zero/many
candidates. Native filter interpretation stays in the Binding, with read-only
Engine tokens identifying distinct recipients. It checks shared packet release,
revocation, independent cancellation/removal, issuer/generation substitution and
per-recipient delivery saturation: B can progress while A retains one event.
Two registered recipients, two physical packet backings, two candidates per
packet, one accepted event per recipient, four topic levels and 32-byte topic/
payload limits bound this experiment. Aggregate packet pressure refuses another
packet explicitly. These are illustrative caps, not physical/account admission.
Current grants use principal/slot pairs; capture-sensitive security is not
implemented by this probe.

The UDP datagram carries only a native endpoint key (max 32 bytes); responses encode
a Boolean under the compiled mapping policy. Route/observation/delivery/job caps
are 2/1/2/1. Its equality-key/single-recipient index and collision rule are limited
to exclusive exact-unicast routes. The matching experiment separately injects
decoded native packet data; it is not connected to that UDP route API. Neither
is an MQTT client, broker session, QoS/acknowledgement implementation or WoT
subscription lifecycle. This avoids a thread/reactor, semantic defaulting, a
general extension tree and a complete TD validator.
The job token models an outstanding native callback's lifetime only. The test
uses actual UDP send/receive; it does not implement asynchronous SDK callbacks.

Limitations are material: `Rc`/`RefCell` is a single-thread Host adapter; the Core,
TD and Planning target check executes no firmware. There is no full schema/
security engine, physical admission/account implementation, allocator/OOM
qualification, scheduler fairness, concurrent publication, cancellation matrix,
cross-Servient infrastructure sharing,
complete Producer coverage proof, general program parameterization, trusted
prepared-image importer or async final-root successor. The credit counter stalls
step only; start/bind/finish allocate, so it does not prove funded construction,
recoverable failure or linked callback CPU bounds. Production bounds/start/bind/
step/abort/erasure retain predebit, actual backing, owner-preserving refusal and
bounded cleanup obligations. Public data envelopes and semantic stand-ins carry
no validation or install certificate.

Counters observe program destructors and actual socket owners, not exact bytes
or original accounts. Production must retain the backing/account owner at each
shared physical object, fund each reference/operand, and preserve those owners
through rollback, accepted work and retirement. A shared reference is insufficient
if an earlier instance releases its original allowance before physical cleanup.

The Binding has no detached work: synchronous local socket destruction qualifies
this sketch's final-root disposal only. Native Pending/error keeps the same owner
and its backing; no conclusion about an arbitrary async SDK follows. Program,
route, native participation and executor lifetimes remain separate.
