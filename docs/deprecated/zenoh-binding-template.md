# Zenoh Binding Template — URI Convention and Session Aggregation

> Status: **Design draft v0.1**. No implementation code is proposed here. This
> document defines the zenoh binding-template semantics that align the
> `clinkz-wot-protocol-bindings-zenoh` crate with W3C WoT TD 1.1 design intent,
> and records the gap in the current convention.

## 0. Reference Points

### 0.1 W3C WoT normative references

| Spec section | What it mandates |
|---|---|
| TD 1.1 §5.3.1.1 `Thing.base` (L685–693) | `base` is the base URI for all relative URI references in a TD; resolution per RFC 3986. |
| TD 1.1 §5.3.3 Security Vocabulary | `securityDefinitions` carry **configuration** (scheme, in, name, endpoints), not secrets. |
| TD 1.1 §5.3.4.2 `Form` (L2107–2277) | `href` is the mandatory submission target; its URI scheme identifies the Protocol Binding (§8.3). `security` is an optional per-form override. |
| TD 1.1 §6.3.4.2 `security` in Forms (L3133–3167) | Per-form `security` is a **complete override** of Thing-level security; different forms may carry independent credentials ("OR" fashion). |
| TD 1.1 §8.2 Data Schemas (L4477–4478) | A Consumer **MUST** generate URIs according to base URIs and form href parameters given in the TD. |
| TD 1.1 §8.3 Protocol Bindings (L4484–4493) | Every form **MUST** follow the Protocol Binding indicated by the URI scheme of its `href`. |
| TD 1.1 §8.3.2 Other Protocol Bindings (L4709–4715) | Non-HTTP protocol binding semantics are defined in separate binding-template documents (`WOT-BINDING-TEMPLATES`). The core TD spec is silent on connection/session lifecycle. |
| TD 1.1 §A.2 MQTT example (L6114–6160) | Established ecosystem precedent: `mqtt://192.168.1.187:1883/illuminance` — authority = broker address, path = topic. |
| Scripting API §6.1 `consume(td)` | Takes only the TD; no credentials parameter. Protocol binding init is implementation-encapsulated. |
| Scripting API §11.1.4 | Runtime **should not** expose any API for scripts to query provisioned security credentials. No standardized credential-acquisition interface exists. |

### 0.2 Current implementation reference points

| File:line | Role |
|---|---|
| `td/src/core/data_type/uri.rs:393–421` | `resolve_form_href` — protocol-neutral RFC 3986 base+href resolution. **Already spec-correct.** |
| `protocol-bindings/core/src/form.rs:201–205` | `resolve_form_target` — binding wrapper feeding `Thing.base` + `Form.href`. |
| `protocol-bindings/protocols/zenoh/src/form.rs:231–242` | `extract_zenoh_target_from_resolved_href` — **the gap**: strips `zenoh://` and lumps authority into the key expression. |
| `protocol-bindings/protocols/zenoh/src/form.rs:25–28` | `ZenohFormTarget { key_expr: String }` — no authority field. |
| `protocol-bindings/protocols/zenoh/src/runtime/zenoh.rs:34–37` | `ZenohSessionTransport { session: zenoh::Session }` — single session, no pool. |
| `protocol-bindings/protocols/zenoh/src/protocol_binding.rs:17–28` | `shared(session)` — canonical single-session constructor. |
| `core/src/binding.rs:56–88` | `ClientBinding` trait — stateless by design (AD57); no session lifecycle methods. |
| `core/src/thing.rs:1127–1157` | `ConsumedThing::request` — form selection by first matching `supports_with_thing()`. |
| `td/tests/fixtures/clinkz-extension-defaults.td.jsonld:18` | Fixture using `base: "zenoh://clinkz/things/targeted-roundtrip/"` (legacy convention). |
| `protocol-bindings/protocols/zenoh/tests/zenoh_form_test.rs:111` | Test asserting `key_expr == "clinkz/things/lamp/status"` (authority swallowed). |

---

## 1. Problem Statement

### 1.1 The current convention

The zenoh binding treats every `zenoh://` URI as authority-less: everything
after the scheme becomes the key expression. Trace through the fixture:

```
base: "zenoh://clinkz/things/targeted-roundtrip/"
href: "properties/status"
  ↓ resolve_form_href (RFC 3986, spec-correct)
resolved = "zenoh://clinkz/things/targeted-roundtrip/properties/status"
  ↓ extract_zenoh_target_from_resolved_href (strip "zenoh://")
key_expr = "clinkz/things/targeted-roundtrip/properties/status"
```

The `clinkz` token — which RFC 3986 parses as the URI **authority** — is folded
into the key expression. The session is supplied out-of-band via
`shared(session)`, so the TD never tells the Consumer which router to connect
to.

### 1.2 Why this conflicts with WoT design intent

| WoT requirement | Current behavior | Conflict |
|---|---|---|
| §8.2: Consumer MUST generate URIs from `base`+`href` per RFC 3986 | Authority is parsed by `fluent_uri` then discarded into the key string | The mandated authority component is semantically lost. |
| `base` is the common place for the server address (§5.3.1.1) | `base` cannot express a router address; the session is connected out-of-band | A TD cannot tell a Consumer where the zenoh router is. The Consumer is not self-sufficient. |
| MQTT precedent: `mqtt://host:port/topic` (§A.2) | `zenoh://keyprefix/...` with no host:port | Inconsistent with the established ecosystem pattern for broker-based protocols. |
| Per-form `security` override (§6.3.4.2) implies per-form credentials | One shared session, no per-authority or per-credential isolation | Cannot model Things whose affordances live on routers with different security. |
| Multi-Thing servient aggregation (inherent client/server asymmetry) | One session per servient; distinct authorities unreachable | A Servient consuming Things on different zenoh networks cannot reach all of them. |

