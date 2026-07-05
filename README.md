# clinkz-wot

A protocol-neutral Rust Web of Things engine targeting **W3C WoT Scripting API
conformance** (Consumer, Producer, Discovery), running on both `std` and
`no_std + alloc`.

The engine uses W3C WoT Thing Descriptions (TD 1.1) as the semantic contract.
Protocol bindings are pluggable; **Zenoh** is the first concrete binding.

## v4.0 Architecture

The v4.0 baseline is a one-shot breaking refactor driven by three decisions:

1. **Full WoT Scripting API alignment** — the engine surfaces (`produce`/
   `consume`/`discover`/`fetch_td`, `set_*_handler`, `read_property`/
   `write_property`/`invoke_action`/`subscribe_event`, `expose`/`destroy`)
   follow the Scripting API method catalogue.
2. **Frozen TD at expose** — no dynamic affordance add/remove after `expose()`;
   handlers may be replaced throughout the exposed lifetime.
3. **Sync-primary handlers, async driving** — inbound handlers are synchronous
   (zero-allocation hot path); the driving/transport layer is async; `no_std`
   super-loops drive the same futures by manual polling.

### Key Types

| Type | Crate | Role |
| --- | --- | --- |
| `WotLock<T>` | `core` | Arc-backed `Clone`-able lock (`std::sync::RwLock` / `critical_section::Mutex`). |
| `ExposedThing` | `core` | Produced Thing + per-affordance handler sets (9 sync + 9 async traits). |
| `ConsumedThing` | `core` | Consumed Thing + registered `ClientBinding`s. |
| `Servient` | `servient` | Non-generic composition root: registries, bindings, fan-in channel, discoverer. |
| `ServientBuilder` | `servient` | Consuming fluent builder. |
| `InMemoryDirectory` | `discovery` | Reference directory backend (all 4 capability traits). |
| `ServerBinding` / `ClientBinding` | `core` | Inbound (`try_accept`/`send_response`/`register_thing`) / outbound (`async invoke`/`subscribe`). |

## Workspace Crates

| Crate | Role | `no_std` |
| --- | --- | --- |
| [`clinkz-wot-td`](td) | TD/TM data models, builders, serde, validation, URI helpers. | ✅ root |
| [`clinkz-wot-core`](core) | Interaction core: handler traits, `ExposedThing`/`ConsumedThing`, `WotLock`, `EventBroker`, `ServerBinding`/`ClientBinding`, `PushFn`. | ✅ root |
| [`clinkz-wot-discovery`](discovery) | Introduction→Exploration sessions, `DirectoryReader`/`Publisher`/`Watch`, `Discoverer`, `InMemoryDirectory`. | ✅ root |
| [`clinkz-wot-protocol-bindings`](protocol-bindings/core) | Shared form selection, op resolution, `error_status`, URI-template expansion. | ✅ root |
| [`clinkz-wot-protocol-bindings-zenoh`](protocol-bindings/protocols/zenoh) | Zenoh planning + async runtime (`zenoh` feature). | ✅ planning layer |
| [`clinkz-wot-servient`](servient) | `Servient` + `ServientBuilder` + driving (`poll_serve`/`serve`/`poll_serve_once`) + handles. | ✅ root |

## Quick Start

```sh
git clone git@github.com:yushun1990/clinkz-wot.git
cd clinkz-wot
cargo test --workspace          # all suites
cargo clippy --workspace --all-targets  # 0 warnings
scripts/check-baseline.sh       # fmt + test + clippy + no_std + feature-matrix
```

## Engine Usage

### 1. Build a Servient

```rust
use clinkz_wot_servient::{ServientBuilder, ClientBindingFactory, Servient};
use clinkz_wot_core::{ClientBinding, ServerBinding};
use clinkz_wot_discovery::{Discoverer, InMemoryDirectory, LocalDiscoverer};
use alloc::{boxed::Box, sync::Arc};

// A custom client binding factory (or use a provided one).
struct MyClientFactory;
impl ClientBindingFactory for MyClientFactory {
    fn build(&self) -> Box<dyn ClientBinding> {
        // Return your async ClientBinding impl.
        todo!()
    }
}

// Build the Servient.
let servient = ServientBuilder::new()
    .with_server_binding(my_server_binding)   // Arc<dyn ServerBinding>
    .with_client_factory(Arc::new(MyClientFactory))
    // .with_discoverer(custom)  // optional; defaults to LocalDiscoverer
    // .with_fanin_capacity(256) // optional inbound fan-in capacity
    .build()
    .expect("build servient");
```

`build()` constructs the bounded fan-in channel, injects `set_request_sink` +
`set_event_broker` into every server binding, and defaults the discoverer to a
`LocalDiscoverer` over a fresh `InMemoryDirectory`.

### 2. Produce + Expose a Thing (Producer)

