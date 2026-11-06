# Executable TD Security and Host HTTPS

## Assessment and selected boundary

Assessment basis: master `1893d11403965520b7b4e3563851ea649b4ab65d` after
#260/#261, v6.0 / ADR-0025. This increment exercises finite Property Read over
HTTP/1.1 and TLS 1.3, JSON, IP literal endpoints and loopback Producer listeners.
It does not close a milestone or establish a general production HTTPS profile.

| Boundary | Source finding and disposition |
| --- | --- |
| TD | `SecurityFacts` already supplies effective Form override and defaulted Basic operands. Preserve shared interpretation; reject unsupported complete expressions rather than choosing a weaker branch. |
| Planning | Open Consumer qualification hardcoded NoSec before asking actual services. Replace this with immutable requirement preparation plus actual access-service and native compiler qualification. |
| Core / #260 content | `ConsumerExecutionContent` contained only data/operation meaning. Include selected immutable security meaning in checked reuse. Static scheme/parameter records are this bounded preparation adapter, not a universal expression engine. Core does not branch on Basic, HTTP, TLS or credential bytes. |
| Consumer instance | Access services and accounts belong to instance bindings. A Binding-specific opaque capability belongs to a call. Credential replacement changes the captured dependency revision and fences old bindings/calls/unread results; a new binding can use the same qualified immutable content. |
| Servient | Finite dispatch trusted the native input principal and did not reauthorize delivery. Add a captured, prepaid Binding verification callback. Security-bearing exchanges establish identity before authorization and repeat verification after authorization and around fresh permission before each successful native send. Changed identity/claims, policy or original instance authority fences delivery while retaining the original result. |
| Binding | HTTP owns certificate configuration, trust anchors, IP SAN/chain/expiry checks, TLS records, Authorization placement, evidence parsing and HTTP refusals. Native reply ownership retains unverified credentials outside application input. |