### 1.3 What this template does NOT change

- The protocol-neutral `ClientBinding` trait stays stateless (AD57). Session
  aggregation belongs **inside** the zenoh binding, not in the engine core.
- `resolve_form_href` / `resolve_form_target` are already spec-correct and
  untouched. Only zenoh-specific target extraction changes.
- The `no_std + alloc` planning surface (`form.rs`) stays no_std-capable.
  Session pooling is a `std` runtime concern (AGENTS.md no_std policy).

---

## 2. URI Convention

### 2.1 Canonical form

A zenoh TD form resolves to an absolute URI of the form:

```
zenoh[+<transport>]://<authority>/<key-expr>
```

Where, per RFC 3986:

| Component | Definition | zenoh meaning |
|---|---|---|
| `scheme` | `zenoh` or `zenoh+<transport>` | Identifies the zenoh binding (§8.3). The optional `+transport` suffix selects the zenoh transport, following the RFC 8323 / CoAP precedent (`coap+tcp`, `coap+ws`). Bare `zenoh` = `tcp`. See §2.4. |
| `authority` | `host[:port]` (RFC 3986 §3.2) | The zenoh router/peer endpoint. `host` is a DNS name or IP literal; `port` defaults to `7447` when omitted. |
| `path` | `/key-expr` | The zenoh key expression, with the leading `/` stripped. |

Examples:

```
zenoh://router-a:7447/mything/properties/status
zenoh+tcp://router-a:7447/mything/properties/status   (equivalent to the above)
zenoh+udp://router-a:7447/mything/events/alarm
└ scheme ┘└─ authority ─┘└────── key_expr ──────┘
```

### 2.2 Authority is mandatory

A well-formed WoT TD must let a Consumer locate the target server from the TD
alone (§8.2 MUST). For the zenoh binding, the router endpoint **is** the
authority component of the resolved `base`+`href` URI. An empty authority
(e.g. `zenoh:///key`) leaves the Consumer with no router to connect to and is
therefore a **TD configuration error**, not a valid form:

```
zenoh:///mything/properties/status   ← REJECTED at plan-build time
                                      ZenohBindingError::MissingAuthority
```

The binding rejects such forms with `ZenohBindingError::MissingAuthority`
during `extract_zenoh_target_from_resolved_href` (§3.2). There is no
"default session" fallback on the client side — every consumer interaction
derives its session from a non-empty authority in the TD. This keeps the TD
self-sufficient, as §8.2 requires.

The server side is exempt (§8): a Thing exposes on a router it already holds a
session to, supplied explicitly via `server(session)`. This is the
client/server asymmetry: the server's router is app-managed, the client's
router is TD-driven.

### 2.3 base + href resolution (unchanged, spec-correct)

`base` carries the router address once at the Thing level; each affordance
`href` is a relative key path:

```json
{
  "base": "zenoh://router-a:7447/mything/",
  "properties": {
    "status": {
      "forms": [{ "href": "properties/status", "op": "readproperty" }]
    }
  }
}
```

Resolves to `zenoh://router-a:7447/mything/properties/status` →
authority `router-a:7447`, key_expr `mything/properties/status`.

The trailing `/` on `base` is significant: it makes `properties/status`
resolve as a path segment under `mything/`, not replace it (RFC 3986 §5.3).
This is the same convention the MQTT example uses with `base:
"mqtt://192.168.178.72:1883"`.

### 2.4 Transport protocol (in the URI scheme)

The zenoh transport protocol is encoded in the URI scheme via a `+transport`
suffix, following the established IANA precedent: `coap+tcp` and `coap+ws`
(registered Permanent URI schemes, RFC 8323) encode the transport in the
scheme rather than in form metadata. The zenoh binding adopts the same
convention so a TD form is self-describing about its transport:

| Scheme | zenoh locator proto | Status |
|---|---|---|
| `zenoh` | `tcp` | Default. Equivalent to `zenoh+tcp`. |
| `zenoh+tcp` | `tcp` | Explicit TCP. |
| `zenoh+udp` | `udp` | UDP transport. |
| `zenoh+<other>` | `<other>` | Reserved for future zenoh transports (e.g. serial, quic). v0.1 of this template defines only `tcp` (and `udp` as a recognized variant); others are rejected as `UnsupportedTransport` until a binding revision registers them. |

The authority `host:port` plus the scheme-derived proto combine into the zenoh
locator `<proto>/<authority>` (e.g. `tcp/router-a:7447`). No extension term is
needed for transport selection — it lives in the URI where §8.3 (binding
identified by `href` scheme) expects it.

**Exotic locators** that do not fit the `host:port` authority model (e.g.
`serial/usb0`, `unixsock/path`) cannot be expressed as a standard RFC 3986
authority and are **deferred**. A future revision may address them via a
dedicated scheme variant or a binding-level configuration; v0.1 rejects forms
whose authority is not a `host[:port]` pair.

### 2.5 Key expression validity

