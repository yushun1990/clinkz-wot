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

### 1. Implement a ServerBinding

每个 binding 在 `configure` 里选择自己的 dispatch 模式：

```rust
use clinkz_wot_core::{
    BindingContext, CoreError, Dispatch, InboundRequest, InboundResponse,
    ServerBinding, ThingId,
};
use clinkz_wot_td::thing::Thing;
use alloc::sync::Arc;

struct MyServerBinding {
    // 按需存储 — 不是全部都要
    dispatch: Option<Arc<dyn Dispatch>>,
}

impl ServerBinding for MyServerBinding {
    fn configure(&self, ctx: &BindingContext) {
        // 从 context 里拿需要的 capability。
        // 新增 capability = 给 BindingContext 加字段，不改 trait。
        //
        // 直派发模式 (HTTP/CoAP async handler):
        //   self.dispatch = ctx.dispatch.clone();
        //
        // fan-in 模式 (zenoh sync callback):
        //   self.fanin = ctx.fanin_sender.clone();
        //
        // poll 模式 (bare no_std):
        //   什么都不存; 实现 try_accept().
    }

    fn send_response(&self, response: InboundResponse) {
        // 把 InboundResponse 映射回协议回复。
    }

    fn register_thing(&self, thing_id: &ThingId, td: &Thing) -> Result<(), CoreError> {
        Ok(())
    }

    fn unregister_thing(&self, thing_id: &ThingId) {}
}
```

### 2. Build a Servient

```rust
use alloc::{boxed::Box, sync::Arc};
use clinkz_wot_core::{
    ClientBinding, ClientBindingFactory, ProtocolBinding, ProtocolId, ServerBinding,
};

struct MyClientFactory;
impl ClientBindingFactory for MyClientFactory {
    fn build(&self) -> Box<dyn ClientBinding> {
        todo!() // your async ClientBinding impl
    }
}

struct MyProtocolBinding;
impl ProtocolBinding for MyProtocolBinding {
    fn protocol(&self) -> ProtocolId { ProtocolId("custom") }
    fn client_factory(&self) -> Option<Box<dyn ClientBindingFactory>> {
        Some(Box::new(MyClientFactory))
    }
    fn server(&self) -> Option<Arc<dyn ServerBinding>> {
        Some(Arc::new(MyServerBinding { dispatch: None }))
    }
}

let servient = ServientBuilder::new()
    .with_protocol_binding(Arc::new(MyProtocolBinding))
    // .with_discoverer(custom)  // optional; defaults to LocalDiscoverer
    .build()
    .expect("build servient");
```

`build()` 组装 Servient，然后对每个 binding 调一次 `configure(&ctx)`，
传入 `BindingContext { event_broker, dispatch, fanin_sender }`。每个 binding
按需取用。

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
handle.set_property_read_handler("status", StatusRead);

// expose() 在所有 server binding 上注册路由 + 插入 servable 注册表。
// TD 在此后冻结。
handle.expose().await.expect("expose");

// 本地服务端交互。
let value = handle.read_property("status", &InteractionInput::empty())?;
handle.emit_event("overheat", payload)?;
```

### 4. Dispatch — 由 binding 驱动，不是 Servient

**Servient 不跑循环。** 它只暴露 `Dispatch::serve_request(req).await`。
每个 binding 自己决定怎么调它：

**zenoh binding（sync 回调）** — binding 自己 owns channel + draining task：

```rust
// binding::configure 里 spawn 一个 draining task:
fn configure(&self, ctx: &BindingContext) {
    let dispatch = ctx.dispatch.clone().expect("dispatch");
    let rx = self.internal_rx.clone();  // binding 自己的 channel
    tokio::spawn(async move {
        while let Ok(req) = rx.recv().await {
            let resp = dispatch.serve_request(req).await;
            // binding 自己负责把 resp 发回客户端
        }
    });
}
// zenoh 回调 (sync): try_send(req) 到 binding 自己的 channel
```

**HTTP/CoAP binding（async handler）** — route handler 里直接调：

```rust
// HTTP route handler:
async fn handle_read(req: Request) -> Response {
    let inbound = build_inbound_request(req);
    let resp = self.dispatch.serve_request(inbound).await;
    resp.into_http()
}
// hyper 连接池提供 backpressure，不需要 channel、不需要循环
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

// 所有方法是 async — 驱动真实 ClientBinding。
let output = consumed.read_property("status", InteractionOptions::new()).await?;
let _ = consumed.invoke_action("toggle", InteractionOptions::new()).await?;
```

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

### 7. Destroy (Quiescing Teardown)

```rust
// destroy() 幂等（AD27）。注销路由、drain 在途、移除注册表条目。
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
