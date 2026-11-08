The singleton/composition refusal limits in this initial increment are superseded
by [the composition correction](security-composition.md). Its Basic/TLS execution,
authority and ownership evidence remains scoped below; current API names and
supported combinations are owned by the HTTP Binding documentation.

# Executable TD Security and Host HTTPS

## Assessment and selected boundary

Assessment basis: master `1893d11403965520b7b4e3563851ea649b4ab65d` after
#260/#261, v6.0 / ADR-0025. The original increment exercises finite Property Read over
HTTP/1.1 and TLS 1.3. The Property Write extension was independently assessed
against master `e267ac6241c11692feef986e994e2f501425bd09` after #265; it adds
structured JSON Write on the same finite owners, IP literal endpoints and
loopback Producer listeners.
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
  registration, exact destination, exact Property Read or Write operation and credential revision. No
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
  qualifies both operations independently; adding an unsupported notification
  operation refuses complete Producer publication. Form override and
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

## Secure effectful Property Write

The [boundary reassessment](../../workspace/0112-secure-property-write.md)
identified two concrete gaps, independent of TLS/HTTP mechanics: Property Write
lost validated completion evidence in its effect field, and the final verifier
could invalidate earlier application authorization while returning the same
Principal. The existing finite execution and structured schema boundary remain
suitable. The Core outcome and security contracts now describe completion and
passive service revisions; the Binding still owns trust, credentials and HTTP.
Finite dispatch also removes an unnecessary exclusive logical-owner borrow.
Callback-safe one-way instance revocation uses the existing currentness cell and
retains its original funding, without granting replacement/dispatch authority.
No universal security service or parallel Write runtime was added.

The public [joined Producer regression](../../protocol-bindings/protocols/http/tests/joined_producer.rs)
also invalidates application policy inside authorization. Its funded handler
adapter forwards the actual revision; the old allowed decision cannot enter
behavior or deliver a result. [Action kernel](../../servient/finite/tests/action_lifecycle.rs)
and [qualified HTTP Action regressions](../../protocol-bindings/protocols/http/tests/action_producer.rs)
revoke the original instance during invoke/query/cancel authorization. They fence
new invocation and public control, retain already accepted work and results,
and release the original capacity after settlement and final control disposal.
These are existing NoSec Actions; authenticated Action support remains excluded.

`security_write` uses the same publication/security fixture as `security_https`.
It prepares and completely qualifies a Producer with default Read/Write Forms,
then executes actual Engine Consumers after preparation sources retire. It
checks the original Principal (different from the Basic username), recipient,
operation, Binding and retained accounts. Its falsifiable cases cover:

- Structured nested values and a subsequent Read on the same native listener;
  two independently credentialed Write instances share the exact immutable
  content. Revoking one leaves the other executable; fresh checked binding can
  reuse that content without adopting old authority.
- Invalid required fields, nested strings, types, bounds and duplicate members
  return original Consumer input before native work. An independent rustls client
  sends the same invalid data to the Engine Producer, proving pre-effect checks
  there independently. Wrong/missing credentials yield 401, verified but denied
  principals/policy yield 403, and neither invokes behavior. An untrusted root
  fails TLS before verifier/behavior. Forbidden destinations fail preparation;
  changed application policy refuses original input before network effects.
- Authorization/final-verification callbacks close publication, revoke the
  verifier, change application policy, or change the verifier revision.
  Pre-dispatch refusal is NotInvoked. After behavior completion these changes
  fence delivery while preserving Completed and the original result/identity.
  Passive revisions must be non-reused and must cover actual mutable policy;
  they are sampled around callbacks, not treated as credential verification.
  With just the final application-revision comparisons disabled, the two TLS
  callback regressions reproduce unauthorized behavior entry and successful
  delivery after policy revocation; the comparisons fence both counterexamples.
  A one-way Producer revocation handle also invalidates the original instance
  from either callback, without taking a registry borrow.
- An explicitly lost Producer response after effects leaves Producer Completed /
  NotDelivered and Consumer Unknown. Handler error or invalid output after state
  mutation leaves both sides uncertain. An independent TLS server applies a PUT
  and replies with valid 204, 202, 403, trailing data or no response. Only the
  supported complete empty-204/EOF mapping yields Consumer Completed; no error
  causes another connection or automatic replay.
- Cancellation before network progress and after effects, plus reentrant
  Consumer permission callbacks that cancel and drive the occupied call,
  preserve original owners. A validated unread completion stays Completed even
  when callback-time cancellation refuses handoff. Socket/listener disposal and
  all tracked accounts are checked separately from effect/delivery outcomes.
  Credential rotation and original Consumer revocation during the unread check
  likewise retain Completed while refusing handoff. A retained Producer control
  keeps its original capacity after listener/result/registry disposal until its
  last release. Independent TCP/UDP finite regressions additionally exercise
  sibling dispatch reentrancy and an old handle after same-slot replacement.
- Full final-TD qualification still refuses authenticated collections, Actions,
  observation, unsupported response/schema/security assertions, mixed cleartext
  Forms and contradictory operation applicability. Basic is not an all-operation
  support certificate. Existing Read trust/identity/work/cleanup regressions are
  reaffirmed through the shared fixture.

The Property Write completion change also applies to the cleartext finite path
and independent downstream test Binding. Their prior Unknown assertions for valid
Write completion are superseded; malformed/ambiguous/error cases remain Unknown.
Read/Action and incomplete collection report semantics remain unchanged. No HTTP
status is interpreted in Core, and no new native error implies remote nonexecution.
These tests establish a bounded Host/manual capability, not production security
qualification, physical-state confirmation, atomicity, rollback or remote cancellation.

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

Remaining unsupported: authenticated collection/Action/notification roles,
all other WoT security schemes/compositions/placements, OAuth/token refresh,
TLS 1.2/mTLS/PSK, DNS/proxies/redirects, non-loopback Producer deployment,
connection pooling, multiple concurrent exchanges, broad production hardening,
constrained TLS execution and independent acceptance of a production profile.
Existing cleartext NoSec capabilities retain their narrower explicit permission
and policy contracts. NoSec execution does not gain Basic or HTTPS support.