The path component (minus the leading `/`) must be a valid zenoh key
expression. The binding validates this at plan-build time and rejects malformed
key expressions with a `ZenohBindingError::InvalidKeyExpr`. URI-template hrefs
(`FormHref::Template`) are preserved verbatim and expanded by the caller via
`uriVariables` before the binding sees a concrete path — same as the current
behavior for HTTP.

---

## 3. Form Parsing Changes

### 3.1 `ZenohFormTarget` gains an authority field

```rust
/// Resolved zenoh form target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZenohFormTarget {
    /// Zenoh transport protocol, derived from the URI scheme suffix.
    ///
    /// `zenoh` and `zenoh+tcp` normalize to `"tcp"`; `zenoh+udp` to `"udp"`.
    /// Combines with `authority` to form the zenoh locator
    /// `<transport>/<authority>` (§2.4).
    pub transport: String,

    /// Zenoh router/peer endpoint.
    ///
    /// Derived from the RFC 3986 authority component of the resolved href.
    /// Always non-empty for a valid form; an empty authority is rejected at
    /// extraction time as `MissingAuthority` (§2.2).
    pub authority: String,

    /// Zenoh key expression used by the concrete zenoh operation.
    ///
    /// The path component of the resolved href with the leading `/` stripped.
    pub key_expr: String,
}
```

### 3.2 `extract_zenoh_target_from_resolved_href` rewrites

The current implementation strips `zenoh://` and returns the remainder as
`key_expr`. The new implementation parses the resolved href as a URI and splits
authority from path:

```rust
fn extract_zenoh_target_from_resolved_href(href: &str) -> ZenohBindingResult<ZenohFormTarget> {
    let uri = fluent_uri::Uri::parse(href).map_err(ZenohBindingError::from)?;
    // scheme: "zenoh", "zenoh+tcp", "zenoh+udp", ...
    let scheme = uri.scheme().as_str();
    let transport = parse_zenoh_transport(scheme)?;
    let authority = uri
        .authority()
        .map(|a| a.as_str().to_string())
        .filter(|a| !a.is_empty())
        .ok_or_else(|| ZenohBindingError::MissingAuthority(format!(
            "zenoh form href '{}' has no authority; the TD base/href must name a router (e.g. zenoh://router:7447/...)", href
        )))?;
    let path = uri.path().as_str();
    let key_expr = path
        .strip_prefix('/')
        .ok_or_else(|| ZenohBindingError::UnsupportedForm(format!(
            "href '{}' has no path component", href
        )))?;
    if key_expr.is_empty() {
        return Err(ZenohBindingError::UnsupportedForm(format!(
            "href '{}' has an empty key expression", href
        )));
    }
    Ok(ZenohFormTarget { transport, authority, key_expr: key_expr.into() })
}

/// Splits `zenoh[+transport]` into a normalized transport string.
/// `zenoh` and `zenoh+tcp` → `"tcp"`; `zenoh+udp` → `"udp"`;
/// unknown suffix → `UnsupportedTransport`.
fn parse_zenoh_transport(scheme: &str) -> ZenohBindingResult<String> {
    match scheme {
        "zenoh" | "zenoh+tcp" => Ok("tcp".into()),
        "zenoh+udp" => Ok("udp".into()),
        other if other.starts_with("zenoh+") => {
            Err(ZenohBindingError::UnsupportedTransport(other.into()))
        }
        other => Err(ZenohBindingError::UnsupportedForm(
            format!("href scheme '{}' is not zenoh", other),
        )),
    }
}
```

(The exact `fluent_uri` accessor names follow the crate's current API; the
contract is: split authority from path, strip the leading `/`.)

### 3.3 What stays unchanged

- `try_extract_zenoh_target` — same shape, delegates to the rewritten
  extractor.
- `plan_zenoh_operation` / `plan_zenoh_affordance_operation*` — carry the new
  `transport` and `authority` fields through `ZenohOperationPlan`.
- `ZenohFormMetadata` (`cz-zenoh:qos` etc.) — untouched.

### 3.4 `is_zenoh_form` scheme check (updated)

`is_zenoh_form` / `is_zenoh_form_target` currently match `starts_with("zenoh://")`.
Under the scheme-suffix convention they must recognize any `zenoh[+transport]`
variant. The check becomes: the scheme (the portion before `://`) starts with
`zenoh` and is either exactly `zenoh` or begins with `zenoh+`. This is a
one-line predicate change; the `parse_zenoh_transport` helper in §3.2 then
validates the specific suffix.

### 3.5 `no_std` boundary

`form.rs` already depends on `fluent_uri` (used by `resolve_form_href`), so
the authority parsing adds no new dependency and stays `no_std + alloc`. No
session logic enters the planning crate.

---

## 4. Extension Vocabulary

### 4.1 Existing terms (unchanged)

| Term | Purpose |
|---|---|
| `cz-zenoh:qos` | Preferred zenoh QoS hint. |
| `cz-zenoh:priority` | Preferred zenoh priority hint. |
| `cz-zenoh:congestionControl` | Preferred zenoh congestion control hint. |

### 4.2 Deferred term: `cz-zenoh:sessionPolicy`

A per-form policy-name extension (`cz-zenoh:sessionPolicy: "tls"`) is
**deferred** past v0.1. v0.1 uses a single binding-wide policy passed to
`client_pooled(policy)` (§5.7); per-form config variation is driven by the
policy reading the resolved target — transport (§2.4), authority (§2.2), and
effective security (§6.1) — not by a per-form policy string.

