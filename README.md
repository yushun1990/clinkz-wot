# clinkz-wot

A protocol-neutral Rust Web of Things engine targeting **W3C WoT Scripting API
conformance** (Consumer, Producer, Discovery), running on both `std` and
`no_std + alloc`.

The engine uses W3C WoT Thing Descriptions (TD 1.1) as the semantic contract.
Protocol bindings are pluggable; **Zenoh** is the first concrete binding.

## v4.1 Architecture

The v4.1 baseline amends v4.0's binding ownership, lifecycle, and registration
model (AD55–AD58):

1. **Full WoT Scripting API alignment** — the engine surfaces (`produce`/
   `consume`/`discover`/`fetch_td`, `set_*_handler`/`set_async_*_handler`,
   `read_property`/`write_property`/`invoke_action`/`observe_property`/
   `subscribe_event`/`subscribe_all_events`/`read_all_properties`/
   `write_multiple_properties`, `expose`/`destroy`) follow the Scripting API
   method catalogue.
2. **Frozen TD at expose** — no dynamic affordance add/remove after `expose()`;
   handlers may be replaced throughout the exposed lifetime.
3. **Sync-primary handlers, binding-owned driving** — inbound handlers are
   synchronous (zero-allocation hot path); each binding owns its driving model
   (`serve()` spawns a draining task on std; `no_std` super-loops poll
   `try_accept`).
4. **Direct binding registration** (v4.1) — `ProtocolBinding` facade removed;
   `ServerBinding` and `ClientBinding` are registered directly via
   `ServientBuilder::with_server_binding` / `with_client_binding`. Bindings
   are owned by handles, not the Servient.

### Key Types

| Type | Crate | Role |
| --- | --- | --- |
| `WotLock<T>` | `core` | Arc-backed `Clone`-able lock (`std::sync::RwLock` / `critical_section::Mutex`). |
| `ExposedThing` | `core` | Produced Thing + per-affordance handler sets (9 sync + 9 async traits). |
| `ConsumedThing` | `core` | Consumed Thing + shared `Arc<dyn ClientBinding>` list. |
| `ServerBinding` | `core` | **Binding extension trait**: `serve(thing_id, td, ctx)` / `shutdown(thing_id)` lifecycle + `try_accept` / `send_response`. |
| `ClientBinding` | `core` | **Binding extension trait**: async `invoke` / `subscribe` outbound. Shared `Arc` across all consumed Things. |
| `ConsumedThingHandle` / `ExposedThingHandle` | `servient` | **App-facing** interaction surfaces (Scripting API §6/§7). Own their binding `Arc` references. |
| `Servient` | `servient` | Non-generic composition root: dispatch engine + discovery facade. Holds default bindings cloned into handles. |
| `ServientBuilder` | `servient` | Consuming fluent builder: `with_server_binding` + `with_client_binding`. |
| `InMemoryDirectory` | `discovery` | Reference directory backend (all 4 capability traits). |

## Workspace Crates

| Crate | Role | `no_std` |
| --- | --- | --- |
| [`clinkz-wot-td`](td) | TD/TM data models, builders, serde, validation, URI helpers. | ✅ root |
| [`clinkz-wot-core`](core) | Interaction core: handler traits, `ExposedThing`/`ConsumedThing`, `WotLock`, `EventBroker`, `ServerBinding`/`ClientBinding` (binding extension traits), `PushFn`. | ✅ root |
| [`clinkz-wot-discovery`](discovery) | Introduction→Exploration sessions, `DirectoryReader`/`Publisher`/`Watch`, `Discoverer`, `InMemoryDirectory`. | ✅ root |
| [`clinkz-wot-protocol-bindings`](protocol-bindings/core) | Shared form selection, op resolution, `error_status`, URI-template expansion. | ✅ root |
| [`clinkz-wot-protocol-bindings-zenoh`](protocol-bindings/protocols/zenoh) | Zenoh planning + async runtime + `shared()`/`client()`/`server()` constructors (`zenoh` feature). | ✅ planning layer |
| [`clinkz-wot-servient`](servient) | `Servient` + `ServientBuilder` + `ConsumedThingHandle`/`ExposedThingHandle`. Dispatch is binding-owned; the Servient exposes `Dispatch::serve_request` for bindings to call. | ✅ root |

## Quick Start

```sh
git clone git@github.com:yushun1990/clinkz-wot.git
cd clinkz-wot
cargo test --workspace          # all suites
cargo clippy --workspace --all-targets  # 0 warnings
scripts/check-baseline.sh       # fmt + test + clippy + no_std + feature-matrix
```

## Engine Usage

### Architecture: who does what

```
┌─────────────────────────────────────────────────────┐
│                    Servient                          │
│  produce / consume / discover / fetch_td             │
│  expose / destroy (lifecycle)                        │
│  dispatch(req) → handler → response    ← 唯一入口    │
│  (不关心谁来调、怎么调)                                │
└───────────────────┬─────────────────────────────────┘
                    │ Dispatch::serve_request(req).await
                    │
     ┌──────────────┼──────────────────┐
     │              │                  │
 zenoh binding   HTTP/CoAP binding   no_std binding
     │              │                  │
 自己跑 draining  route handler       super-loop
 task 从 channel  里直接调             poll try_accept
 drain → dispatch  serve_request       → dispatch
 → send_response                      → send_response
```