```rust
use clinkz_wot_core::{InteractionInput, InteractionOutput, CoreError, PropertyReadHandler};

struct StatusRead;
impl PropertyReadHandler for StatusRead {
    fn read(&self, _input: &InteractionInput) -> Result<InteractionOutput, CoreError> {
        Ok(InteractionOutput::with_data(
            clinkz_wot_core::Payload::new(b"on".to_vec(), "text/plain"),
        ))
    }
}

// produce() creates a draft handle (not yet remotely servable).
let handle = servient.produce(lamp_td()).expect("produce");

// Attach handlers (replaceable throughout the lifetime — AD14).
handle.set_property_read_handler("status", StatusRead);
// handle.set_property_write_handler("status", ...);
// handle.set_action_handler("toggle", ...);
// handle.set_event_subscribe_handler("overheat", ...);

// expose() registers routes on all server bindings + inserts into the
// servable registry. TD is frozen after this.
handle.expose().await.expect("expose");

// Local server-side interactions.
let value = handle.read_property("status", &InteractionInput::empty())?;
handle.emit_event("overheat", payload)?;
```

### 3. Drive the Inbound Loop

```rust
// std (tokio): serve until shutdown.
let shutdown = servient.shutdown_handle();
tokio::spawn(async move {
    servient.clone().serve().await;
});

// ... or step-by-step.
servient.poll_serve().await?;  // dispatches ≤1 inbound request per call

// Graceful shutdown.
shutdown.shutdown();
```

**Bare `no_std` super-loop** (no executor):

```rust
loop {
    let waker = noop_waker();
    let mut cx = core::task::Context::from_waker(&waker);
    let _ = svc.poll_serve_once(&mut cx);  // ≤1 accept→dispatch→reply
    // ... other super-loop work
}
```

### 4. Consume a Remote Thing (Consumer)

```rust
// consume() builds a ConsumedThing with fresh client bindings.
let consumed = servient.consume(remote_td()).expect("consume");

// All methods are async — they drive the real ClientBinding.
let output = consumed.read_property("status", InteractionOptions::new()).await?;
let _ = consumed.invoke_action("toggle", InteractionOptions::new()).await?;
```

### 5. Discover Things

```rust
use clinkz_wot_discovery::DiscoveryFilter;

// discover() is synchronous and returns a lazy process (AD10).
let mut process = servient.discover(DiscoveryFilter::all());

// Real directory work happens inside the first next().
while let Some(thing) = process.next().await? {
    println!("found: {:?}", thing.id);
}
```

### 6. Destroy (Quiescing Teardown)

```rust
// destroy() is idempotent (AD27). Unregisters routes, drains in-flight,
// removes the registry entry.
handle.destroy().await.expect("destroy");
```

## Feature Flags

| Feature | Effect |
| --- | --- |
| `default = ["std"]` | std runtime + tokio driving. `std` implies `async`. |
| `alloc` | Dynamic data on `no_std`. |
| `std` | Networking, filesystem, async runtime, host conveniences. Implies `alloc` + `async`. |
| `async` | Native-async driving (`poll_serve`, `serve`). On `no_std` requires an executor (embassy). |
| `zenoh` | Rust `zenoh` std backend (real async consume + inbound). |
| `zenoh-pico` | Constrained `no_std+alloc` platform-hook backend (mutually exclusive with `zenoh`). |
| `td2-preview` | Experimental TD 2.0 fields. |

## Architecture Principles

- **Layering is non-negotiable.** Data contract (TD/TM) → interaction core →
  bindings → servient. Core knows nothing of concrete protocols.
- **`no_std + alloc` is the baseline contract.** Every crate whose
  responsibility permits it compiles `no_std + alloc`.
- **Stable unknown-field round-trip fidelity.** TD/TM documents are preserved
  verbatim through serde.
- **Sync-primary handlers** = zero-allocation inbound hot path. Async twins are
  opt-in for I/O-bound cloud handlers.
- **One lock primitive** — `WotLock<T>` (always thread-safe, `Clone`-able).
- **Scripting API alignment** — method catalogue + semantics, in Rust idiom;
  engineering concerns (performance, extensibility, code reasonableness) take
  priority over verbatim JS naming where they conflict.

## Verification

```sh
scripts/check-baseline.sh     # aggregate: fmt + test + clippy + no_std + feature-matrix
scripts/check-no-std.sh       # 7 crates bare no_std + 2 async no_std flavors
scripts/check-feature-matrix.sh  # 21 feature combinations
```

Zenoh runtime smoke tests are opt-in:

```sh
CLINKZ_WOT_RUN_ZENOH_RUNTIME_TESTS=1 \
  cargo test -p clinkz-wot-protocol-bindings-zenoh --features zenoh
```

## Documentation

- [Implementation plan](PLAN.md)
- [Engine architecture baseline (v4.0)](docs/baseline/engine-architecture-baseline.md)
- [Servient workflow diagrams](docs/servient-workflow.md)
- [Technical specification](docs/technical-spec.md)
- [WoT compliance notes](docs/wot-compliance.md)
- [no_std and embedded support](docs/no-std-embedded.md)
- [Discovery refactor plan](docs/plan/discovery-directory-refactor-plan.md)

## License

MIT. Portions derived from `wot-td`. See [LICENSES/MIT.txt](LICENSES/MIT.txt).