The pool's default behavior is simply: for each new authority the policy
builds a config, `zenoh::open` runs once, and the session is cached. There is
no separate "per-authority open" mode distinct from the pool — the pool IS
the per-authority open-and-cache mechanism.

A future revision may reintroduce `cz-zenoh:sessionPolicy` with a named-policy
registry (`pool.register_policy("tls", Arc<dyn ZenohSessionPolicy>)`) when
concrete deployments show forms that need radically different session
strategies on the same authority beyond what per-form security already
expresses.

### 4.3 Namespace

All terms remain under `cz-zenoh: https://clinkz.io/wot/zenoh#` as already
declared in `docs/protocol-bindings.md:119`. No new namespace is introduced.

The earlier draft's `cz-zenoh:locator` extension term is **removed**: transport
selection now lives in the URI scheme suffix (§2.4), which is both more
URI-native and consistent with the CoAP `coap+tcp`/`coap+ws` precedent.

---

## 5. Per-Authority Session Aggregation

### 5.1 Where it lives: two backends, one trait seam

Session aggregation is a **runtime adapter concern**. It does not touch:

- `clinkz-wot-core` (`ClientBinding` trait, `ConsumedThing`) — stays stateless.
- `clinkz-wot-protocol-bindings` (shared form selection) — stays protocol-neutral.
- `form.rs` (planning surface) — stays `no_std + alloc`, only parses authority.

The `ClientBinding` impl is `ZenohBindingTransport<T>`, generic over a
`T: ZenohTransport` adapter (`zenoh.rs:38–56`). The trait is the seam:

```rust
pub trait ZenohTransport: Send + Sync {
    fn execute(&self, request: ZenohTransportRequest) -> CoreResult<InteractionOutput>;
    fn open_subscription(...) -> CoreResult<(Subscription, Box<dyn SubscriptionGuard>)>;
}
```

`ZenohTransportRequest` carries the `Arc<ZenohOperationPlan>` — which now
includes `transport`, `authority`, and `key_expr`. **Each backend decides how
to turn the authority into a session.** There are two backends, selected by
feature:

| Backend | Feature | Environment | Authority handling |
|---|---|---|---|
| `ZenohSessionPool` (§5.2) | `zenoh` | `std` (host/gateway) | Lazy-opens one session per distinct authority; caches and reuses. Reaches Things on many routers/networks. |
| `ZenohPicoTransport` (single session) | `zenoh-pico` | `no_std + alloc` (constrained) | Holds one session to one router, supplied by the platform. Validates the form's authority against that router; cross-authority forms are unreachable. |

The `std` path is the multi-router aggregator. The `no_std` path is the
constrained single-connection reality — a microcontroller cannot hold many
zenoh sessions, and cross-isolated-network consumption is a gateway job, not a
device job. Both backends receive the same `ZenohTransportRequest` (with the
mandatory authority from §2.2); the difference is purely how many sessions
they can manage.

This respects AGENTS.md: *"Put host/cloud runtime functionality behind `std`
features or in separate `std` crates"* and §8.3.2's delegation of non-HTTP
session semantics to binding templates. The planning crate stays
`no_std + alloc`; only the concrete backends differ by feature.

### 5.2 The `std` backend: `ZenohSessionPool`

The `zenoh`-feature backend aggregates sessions keyed by router authority.
It replaces the single-session `ZenohSessionTransport` as the default
multi-router `std` backend:

```rust
/// Aggregates zenoh sessions keyed by router authority.
///
/// Available with the `zenoh` feature. Replaces the single-session
/// `ZenohSessionTransport` for Consumers that must reach Things on
/// multiple zenoh routers/networks.
pub struct ZenohSessionPool {
    /// Cached sessions keyed by authority string (e.g. "router-a:7447").
    /// Every entry is lazily opened from a TD-resolved authority (§2.2);
    /// there is no default session.
    sessions: Mutex<HashMap<String, Arc<zenoh::Session>>>,
    /// Policy that builds a zenoh::Config from an authority + form metadata.
    policy: Arc<dyn ZenohSessionPolicy>,
    reply_timeout: Duration,
}
```

### 5.3 The `no_std` backend: single-session transport

The `zenoh-pico`-feature backend (and any future constrained adapter)
implements `ZenohTransport` with **one session**, supplied by the platform at
init time. It cannot pool sessions across authorities — memory and connection
limits on constrained targets make that infeasible. Its authority handling:

- The platform configures the transport with the router it connects to
  (e.g. `tcp/gateway.local:7447`), typically at boot or build time.
- On `execute(request)`, the backend compares the plan's `authority` against
  its connected router:
  - **Match** → issue the get/put/subscribe on its single session at
    `key_expr`. zenoh routing within the connected network may still reach
    keys hosted on other routers of that same network.
  - **Mismatch (isolated network)** → return `CoreError::UnreachableAuthority`
    (or equivalent). The constrained device cannot open a second session to a
    different router; consuming a Thing on an isolated network is a gateway
    job, done by a `std` Servient running `ZenohSessionPool`.

This keeps the §2.2 invariant intact under `no_std`: the TD still carries a
mandatory authority, and the backend uses it to **validate reachability** even
when it cannot open a new session. A constrained device can faithfully consume
any TD whose authority resolves to its connected network; it correctly
declines TDs it cannot reach, rather than silently routing them to the wrong
router.

The `ZenohPicoTransport` shape (per `docs/zenoh-pico-runtime-target.md:36–52`)
already accepts an injected session/transport handle, polling loop, and
buffer ownership — the authority check is an additional guard in `execute`,
not a structural change.