### 1. (Binding authors only) Implement ServerBinding / ClientBinding

> Application developers register bindings via `ServientBuilder`. This section
> is for binding authors adding a new protocol (HTTP, CoAP, MQTT, ...).

`ServerBinding` has explicit lifecycle: `serve()` declares routes AND starts
the driving model; `shutdown()` tears them down.

```rust
use clinkz_wot_core::{
    BindingContext, CoreResult, Dispatch, InboundRequest, InboundResponse,
    ServerBinding, ThingId,
};
use clinkz_wot_td::thing::Thing;
use alloc::sync::Arc;

struct MyServerBinding;

impl ServerBinding for MyServerBinding {
    fn serve(
        &self,
        thing_id: &ThingId,
        td: &Thing,
        ctx: &BindingContext,
    ) -> CoreResult<()> {
        // 1. Declare transport routes for this Thing based on td.
        // 2. On std: spawn a draining task that calls
        //    ctx.dispatch.serve_request(req).await then self.send_response(resp).
        // 3. On no_std: configure poll state; the super-loop calls try_accept().
        Ok(())
    }

    fn shutdown(&self, _thing_id: &ThingId) {
        // Undeclare routes, cancel background tasks.
    }

    fn try_accept(&self) -> Option<InboundRequest> {
        None // default; no_std bindings override for super-loop polling.
    }

    fn send_response(&self, _response: InboundResponse) {
        // Map InboundResponse back to the protocol reply.
    }
}
```

`ClientBinding` is stateless — all per-Thing context is in `BindingRequest`:

```rust
use clinkz_wot_core::{ClientBinding, BindingRequest, CoreResult, InteractionOutput};

#[async_trait::async_trait]
impl ClientBinding for MyClientBinding {
    fn supports(&self, form: &clinkz_wot_td::form::Form, op: clinkz_wot_td::data_type::Operation) -> bool {
        // Return true if this binding can drive the form's protocol scheme.
        todo!()
    }
    async fn invoke(&self, request: BindingRequest) -> CoreResult<InteractionOutput> {
        // Drive the real protocol (zenoh get/put, HTTP fetch, ...).
        todo!()
    }
}
```

### 2. Build a Servient (application entry point)

Application code registers `ServerBinding` and `ClientBinding` directly.
A two-direction binding (the common case) registers both from one shared
session:

```rust
use clinkz_wot_servient::ServientBuilder;
use clinkz_wot_protocol_bindings_zenoh as zenoh;
use std::sync::Arc;

let session = zenoh::open(config).await.unwrap();
let (server, client) = zenoh::shared(session);

let servient = ServientBuilder::new()
    .with_server_binding(server)       // Arc<dyn ServerBinding>
    .with_client_binding(client)       // Arc<dyn ClientBinding>
    // .with_discoverer(custom)        // optional; defaults to LocalDiscoverer
    .build()
    .expect("build servient");
```

For pure-consumer / pure-exposer use cases, register only the needed side.
The `ClientBinding` is a shared `Arc` — one instance per protocol serves all
consumed Things (v4.1 AD57).

### 3. Produce + Expose a Thing (Producer)

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

// produce() 创建 draft handle（尚未可远程访问）。
let handle = servient.produce(lamp_td()).expect("produce");

// 挂载 handler（生命周期内可随时替换 — AD14）。
// sync handler（零分配热路径）：
handle.set_property_read_handler("status", StatusRead);

// 或 async handler（I/O 密集型，feature = "async"）：
// handle.set_async_property_read_handler("status", MyAsyncRead);

// expose() 在所有 server binding 上注册路由 + 插入 servable 注册表。
// TD 在此后冻结。
handle.expose().await.expect("expose");

