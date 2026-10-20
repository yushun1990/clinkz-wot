# Review 08 constructibility sketches

Non-production, isolated workspace for the three disputed boundaries in
[Review 08](../../../docs/reviews/review-08-integrated-architecture-reassessment.md).
The review owns the selected candidate, alternatives and authority impact.
These small stand-ins do not alter the repository's active dependency graph,
SPI, admission, gate or work-package contracts. There are no registry dependencies,
build scripts, cache, universal SPI, production Binding or CI gate here.

| Crate | What reviewers can inspect |
| --- | --- |
| [core](core/src/lib.rs) | TD-free vocabulary, generic definition/bound data, captured authority, scoped permit and one-operation runtime interfaces. |
| [td](td/src/lib.rs) | TD-owned semantic input stand-ins using Core operations; deliberately no parser or second semantic kernel. |
| [planning](planning/src/lib.rs) | Public associated-type compiler/cursor/bind contract importing TD views; runtime output types remain Core-owned and native program types remain Binding-owned. |
| [binding](binding/src/lib.rs) | Owned non-Copy program; one loopback UDP socket shared by two route leases; observation/reply/job lifetimes; retained retirement owner. [Preparation](binding/src/prepare.rs) is an optional feature. |
| [engine](engine/src/lib.rs) | Runtime depends only on Core. Owns authorization, route/handler indexing, generation-qualified queued decisions, publication and dispatch. Binding is only a test dependency. |

From the repository root:

```sh
cargo test --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml
cargo check --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml -p review08-core -p review08-td -p review08-planning --target thumbv6m-none-eabi
cargo tree --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml -p review08-engine --edges normal --no-default-features
cargo tree --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml -p review08-third-party-binding --edges normal --no-default-features
cargo tree --locked --offline --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml -p review08-third-party-binding --edges normal --features prepare
cargo fmt --manifest-path tools/architecture-fixtures/review08-boundaries/Cargo.toml --all --check
```

Expected: three scenario tests plus two compile-fail permit doctests pass. Normal
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
- Foreign issuer, cross-route and old-generation permit rejection; generation
  pinning prevents buffered input from targeting a replacement route.
- Private permit construction and scoped, non-retained lending checked by rustdoc.

The datagram carries only a native endpoint key (max 32 bytes); responses encode
a Boolean under the compiled mapping policy. Route/observation/delivery/job caps
are 2/1/2/1. This deliberately avoids HTTP, a broker SDK, a thread or reactor,
semantic defaulting, a general extension tree and a complete TD validator.
The job token models an outstanding native callback's lifetime only. The test
uses actual UDP send/receive; it does not implement asynchronous SDK callbacks.

Limitations are material: `Rc`/`RefCell` is a single-thread Host adapter; the Core,
TD and Planning target check executes no firmware. There is no full schema/
security engine, physical admission/account implementation, allocator/OOM
qualification, scheduler fairness, concurrent publication, cancellation matrix,
cross-Servient infrastructure sharing,
complete Producer coverage proof, general program parameterization, trusted
prepared-image importer or async final-root successor. The credit counter is
illustrative and does not bound linked callback CPU time. Public data envelopes
and semantic stand-ins carry no validation or install certificate.

Counters observe program destructors and actual socket owners, not exact bytes
or original accounts. Production must retain the backing/account owner at each
shared physical object, fund each reference/operand, and preserve those owners
through rollback, accepted work and retirement. A shared reference is insufficient
if an earlier instance releases its original allowance before physical cleanup.

The Binding has no detached work: synchronous local socket destruction qualifies
this sketch's final-root disposal only. Native Pending/error keeps the same owner
and its backing; no conclusion about an arbitrary async SDK follows. Program,
route, native participation and executor lifetimes remain separate.