### 5.4 `ZenohSessionPolicy` trait

Decouples "which authority → which config" from the pool itself, so
applications can inject TLS, credentials, and custom transports:

```rust
/// Builds a zenoh session configuration for a given form target.
///
/// Implementations translate a resolved target — transport (from the scheme
/// suffix, §2.4), authority (§2.2), and effective security (§6.1) — into a
/// `zenoh::Config` ready for `zenoh::open`.
pub trait ZenohSessionPolicy: Send + Sync {
    fn config_for(&self, target: &ZenohFormTarget) -> CoreResult<zenoh::Config>;
}
```

The default implementation (`DefaultSessionPolicy`) builds a client-mode config
with `connect/endpoints: ["<transport>/<authority>"]` — e.g.
`tcp/router-a:7447` or `udp/router-a:7447` — using the transport string
parsed from the scheme and the authority from the resolved href.

### 5.5 `ZenohTransport::execute` flow (revised)

```text
1. plan_for(thing, form, op) → Arc<ZenohOperationPlan { authority, key_expr, kind, metadata }>
2. session = pool.get_or_open(&plan.authority, &plan) → Arc<zenoh::Session>
   (authority is always non-empty — §2.2 rejects empty authority at plan-build time)
3. match plan.kind {
     Query         => session.get(key_expr, reply_timeout),
     Put           => session.put(key_expr, payload),
     Subscribe     => session.declare_subscriber(key_expr),
     RequestReply  => session.get(key_expr, reply_timeout),  // queryable-backed
     Unsubscribe   => (caller-held guard; no session lookup),
   }
```

Sessions are `Arc`-shared and reused across all forms targeting the same
authority. `zenoh::Session` is already thread-safe and internally `Arc`-shared,
so the pool holds no outer `Mutex` around the session itself (mirrors the
`SharedZenohTransport<T>` rationale at `runtime/zenoh.rs:377–384`).

### 5.6 Session lifecycle

- **Open**: lazily on first interaction targeting a given authority. The
  `ZenohSessionPolicy` builds the config; `zenoh::open(config)` runs once per
  authority and is cached.
- **Reuse**: subsequent interactions with the same authority `Arc::clone` the
  cached session.
- **Close**: explicitly via a new `shutdown_authority(&str)` / `shutdown_all()`
  on the pool, or on pool drop. zenoh sessions close themselves on drop, so the
  pool's `Drop` releases every cached session.

### 5.7 Constructor surface (revised)

The current `shared(session)` returns both a server and a client binding from
one session. Under the spec-aligned convention this is split to reflect the
client/server asymmetry: the **server** exposes on an app-managed session (one
router), the **client** derives sessions from TD authorities (many routers).

```rust
/// Server-only constructor. The Thing exposes on the given session's router.
/// The application owns session lifecycle (open, TLS, credentials).
pub fn server(session: zenoh::Session) -> Arc<dyn ServerBinding>;

/// Client-only constructor. Sessions are derived per-TD via the policy.
/// There is no pre-opened session on the client side — §2.2.
pub fn client_pooled(policy: Arc<dyn ZenohSessionPolicy>) -> Arc<dyn ClientBinding>;
```

The current `shared(session)` is **removed** for the client path. Applications
that both expose and consume Things compose the two constructors:

```rust
let session = zenoh::open(server_config).await?;
let server_binding = zenoh::server(session);
let client_binding = zenoh::client_pooled(Arc::new(DefaultSessionPolicy::default()));
```

A Servient that consumes Things only (no local Thing exposed) uses
`client_pooled(policy)` alone. A Servient that exposes a Thing only uses
`server(session)` alone.

### 5.8 No session sharing between server and client

A combined Servient (one that both exposes and consumes Things) **must not**
share a session between its server and client sides, even when both happen to
target the same router. Two independent reasons:

1. **Client and server face different Things.** The server exposes the
   Servient's own Thing on an app-managed session; the client consumes remote
   Things whose TDs declare their own authorities. The two sides address
   different Things on different routers in the general case — there is nothing
   to share.

2. **Even for the same Thing, the TD authority is not the server's session.**
   The server's session is connected to whatever router the *application*
   opened it against (a runtime fact). The Thing's TD `base` authority is what
   the *producer* declared (TD metadata). Nothing enforces these are the same
   router: a producer could expose on router-A but misconfigure `base` to
   router-B. If the client shared the server's session, it would silently use
   router-A regardless of what the TD says — masking the misconfiguration and
   violating §8.2 (the Consumer must derive the target from the TD). Keeping
   them decoupled means a mismatch surfaces as a failed interaction on
   router-B, which is the correct, diagnosable behavior.

Concretely, the server's session and the client pool are completely separate
objects with no handle in common:

```rust
// Combined Servient — two independent session owners.
let server_session = zenoh::open(server_config).await?;   // app-managed
let server_binding = zenoh::server(server_session);        // exposes on this router

let client_binding = zenoh::client_pooled(Arc::new(
    DefaultSessionPolicy::default(),                            // TD-driven
));
// The client pool opens its own sessions to each consumed TD's authority.
// It never sees, borrows, or clones the server's session.
```

This intentionally rejects any `pool.seed(authority, session)` or
"donate-the-server-session" optimization. The two-connections cost in the
self-consume-same-router edge case is negligible (the router multiplexes), and
the decoupling preserves the WoT invariant that the Consumer's target comes
from the TD, not from the producer's runtime state.

