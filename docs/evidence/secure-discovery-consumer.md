# Secure Discovery-to-Thing Consumer composition

## Assessment

Basis: master `23f8d695784599361e3b070124ecba0244eac51c`, after #263, including
the reusable execution, structured DataSchema and HTTPS/Basic capabilities from
#260–#262. The existing public APIs compose the requested workflow. This change
adds a runnable application example and real interoperability/regression evidence;
it changes no production library API, semantic contract or architecture.

The assessment inspected Discovery admission/transport, TD semantics, open
Consumer preparation/selection, the HTTP compiler and access service, and finite
Consumer acceptance/result handoff. Existing individual capabilities and earlier
PR acceptance were hypotheses to exercise, not evidence of this joined workflow.
The applicable authority remains DISC-CLIENT-001, WOT-SEM/EXT-001,
PREP-SELECT/COVER-001, EXEC-SECURITY-001, SAFE-AUTH/CANCEL/CLEANUP-001 and
RES-OWNER/WORK-001 in their existing registered owners.

| Application step | Actual public composition and observed boundary |
| --- | --- |
| Configure retrieval | `TrustedDiscoveryEndpoint`, `DiscoveryTransport` and `DiscoveryCredentials` provide the Directory URL/socket pin, TLS trust and exact Basic audience. None is a Consumer capability. |
| Retrieve/admit | `HttpDiscoverySession::directory().next_page()` accepts a complete bounded array page through existing TD inspection/Basic admission. `DiscoveredThing` retains an immutable typed TD, including untrusted links and unsupported metadata. Admission is not selected semantic support. |
| Select | The application finds the exact Thing ID, supplies a Property name and zero-based Form index, and uses `ConsumerSelector { binding: Some(...), form: Some(...) }`. It passes the original TD intact, with its actual Forms and requirements. |
| Qualify | `ConsumerRegistry::prepare` resolves shared TD meaning and pairs actual services. `try_https_basic_consumer_registration` supplies separately provisioned Thing roots and `HttpBasicClientAccess`; the latter binds separate credentials, exact destination policy and original `ConsumerInstanceRef`. |
| Retain the selection | `PreparedConsumer::into_execution` yields owned bindings. The application retains only the binding matching the requested Property, Read operation and Form. An unrelated supported Property cannot stand in for an unsupported selection. |
| Execute/validate | `ConsumerRuntime::read_property` and `ConsumerCall::poll` check live authority/policy and perform real HTTPS. Captured shared schema validation checks the structured result. The source TD, Directory session and compiler can already be gone. |
| Release | Request sockets close on completion/refusal/cancellation/abandonment. A settlement retains original Engine funding and access owners until disposal, independently of Directory lifetime. |

No Directory URL, security definition, trust root, credential, ID or successful
retrieval becomes native execution permission. The application-supplied allowed
Thing URL is a constraint checked against the selected mapping, never a replacement
for its `href`. The test peers use different certificates/keys, listeners,
credentials and authentication/authorization checks even though both run on
loopback. Changing trust in either direction fails before HTTP credentials arrive.

## Interface judgment

The independent retrieval and interaction configurations are necessary security
boundaries. Sharing a Basic credential object, a TLS service, or a combined
"discovered authorized Thing" object would obscure those boundaries. Neither a
universal workflow facade nor a Discovery-to-Property-Read transport adapter is
needed. There is no source-TD callback, native target rewrite, synthetic TD,
compatibility wrapper or Core protocol branch in this implementation.

The finite preparation API is deliberately whole-description: its Form index
selector applies at every affordance and success means at least one supported
interaction. Thus successful `prepare` alone is insufficient for a request to
read one particular Property. Exact extraction from its public owned bindings
completes application selection; rejected/unselected artifacts drop normally.
The example exercises this with an additional supported `mirror` Property and
retains only `level`. No authority is acquired by that pure extra preparation.
Whole-description inspection, other same-index candidates and aggregate vector
funding remain part of this API's cost. This exercise establishes constructibility
and isolation within existing bounds, not optimal preparation cost for large TDs.
It supplies no measured reason to replace the aggregate API or add a cache.

`ConsumerSelector::default()` intentionally allows TD Form order as a preference;
it is unsuitable when the application has committed to a particular Form. The
example pins that Form before qualification and refuses if the corresponding
binding is absent. It never retries with another Form or weaker security.
Public support reports remain available to applications needing selection UI;
the example accepts an explicit ID/Property/Form decision as input.

The application still supplies resource/work limits, a manually driven Consumer,
and a policy implementation. These are explicit existing Host responsibilities,
not coupling to Directory internals. No architecture correction was justified by
the executable cases. This conclusion is scoped to the supported subset below.

## Executable evidence

[The runnable example](../../discovery/examples/secure_discover_read.rs) is compiled
directly into [the public-composition tests](../../discovery/tests/secure_workflow.rs),
so the exercised preparation/selection path is the application path. Ten tests
include multiple cases and real TLS exchanges:

- Authenticated listing with two Things, explicit second Form selection, separate
  Basic credentials, a structured object result, required fields, integer bounds
  and string lengths. Unselected Forms/ThingLinks receive no connection. Source,
  Directory and compiler disposal precedes native execution.
- Supported earlier Forms and an unrelated supported Property cannot substitute
  for selected bearer/simultaneous security, unsupported scope/coding/media/schema,
  cleartext, unresolved context, wrong operation or missing Form. Destination
  port/path mismatch refuses without contacting either target or rewriting the TD.
- Successful Discovery with missing Thing credentials, denied application policy,
  revoked authority or a distinct instance with the same slot/generation cannot
  start a read. Directory roots do not trust the Thing and Thing roots do not
  trust the Directory. Failed trust emits no HTTP request/credentials.