Basic is useful because it tests a complete deployed password-based interaction
without inventing token issuance, refresh, federation or a global session manager.
Its transport is [RFC 7617](https://www.rfc-editor.org/rfc/rfc7617); the TD meaning
and default placement come from the
[TD 1.1 Recommendation](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/#basicsecurityscheme).
Basic encoding provides no confidentiality; this adapter requires verified TLS
before credential encoding/use. The selected single requirement is enough to
falsify credential-in-definition, username-as-principal, unverified destination,
service-tag-only pairing and shared-instance-authority designs.

Rejected alternatives: store an Authorization header/password in a prepared
artifact; authenticate by interpreting a username as identity; make Core own an
HTTP authentication enum; add a credential/session manager; implement only a raw
Producer demonstration without testing checked Consumer reuse. Neither #260's
concrete type hierarchy nor #261's schema representation is an API freeze.

## Actual guarantees

- `planning::security::prepare_security` resolves the complete effective selected
  requirement into NoSec or Basic/header Authorization meaning. Explicit default
  header spelling compares case-insensitively. Form override replaces inheritance.
  A simultaneous list, combo allOf/oneOf, empty effective requirement, other scheme,
  proxy, scopes, extension assertion or other credential placement refuses. No
  alternative is silently dropped or selected as a downgrade.
- Complete Producer qualification covers every retained Form/operation. The
  actual startup verifier/key owner issues an unforgeable private registration
  association; prepared programs retain only that association and immutable
  meaning, without a key, verifier or credential. Runtime configuration must
  pair that same actual owner; a second service with equal tags is insufficient.
  Legacy retained Producer/Action paths refuse the new transport profile.
- Consumer registration pairs the real TLS trust owner, data service, native
  compiler/executor and Basic access service. NoSec-only services cannot bind
  Basic content. Per-call grants bind the original instance reference, selected
  registration, exact destination, Property Read and credential revision. No
  redirect, proxy, continuation, implicit retry or target rewrite exists.
  Permission and credential issuance each prepay their complete access-service
  work independently. Insufficient issuance credit preserves the original input
  and never enters issuance or native construction.
  The Basic adapter maintains one nonwrapping local invalidation sequence for
  credential replacement and observed wrapped-policy revisions. It checks the
  captured sequence around delegated permission/issuance and again in grants,
  credential transport and unread handoff. Wrapped metadata is read outside the
  credential lock; counter exhaustion fences the owner. Freshly binding the same
  definition can adopt the new revision without refreshing old work.
- A successful verifier returns an established `Principal`; the username is
  only evidence. Application authorization runs separately. Missing and invalid
  credentials yield HTTP 401 with a Basic challenge; application denial yields
  403. Malformed/duplicate/unsupported credential headers abort the exchange.
  After the last delivery verifier/authorization callback, the original accepted
  instance, publication gate and actual service/configuration are rechecked
  immediately before native send. Callback-time revocation fences retained output.
  Dispatch separately rejects a preclosed publication gate without invoking
  callbacks or consuming their work. It rechecks the original instance,
  publication and actual Binding/service pair after the last callback and before
  behavior acceptance. An invalidated candidate retains its original native
  exchange for funded retirement with NotInvoked; a later response refusal is
  insufficient to establish that no unauthorized behavior ran.
  Producer verification failure never enters behavior. TLS trust/handshake
  failure yields a redacted provider failure and cannot use credentials.
- Credential-bearing grants and Basic/authentication carriers have redacted
  formatting. Public errors carry bounded reasons only; external verifier error
  context is discarded. No credential logging is added. Owned Basic source and
  encoding scratch use zeroizing disposal; this is not a guarantee about all
  Hyper, rustls, allocator, kernel or process-memory copies.
- No listener, handshake, credential acquisition/use or behavior call occurs
  during preparation. Source TD/compiler retirement does not affect execution.
  Definitions, logical authority, call/result funding and native sockets have
  separate lifetimes. Retained late output keeps its original account until
  release even when delivery is refused.

## Executable evidence

`protocol-bindings/protocols/http/tests/security_https.rs` exercises real sockets
and TLS, qualified whole-Thing publication and Engine Consumers. It also uses an
independent rustls HTTP peer. Tests cover:

- Two endpoints share the exact prepared security/data content with zero content
  reconstructions on checked reuse, while credentials, policy, authority and
  original resource accounts remain independent. Verified identity differs from
  the transmitted username.
- Consumer and Producer refuse unsupported complete security expressions,
  simultaneous requirements, combo alternatives/conjunctions, other schemes,
  locations, scopes, proxies, extensions and cleartext. A mixed Read/Write TD
  cannot publish through this Read-only security provider. Form override and
  default Basic header semantics execute.
- Missing credentials refuse before Consumer network effects; a wrong-instance
  service refuses; rebinding a replacement instance succeeds. An independent
  TLS peer receives 401/403/200, and credentials do not appear in diagnostics.
- Wrong roots, wrong IP SAN and expired certificate fail before verifier and
  application dispatch. Revocation before acceptance produces no behavior effect.
- Revocation after acceptance, changed application policy, old-instance revocation
  or replacement, verifier changes inside permission callbacks, changed verifier
  work/identity and exposure cancellation refuse delivery of retained output.
  Consumer credential replacement and policy changes fence original work and
  unread results.
- Review regressions exercise real TLS delivery driven independently of the
  logical owner, with authorization or final-verification callbacks revoking
  that owner or closing its publication gate. The original output retires as
  NotDelivered; all original accounts release. A 4096-unit policy cannot issue credentials under one
  permission payment; exact two-callback credit succeeds. A separate native-start
  probe checks original Write input recovery and zero issuance/native calls on
  credit shortage. Wrapped-policy changes fence an already validated unread TLS
  result and callback-time issuance, while checked rebinding succeeds.
- Five dispatch regressions use real TLS to close publication before dispatch,
  during authorization, during either verification callback, or change verifier
  work during final verification. Each reproduced unauthorized handler entry
  before the repair; each now proves zero handler calls, NotInvoked/NotDelivered,
  retained original recipient/operation/Binding and final resource release.
  A preclosed gate fences even with zero callback/cleanup credit, while native
  retirement stays pending until its original cleanup allowance is available.
- Zero verification credit does not call the verifier. Cancellation before native
  progress, before dispatch and after acceptance settles the original call.
  Closed sockets/listeners and all tracked Engine accounts release; a retained
  result keeps its original funding until dropped.

These are focused integration guarantees, not a full WoT interoperability suite.
The application supplies the credential verifier and its actual verification
algorithm; the fixture's comparison is test-only, not a password database design.

## Resource and transport limits

ASCII Basic username 1–64 bytes (no colon), password 1–128 bytes, strict canonical
base64, one Authorization header. Verifier output is limited to 128 identity
bytes, 16 scopes of at most 64 bytes and 2048 bytes of actual indirect backing.
Verifier work is explicitly captured and checked; callback work is prepaid.
The existing Host callback model remains cooperative, with no mid-crypto or
application callback preemption. Revocation cannot retract bytes already submitted; partial delivery remains
Unknown. Principal/evidence overlap has conservative
Engine backing; call grants retain their original service owner/account.

Existing HTTP limits remain: target 64 bytes, head 8192 bytes, Producer 16 headers,
body 4096 bytes, one exchange/listener owner, manually driven progress. TLS wire
I/O is additionally limited to 4096 bytes per direction per Producer poll and a
shared 4096-byte Consumer callback credit. TLS records/handshake/crypto allocation
and native key/trust/verifier backing belong to the concrete Host service/native
owner, not a transitive Engine aggregate-RAM or embedded allocation-free claim.
Cancellation/Drop performs finite local disposal without network waits or tasks.

TLS 1.3 uses rustls 0.23.46/ring, explicit caller-provisioned trust anchors, normal
certificate chain/validity/IP verification, no dangerous verifier injection,
client certificates, early data, automatic root discovery or resumption. ALPN is
HTTP/1.1. The caller remains responsible for provisioning current trust roots.
Certificate CRL/OCSP revocation and root rotation in an existing registration are
not implemented; replacing the registration is a new trust domain.

Remaining unsupported: authenticated Write/collection/Action/notification roles,
all other WoT security schemes/compositions/placements, OAuth/token refresh,
TLS 1.2/mTLS/PSK, DNS/proxies/redirects, non-loopback Producer deployment,
connection pooling, multiple concurrent exchanges, broad production hardening,
constrained TLS execution and independent acceptance of a production profile.
Existing cleartext NoSec capabilities retain their narrower explicit permission
and policy contracts. NoSec execution does not gain Basic or HTTPS support.