// 本地服务端交互 — sync 派发调 sync handler；async 派发（*_async）调任一种。
let value = handle.read_property("status", &InteractionInput::empty())?;
let _     = handle.read_property_async("status", &InteractionInput::empty()).await?;
handle.emit_event("overheat", payload)?;
handle.emit_property_change("temperature", temp_payload)?;
```

Local dispatch surface on `ExposedThingHandle` (Scripting API §7):

| Op | Sync method | Async method |
| --- | --- | --- |
| read property | `read_property` | `read_property_async` |
| write property | `write_property` | `write_property_async` |
| invoke action | `invoke_action` | `invoke_action_async` |
| query action | `query_action` | `query_action_async` |
| cancel action | `cancel_action` | `cancel_action_async` |
| observe property | `observe_property` | `observe_property_async` |
| unobserve property | `unobserve_property` | `unobserve_property_async` |
| subscribe event | `subscribe_event` | `subscribe_event_async` |
| unsubscribe event | `unsubscribe_event` | `unsubscribe_event_async` |

> Sync dispatch refuses async handlers (returns structured
> `UnsupportedOperation` with handler-phase context). Use the `*_async`
> variant when an async handler is registered. See
> `docs/design.md` for the current Scripting API posture.

### 4. Dispatch — binding-owned driving (v4.1 AD56)

**Servient 不跑循环。** 它只暴露 `Dispatch::serve_request(req).await`。
每个 binding 的 `serve()` 启动自己的驱动模型：

**zenoh binding（sync 回调）** — `serve()` 声明路由 + spawns draining task：

```rust
fn serve(&self, thing_id: &ThingId, td: &Thing, ctx: &BindingContext) -> CoreResult<()> {
    // 1. Declare zenoh queryables/subscribers from td routes.
    // 2. Spawn draining task (first serve only):
    if let Some(dispatch) = &ctx.dispatch {
        self.spawn_draining_task(dispatch.clone());
    }
    Ok(())
}
// zenoh sync callback: try_send(req) → binding's internal channel
// draining task: recv().await → dispatch.serve_request(req).await → send_response(resp)
```

**HTTP/CoAP binding（async handler）** — `serve()` 注册路由，handler 里直接调：

```rust
// HTTP route handler (registered in serve()):
async fn handle_read(req: Request) -> Response {
    let inbound = build_inbound_request(req);
    let resp = self.dispatch.serve_request(inbound).await;
    resp.into_http()
}
// hyper 连接池提供 backpressure，不需要 channel
```

**bare no_std（无 executor）** — super-loop 轮询：

```rust
loop {
    if let Some(req) = binding.try_accept() {
        let resp = dispatch.serve_request(req).await;  // 或 sync dispatch
        binding.send_response(resp);
    }
    // ... 其他 super-loop 工作
}
```

### 5. Consume a Remote Thing (Consumer)

```rust
let consumed = servient.consume(remote_td()).expect("consume");

// One-shot ops — all async (drive real ClientBinding):
let _ = consumed
    .read_property("status", InteractionOptions::new())
    .await?;
let _ = consumed
    .invoke_action("toggle", InteractionOptions::new())
    .await?;

// Bulk property ops (Scripting API §6.5):
let all = consumed
    .read_all_properties(InteractionOptions::new())
    .await?; // aggregated JSON InteractionOutput
let _ = consumed
    .write_multiple_properties(
        &[
            ("brightness", Payload::new(b"75".to_vec(), "text/plain")),
        ].into_iter().collect(),
        InteractionOptions::new(),
    )
    .await?;

// Streaming ops (Scripting API §6.6/§6.7) — pull-queue deviation:
let mut temp = consumed
    .observe_property("temperature", InteractionOptions::new())
    .await?;
while let Some(sample) = temp.next().await {
    println!("temp={:?}", sample.body);
}
// Optional explicit cleanup; dropping the handle also releases the guard:
consumed
    .unobserve_property("temperature", InteractionOptions::new())
    .await?;

// Subscribe to a single event:
let mut motion = consumed
    .subscribe_event("motion", InteractionOptions::new())
    .await?;

// Or fan out across every declared event in one call:
let mut events = consumed
    .subscribe_all_events(InteractionOptions::new())
    .await?;
while let Some((event_name, payload)) = events.next().await {
    println!("{}: {:?}", event_name.as_str(), payload.body);
}
```

`observe_property` / `subscribe_event` / `subscribe_all_events` return a
`Subscription` / `EventStream` implementing `futures_core::Stream`. The
wire-side `SubscriptionGuard` for each open subscription is owned by the
handle; dropping the handle releases every still-active guard. See
`docs/design.md` for the current subscription model.

### 6. Discover Things

```rust
use clinkz_wot_discovery::DiscoveryFilter;

// discover() 同步返回一个惰性 process（AD10）。
let mut process = servient.discover(DiscoveryFilter::all());

// 真正的目录工作发生在第一次 next() 里。
while let Some(thing) = process.next().await? {
    println!("found: {:?}", thing.id);
}
```

> `InteractionOptions` accepts bare field access or two builder
> conveniences: `InteractionOptions::with_data(payload)` and
> `.with_uri_variable("k", "v")` (chainable).

### 7. Destroy (Quiescing Teardown)

```rust
// destroy() 幂等（AD27）。注销路由、drain 在途、移除注册表条目。
handle.destroy().await.expect("destroy");
```

## Feature Flags

| Feature | Effect |
| --- | --- |
| `default = ["std"]` | std runtime + tokio. `std` implies `async`. |
| `alloc` | Dynamic data on `no_std`. |
| `std` | Networking, filesystem, async runtime, host conveniences. Implies `alloc` + `async`. |
| `async` | Native-async Servient surface (`consume`/`produce` handles, async handler setters, async local dispatch, streaming subscriptions). On `no_std` requires an executor (embassy). |
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

- [Current design](docs/design.md) — authoritative architecture, API, feature, and verification reference.
- [Current documentation plan](PLAN.md) — session entry point for documentation.
- [Deprecated archive](docs/deprecated/README.md) — historical baselines, plans, target notes, and audit context.

## License

MIT. Portions derived from `wot-td`. See [LICENSES/MIT.txt](LICENSES/MIT.txt).