- Explicitly misprovisioning Directory credential bytes as Thing credentials
  produces the Thing peer's 401. Correct Thing credentials can still receive 403.
  A 302 cannot cause navigation or credential forwarding. No authentication
  challenge, redirect or alternate Form is retried.
- Authenticated HTTP 200 with out-of-range integers, missing required fields,
  excessive string length or wrong types fails the selected DataSchema.
- In-flight cancellation, policy denial, instance revocation, credential removal
  and abandonment close an actual stalled TLS response. The call has no Engine
  deadline; an external watchdog fails the test if an invalidation does not
  settle it. The four invalidation cases check the original terminal cause: `Cancelled`,
  `Security(AuthorizationDenied)`, `StaleHandle`, or `Validation` for a changed
  captured credential-service revision, respectively. Already validated but
  unread native success is fenced by policy denial/revision or original-instance
  revocation; native evidence and original funding remain until final disposal.
- Directory cancellation/abandonment during an authenticated partial page returns
  no descriptions and closes the socket without replay. A malformed TD from an
  authenticated Directory fails shared admission.

The Rust peers independently parse HTTP requests and check one exact Basic
credential field; they do not use a ClinkZ Directory or Producer. Peer completion
waits for client EOF/error and rejects any further request bytes on that same
connection. After the scripted exchanges, the listener remains open until the
caller checks final release after settlement. It fails on any extra connection,
checking queued connections before accepting the caller's stop signal. No-contact
assertions hold a separate listener open.
Every Engine case checks all original resource accounts after disposal; the
success case also observes final access/policy owner destruction. The unread-result
test delegates to the real validator and observes its completion to synchronize
the authority change, rather than guessing from elapsed time.

The original evidence revision `43a00421d7851a4734140e11ee39c68a5286502e`
could accept deadline expiry as cancellation/revocation evidence, and its peer
closed too early to detect a retry. Those assertions did not establish the
claimed regression protection. The corrected tests above were checked with
eight fault injections in a disposable detached checkout: separately removing
each of the four invalidations fails at the external watchdog; deliberately
retrying the failed HTTP exchange fails for each of 401, 403 and 302; sending a
second request on the original TLS connection fails the byte guard. The normal
tests pass with the production runtime unchanged. These mutations are evidence
of test sensitivity, not runtime behavior or a new production architecture claim.

[The independent Python/OpenSSL fixture](../../discovery/tests/support/secure_peers.py)
runs the actual example binary against two standard-library HTTPS servers. It
generates separate ephemeral self-signed server certificates with IP SAN and
server-auth usage, provisions each certificate as its own trust anchor, and
authenticates/authorizes each endpoint independently. It verifies:

1. Directory listing and Thing read return `{"temperature":23,"label":"office"}`.
2. A denied selected Form refuses without contacting a usable earlier Form.
3. Using the Directory root at the Thing fails before another HTTP request.
4. An authenticated value of 51 fails the selected maximum of 50.

The run ends with four Directory requests, two Thing HTTP requests, zero active
connections and closed listeners. Local execution used Python 3.14.8 and OpenSSL
3.5.9; no ClinkZ server or rustls server participates in that check. The initial
OpenSSL fixture used a CA certificate as the server end entity; normal rustls
verification refused it. Correcting the fixture's certificate role, without
changing verification or production code, made the external run pass.

Reproduce from the repository root:

```sh
cargo test --locked --offline -p clinkz-wot-discovery --all-targets -- --test-threads=1
python3 discovery/tests/support/secure_peers.py
```

To run against provisioned endpoints, supply `CLINKZ_DIRECTORY_USER`,
`CLINKZ_DIRECTORY_PASSWORD`, `CLINKZ_THING_USER` and `CLINKZ_THING_PASSWORD` from
the application environment, then:

```sh
cargo run --locked --offline -p clinkz-wot-discovery --example secure_discover_read -- \
  DIRECTORY_URL PINNED_SOCKET THING_ID PROPERTY FORM_INDEX ALLOWED_THING_URL \
  DIRECTORY_ROOT.der THING_ROOT.der
```

The example requires all four independently supplied values, accepts no password
arguments, and has no credential fallback. Local source buffers are zeroized;
this does not erase environment, TLS, allocator or kernel copies. Deployment
secret management remains an application responsibility. The Python fixture
uses only disposable test credentials and temporary key files.

## Limits and acceptance scope

This is Host HTTP/1.1, verified TLS 1.3, Basic in the Authorization header,
explicit IP-literal Thing targets, bounded JSON Property Read and the existing
supported structured DataSchema subset. Directory URL identity/socket pin and
Thing URL authorization stay separate. Existing target/body/TD/resource limits
in [Discovery evidence](remote-discovery.md) and
[HTTPS security evidence](td-security-https.md) still apply. Preparation repeats
local TD inspection after retrieval; no transferable semantic admission proof
or preparation-performance claim is introduced.

These independent wire/stack peers are focused interoperability fixtures, not
a cross-vendor WoT Directory/Thing certification suite. The Python fixture is
explicitly run, not automatically covered by Cargo tests; the existing TinyIoT
external test remains opt-in. Neither full strict JSON-LD ingestion, general
conformance, certificate chain/revocation-service deployment, transitive native
memory bounds nor broad production hardening is established here.

OAuth, other security schemes/placements, DNS discovery/resolution, automatic
ThingLink/context traversal, authenticated Write/Action/Event/collection paths,
redirects, pooling and a universal orchestration API remain outside this increment.
Revocation fences future acceptance/handoff; it cannot retract already submitted
request bytes or results already handed to the application. No milestone,
architecture gate, work package or release-readiness status changes.