### 5.9 Plan cache interaction

The per-`Arc<Form>` plan cache (`zenoh.rs:211–261`) is unaffected: it still
keys by `(Arc<Form> pointer, Operation)` and caches `Arc<ZenohOperationPlan>`.
The plan now carries `authority`, so the cache amortizes authority parsing
too. No per-authority caching is needed at the plan layer — the session pool
handles that.

---

## 6. Security Considerations

### 6.1 Per-form security override (§6.3.4.2)

The spec allows per-form `security` to completely override Thing-level
security, in "OR" fashion. A Thing may expose one affordance via `nosec` and
another via `bearer`. The resolved `ZenohFormTarget` carries the form's
**effective security** (via `effective_form_security` at
`td/src/td_defaults.rs:56–60`).

Credentials split into two layers, applied at different points (see §6.3 for
the full spec grounding):

| Layer | When applied | Mechanism | Examples |
|---|---|---|---|
| **Session-level** | At `zenoh::open` (connect time) | `ZenohSessionPolicy::config_for` builds the `zenoh::Config` | TLS/mTLS certificate, PSK identity+secret |
| **Request-level** | Per `execute`/`subscribe` call | `SecurityProvider::apply` attaches material to the `ZenohTransportRequest` (repo's intended outbound path, `core/src/security.rs:149`) | Bearer token in zenoh attachment, API key |

`config_for` handles only session-level credentials — those established when
the connection opens. Request-level credentials (bearer tokens, API keys) are
attached per-interaction and are independent of which session the request
rides on.

### 6.2 Session keying

The pool keys sessions by authority by default. Whether two forms on the
**same authority** but **different effective security** share a session
depends on which credential layer differs (§6.1):

- **Request-level difference** (e.g. one form `bearer`, another `apikey` on
  the same router) → **same session**. Both ride one connection; the
  per-request credential is attached by `SecurityProvider::apply` on each
  `execute`. Session keying is unaffected.
- **Session-level difference** (e.g. one form `psk`, another `nosec` on the
  same router) → **separate sessions**. The policy must key the cache by
  `(authority, session-security)` instead of authority alone, since the
  connection-level config differs.

v0.1 of this template recommends authority-only keying (one session per
authority) and treats session-level security separation as a policy
implementation detail. A future revision may add a `cz-zenoh:sessionKey`
extension to force session separation when the policy cannot infer it from
the effective security scheme.

### 6.3 Credential acquisition

#### 6.3.1 What the WoT spec says (and does not say)

The WoT specifications split security metadata from secret material, and
deliberately leave credential *acquisition* unspecified:

- **TD 1.1 §5.3.3**: `securityDefinitions` carry **configuration** — scheme
  type (`basic`, `bearer`, `psk`, `oauth2`, ...), `in` (header/query/body),
  `name`, authorization/token endpoints, proxy. They do **not** carry secrets.
- **Scripting API §6.1 `consume(td)`**: takes only the TD; there is no
  credentials parameter. Step 4 says "make a request to the underlying
  platform to initialize the Protocol Bindings" with an Editor's note that
  this complexity is implementation-encapsulated.
- **Scripting API §11.1.4** (normative): *"the WoT Scripting Runtime **should
  not** expose any API for scripts to query the provisioned security
  credentials."* There is no `CredentialStore` interface, no `setCredentials`
  method, and no `credentials` field on `InteractionOptions` anywhere in the
  standard.

**Implication**: credential acquisition is **entirely implementation-defined**.
A conforming WoT runtime must source secrets through a non-standardized,
out-of-band channel — a platform credential store, runtime configuration, OS
keychain, TPM, or a runtime-specific extension. The TD never carries secrets;
it only declares what the Consumer *must provide*.

#### 6.3.2 The two credential layers in this binding

Credentials are needed at two distinct points, with different lifecycles:

**Session-level (connect time)** — TLS certificates, PSK identity+secret, mTLS
config. These are established when `zenoh::open(config)` runs and cannot vary
per-request on the same session. They flow through `ZenohSessionPolicy::config_for`
(§5.4), which builds the `zenoh::Config`. The policy implementation reads
secrets from an app-injected source (see §6.3.3).

**Request-level (per interaction)** — bearer tokens, API keys attached to a
single get/put/subscribe. These flow through the repo's existing
`SecurityProvider::apply` path (`core/src/security.rs:149`), which attaches
security material to a `TransportRequest`. This is protocol-neutral and
independent of which session the request rides on.

#### 6.3.3 Where secrets come from (Clinkz credential store)

The engine defines its own credential-acquisition abstraction, consistent with
the spec leaving this implementation-defined:

- `CredentialStore` trait (`core/src/security.rs:55–59`), keyed by
  `(thing_id, scheme_name)` → `Credentials` (`BearerToken`, `Basic`, `ApiKey`,
  `Psk`, `Other`). The only impl today is `InMemoryCredentialStore`
  (`security.rs:69–139`); future impls may back this with a vault, keychain, or
  TPM.
- `SecurityContext<'a>` (`security.rs:10–22`) carries an
  `Option<&'a dyn CredentialStore>` alongside the thing/form/scheme, and is
  passed to `SecurityProvider::apply`.
- `cz:credentialSource` — a Clinkz extension term on `SecurityScheme`
  (round-tripped as an opaque field today, `td/tests/validation_test.rs:301–316`)
  that *hints* where the runtime should source credentials for this scheme
  (e.g. `"platform"`, `"vault"`, `"tpm"`). The engine does not mandate its
  interpretation; it is advisory metadata for the deployment.

The store is **not** exposed to application scripts or to the TD — matching
Scripting API §11.1.4. It is injected at the runtime/binding layer.

#### 6.3.4 Current implementation gap (recorded)

The repo's *intended* outbound credential path — `SecurityContext` →
`SecurityProvider::apply` → attaches to `TransportRequest` — is **designed but
unwired** as of this writing:

- `SecurityProvider::apply` has zero production callers; the built-in
  `BearerSecurityProvider` / `BasicSecurityProvider` `apply` impls are no-ops
  with comments noting "the binding handles it" (`security.rs:455, 540`).
- `SecurityContext` is never constructed by production code.
- `BindingRequest` (`core/src/binding.rs:29–40`) carries no credential store
  or security context.

Until that path is wired (tracked separately), `ZenohSessionPolicy::config_for`
is the **only** wired credential injection point for the zenoh binding — and
it covers session-level credentials only. Request-level credentials (bearer
tokens in zenoh attachments) require the `apply` path to be connected; that
work is orthogonal to this URI-convention template but is a prerequisite for
honoring per-form `bearer`/`apikey` schemes on consumed interactions.

---

## 7. Migration Path

### 7.1 What breaks

The current fixture convention `base: "zenoh://clinkz/things/..."` is
reinterpreted under the new convention:

- `clinkz` parses as a URI authority (hostname), not a key prefix.
- `key_expr` becomes `things/...` instead of `clinkz/things/...`.
- The binding would attempt to connect to host `clinkz:7447`.

This is a breaking change to the zenoh binding's URI semantics, gated behind
this template's adoption.

### 7.2 Fixture migration

Every existing fixture and test must carry a non-empty authority (§2.2). There
is no empty-authority form. Two cases:

1. **Integration tests (real router)**: change `base` to
   `zenoh://<router-host>:<port>/<key-prefix>/`. For
   `docs/zenoh-runtime-integration-test.md` tests using
   `CLINKZ_WOT_ZENOH_ENDPOINT=tcp/127.0.0.1:7447`, the fixture becomes
   `base: "zenoh://127.0.0.1:7447/clinkz/things/..."`.

2. **Unit tests (no router)**: unit tests of the planning surface (`form.rs`)
   and the `ClientBinding` control flow already run against a **fake**
   `ZenohTransport` (the `ZenohBindingTransport<T>` generic), not a real
   session. These tests supply a concrete authority in the fixture (e.g.
   `base: "zenoh://test-router:7447/clinkz/things/..."`) and assert on the
   parsed `authority`/`key_expr`; the fake transport never opens a connection.
   No live router is required because the session pool is only exercised by
   the `std` integration tests.

### 7.3 Implementation sequencing

| Step | Crate | Change |
|---|---|---|
| 1 | `protocol-bindings/protocols/zenoh` (`form.rs`) | Add mandatory `authority` to `ZenohFormTarget`; rewrite `extract_zenoh_target_from_resolved_href` to split authority/path and reject empty authority (`MissingAuthority`). Pure planning, `no_std + alloc`. |
| 2 | `protocol-bindings/protocols/zenoh` (`form.rs` tests) | Update `zenoh_form_test.rs` assertions to the new `authority`/`key_expr` split; add a rejection test for empty authority. |
| 3 | `protocol-bindings/protocols/zenoh` (`runtime/`) | Add `ZenohSessionPool`, `ZenohSessionPolicy`, `DefaultSessionPolicy`. `std`-gated. |
| 4 | `protocol-bindings/protocols/zenoh` (`protocol_binding.rs`) | Replace `shared(session)` with `server(session)` + `client_pooled(policy)`. Remove the client-side single-session path. |
| 5 | `protocol-bindings/protocols/zenoh` (`runtime/zenoh.rs`) | Route `ZenohTransport::execute` through `ZenohSessionPool::get_or_open(&authority)`; authority is always non-empty. |
| 6 | `td/tests/fixtures/` | Migrate `clinkz-extension-defaults.td.jsonld` and related fixtures to a real-authority `base` (e.g. `zenoh://test-router:7447/clinkz/things/...`). |
| 7 | `docs/protocol-bindings.md` | Update the example at L154–162 and the `base`/`href` description at L143–145 to the new convention. |
| 8 | Workspace | `cargo test --workspace`, `cargo clippy`, `scripts/check-no-std.sh`; add a multi-router integration test. |

Each step leaves the workspace compiling. Steps 1–2 are pure planning and can
land before any runtime change. Steps 3–5 are the runtime work. Steps 6–7 are
documentation and fixtures.

### 7.4 Backward compatibility window

A transition feature (working name `zenoh-legacy-uri`) may interpret
authority-less `zenoh://<token>/...` URIs (where `<token>` is not a real
host:port) as legacy key prefixes during migration. This is **not
recommended** for the canonical path — every TD must carry a non-empty
authority (§2.2), and the legacy feature exists only to soften the cutover for
deployments that cannot migrate all fixtures at once. If added, it is removed
at the end of the migration window and any remaining authority-less fixture is
treated as a configuration error.

---

## 8. Server Side (out of scope for v0.1 of this template)

The server side (`ZenohServerBinding`) exposes a Thing on **one** router —
this is the client/server asymmetry the design discussion identified. The
server's `serve()` takes a single session supplied explicitly via
`server(session)` and declares queryables/subscribers on that session.
Multi-router exposure is a future concern (a Thing mirrored on several
routers) and is not addressed here.

This asymmetry is spec-consistent: a Thing declares its own `base` (one
endpoint), while a Consumer aggregates many Things (many bases). The binding
template reflects this by giving the client side a session pool and the server
side a single session.

---

## 9. Open Questions

1. **~~`fluent_uri` authority API~~ (resolved by spike)**: confirmed against
   `fluent-uri = "0.4"` (the version in `td/Cargo.toml:7`). The §3.2 pseudocode
   is accurate. Confirmed accessors:
   - `uri.scheme().as_str()` → `"zenoh"`, `"zenoh+tcp"`, `"zenoh+udp"`.
   - `uri.authority()` → `Option<Authority>`. **Empty authority returns
     `Some("")`, not `None`** — the §3.2 `.filter(|a| !a.is_empty())` correctly
     turns it into the `MissingAuthority` error.
   - `auth.as_str()` → full `"host:port"` (or `"[::1]:7447"` for IPv6, brackets
     kept — directly usable as the locator suffix).
   - `auth.host()` → `&str` (not `Option`); `auth.port()` → `Option<&str>`
     (string, e.g. `Some("7447")`).
   - `uri.path().as_str()` → `"/key/expr"`; `strip_prefix('/')` yields the
     zenoh key expression.

 2. **Default port handling**: when authority is `host` without a port, should
    the binding assume `7447` (zenoh default) or require an explicit port?
    Recommendation: assume `7447`, matching zenoh's own default, and document
    it. The `DefaultSessionPolicy` normalizes the locator to
    `<transport>/<host>:7447` before opening the session.

3. **~~IPv6 literals~~ (resolved by spike)**: `zenoh://[::1]:7447/key` parses
   with `authority().as_str() == "[::1]:7447"` — `fluent_uri` **keeps** the
   brackets in the authority string. Building the locator as
   `<transport>/<authority>` yields `tcp/[::1]:7447`, which matches zenoh's
   own locator format (bracketed IPv6). No special handling needed; use
   `authority().as_str()` verbatim.

4. **Session pool eviction**: should the pool evict idle sessions after a TTL,
   or keep them for the binding's lifetime? v0.1 recommendation: keep for
   lifetime; add eviction in a future revision if memory or connection limits
   demand it.

 5. **Named-policy registry (deferred)**: `cz-zenoh:sessionPolicy` and its
    registry are deferred past v0.1 (§4.2). When reintroduced, the API is
    likely a builder method on `ZenohSessionPool`
    (`register_policy("tls", Arc<dyn ZenohSessionPolicy>)`). v0.1 has no
    registry — one binding-wide policy, varied per-form via the target's
    effective security (§6.1).

6. **Discovery interaction**: when a Discovery directory returns TDs whose
   `base` values point at different routers, the session pool must open
   sessions lazily as each TD is consumed. Confirm this composes with the
   `Discoverer` trait without requiring eager session creation at discovery
   time. Expected: yes, because `consume(td)` only `Arc::clone`s the
   `ClientBinding`, and sessions open on first `invoke`.

---

## 10. Compliance Summary

| WoT spec requirement | This template's response |
|---|---|
| §8.2 MUST: Consumer generates URIs from `base`+`href` per RFC 3986 | Authority is parsed from the resolved URI and used as the session endpoint. §2, §3. |
| TD self-sufficiency: the TD locates the server (§5.3.1.1 `base`, §8.2) | Authority is **mandatory**; an empty authority is rejected as `MissingAuthority`, not papered over with a default session. §2.2. |
| §5.3.1.1 `base` carries the server address | `base` authority = zenoh router locator; per-affordance `href` = relative key path. §2.3. |
| §8.3 MUST: form follows the Protocol Binding of its `href` scheme | `zenoh[+transport]` scheme triggers the zenoh binding and selects the transport; follows RFC 8323 `coap+tcp` precedent. §2.1, §2.4. |
| §6.3.4.2 per-form `security` override | Effective security flows into the two-layer credential model: session-level via `config_for`, request-level via `SecurityProvider::apply`. §6. |
| TD carries config, not secrets (§5.3.3); Scripting API §11.1.4 forbids credential-query APIs | TD never carries secrets; credentials sourced from an app-injected `CredentialStore` (not script/TD-visible), split session-level (`ZenohSessionPolicy`) vs request-level (`apply`). §6.3. |
| §8.3.2 binding-template delegation | All session lifecycle semantics live in the zenoh binding crate, not in core/TD/shared. §5.1. |
| AGENTS.md no_std policy | Planning surface (`form.rs`) stays `no_std + alloc`; the `std` pool (`§5.2`) and `no_std` single-session backend (`§5.3`) are feature-selected behind the same `ZenohTransport` seam. §3.5, §5.1. |
| AGENTS.md protocol-neutral engine | `ClientBinding` trait unchanged; no session methods added. §1.3. |
| MQTT precedent (§A.2) | `zenoh://host:port/key` mirrors `mqtt://host:port/topic`. §2.1. |

This template brings the zenoh binding into alignment with WoT TD 1.1 design
intent: the TD becomes self-sufficient (a Consumer can locate the router from
`base`), multi-router Consumers are supported, and the protocol-neutral engine
stays unchanged.
