# Remote Discovery: assessment and executable evidence

Scope: the first explicit Host client increment under DISC-CLIENT-001 and v6 /
ADR-0025. [The client contract](../architecture/10-primary-data-flows.md#host-discovery-client-contract)
owns semantics; [WP-500](../work-packages/WP-500-discovery.md) owns admission and
remaining package scope. No milestone, broad conformance, performance or release
qualification follows from these tests.

## Standards and boundary assessment

Baseline: [W3C WoT Discovery Recommendation, 5 December 2023](https://www.w3.org/TR/2023/REC-wot-discovery-20231205/),
§6.1 (direct introduction), §7.2 (single TD GET) and §7.3.2.1.5 (array listing,
limit/offset, `next`, canonical collection and etag). This is the Recommendation,
not an editor's draft or a project-specific directory protocol.

The removed LocalDiscoverer refused both remote URLs and TD fetches. Its local
publication, lease, filtering and watch abstractions did not provide remote
retrieval or transport trust. Its ProcessState returned the first full TD and
silently lost later batch members. No production caller outside Discovery used
those APIs. Retaining the facade would preserve incorrect composition, with no
compatibility benefit in this unreleased project. Old in-memory tests are retired;
they are not transferred as acceptance evidence.

Alternatives considered in [the investigation](../../workspace/0110-remote-discovery-client.md):
repair the local facade, force listing into Consumer Property Read, or introduce
a universal retrieval SPI. The selected smaller boundary is one explicit native
HTTP client using existing Hyper, rustls and URI libraries. Its media/framing,
continuation and trust requirements differ from the existing HTTP WoT Read
adapter. Reusing that adapter would require changing operation meaning. Common
TD admission and subsequent Engine preparation are reused without adding protocol
cases to Core, Planning or Servient.

## Executable claims

`cargo test --locked --offline -p clinkz-wot-discovery --all-targets -- --test-threads=1`
runs independent raw socket/TLS transcript peers and the existing Engine:

| Evidence | Claim |
| --- | --- |
| `remote.rs` listing | Every member survives multi-member pages; base-relative and combined Link fields, quoted comma/semicolon, opaque session values, short intermediate/final pages, JSON and JSON-LD array media |
| Listing order | Unpaginated descending-ID arrays retain peer order with separate duplicate refusal; single/final paginated pages still enforce ascending IDs |
| Single TD test | HTTP TD-server GET, TD media, chunked framing and anonymous TD admission |
| Semantic/JSON refusal | Complete-page refusal for malformed/truncated JSON, duplicate decoded keys, invalid TD structure, undefined security and invalid operation context; no accepted prefix escapes |
| Navigation refusal | Foreign origin/path, userinfo, fragments, downgrade, unchanged offset, altered/duplicate/encoded limit parameters, ambiguous rel/anchor, missing canonical state, empty progress, repeated IDs and changed etag |
| Framing/media refusal | Unsupported media/encoding/charset, duplicate media and framing fields, mixed Content-Length/Transfer-Encoding, declared trailers, 302/401/403/503; no redirect/challenge retry |
| Bounds | TD/page/total input, result/page count and JSON depth; remaining session bytes constrain Content-Length and streamed chunks/EOF-delimited bodies before retention, including incomplete stalled responses; exhausted allowances acquire no socket and exact limits admit complete final pages; absolute session/request deadlines also bound a stalled TLS handshake |
| Interruption | Cancellation before connect, during a partial page and between pages; timeout, truncated peer close and dropped pending future; no replay and retained first terminal cause |
| `trust.rs` | Real TLS with a pinned socket and verified DNS or IP identity; exact Basic audience, missing/wrong credentials and 401/403; wrong roots/name/expiry transmit no HTTP credentials |
| `workflow.rs` | TLS/Basic Directory → admitted TDs → explicit application selection → actual Engine preparation → independently authorized HTTP Property Read against another peer; Directory credentials never reach that native peer |
| Unsupported execution | A structurally admitted Basic TD does not gain NoSec service support; denied interaction policy and unresolved remote contexts refuse preparation/calls without network effects; ThingLinks/context URLs are not fetched |
| Resource release | Transcript peers wait for client EOF/reset after success, refusal, cancellation, timeout and abandonment. Engine result backing retains original accounts after runtime disposal; all accounts reach zero on final result disposal |

The peers implement wire transcripts, independently of ClinkZ TD serving,
Directory, Planning or Binding handlers. They are not a project Directory server
or a cross-vendor certification suite. The test helper counts completed physical
connections and bounds its own waits.

The runnable application is
[`discover_read`](../../discovery/examples/discover_read.rs). It takes a separately
configured collection URL/socket, selected Thing ID, Property and allowed native
interaction URL. Retrieval trust does not populate the interaction policy.
Discovery/session/source/compiler disposal precedes actual Consumer execution.
Native interaction has a separate deadline and original-account release check.

## Independent Directory interoperability

The example at `c9f27de3ce5a64d933f64cd4748bb89bf7aa487e` was additionally
executed against unmodified
[TinyIoT Thing Directory source `fbec084f189d3c89183db7715805cddd93bba284`](https://github.com/TinyIoT/thing-directory/tree/fbec084f189d3c89183db7715805cddd93bba284),
built with Go 1.27.3 (`go build -mod=mod`). The server uses its own HTTP router,
Go JSON serialization and LevelDB store. Its sample configuration was restricted
to a temporary loopback listener/store, with DNS-SD/auth/TLS disabled and no
configured JSON validation schemas. No source adaptation or ClinkZ server was
used. The client therefore remained responsible for admission.

Three TDs (`urn:interop:a`, `b`, `c`) were registered by the fixture through
`PUT /things/{id}` with `application/td+json`; each returned 201. Each describes a
read-only integer `level` at an independently configured Python standard-library
HTTP peer. `GET /things?limit=16` returned 200, `application/ld+json` and all three
TDs. The example selected `urn:interop:c`, prepared the existing NoSec Consumer,
read `23` from that separate peer exactly once, and returned every Engine account
to zero. No Authorization field reached the native peer. An additional malformed
TD (missing required security/context) was accepted by that non-validating server
with 201; the same client workflow refused the complete page with
`InvalidThingDescription`, before any additional native request. The malformed
fixture was then deleted (204).

This TinyIoT revision does not supply Recommendation pagination Link fields.
The external result establishes bounded array listing and individual TD retrieval,
not third-party pagination interoperability. Pagination evidence comes from the
independent Recommendation transcripts. A peer that silently omits required
navigation can hide data; clients cannot independently prove inventory completeness.

Reproduction, after building that source and starting it with a loopback sample
configuration and an isolated store:

```sh
# Seed the three TDs above with separately configured actual native URLs.
# Use fewer than 16 records for this TinyIoT revision's listing evidence.
CLINKZ_TDD_URL=http://127.0.0.1:PORT/things \
CLINKZ_TDD_SOCKET=127.0.0.1:PORT \
CLINKZ_TDD_IDS=urn:interop:a,urn:interop:b,urn:interop:c \
cargo test --locked --offline -p clinkz-wot-discovery --test external -- --ignored

cargo run --locked --offline -p clinkz-wot-discovery --example discover_read -- \
  http://127.0.0.1:PORT/things 127.0.0.1:PORT \
  urn:interop:c level http://127.0.0.1:NATIVE_PORT/level
```

`external.rs` is intentionally opt-in and checks the exact configured inventory,
then retrieves each independently configured fixture TD URL. Network-free or
fixture-only CI must not claim this external check was executed.

## Actual resource and security boundary

| Enforcing owner | Bound / lifetime |
| --- | --- |
| Endpoint/session | URL/reference ≤ 2048 bytes; configured pin; TLS 1.3 verified host; ≤ 8 roots of ≤ 16 KiB; Basic username ≤ 64 and password ≤ 128 ASCII bytes |
| Native request | 64 headers / 16 KiB raw head; aggregate encrypted/plaintext socket traffic ≤ twice page_bytes + 64 KiB, including TLS handshake and chunk overhead; no pooling/tasks/retry; request and session deadlines |
| Page | Default body 256 KiB, TD 32 KiB; JSON depth 32, 8192 visited values and 1024 members per object; duplicate decoded names refuse; no decompression, alternative envelope or partial delivery |
| Session | Default 16 pages, 128 results, 1 MiB total bodies, 16 requested members/page, 10 s/request and 60 s/session; the body reader enforces min(page_bytes, remaining total_bytes) during retrieval; constructors reject zero or excessive policies |
| TD admission | Existing GatewayDefaultV1 inspection/Basic limits, 8 MiB temporary ledger ceiling and one nonrenewable million credits/class/TD; yield/check cancellation between bounded steps; proof/scratch released before returning the immutable Thing |
| Returned description | Application-owned typed TD; its allocation/traversal is bounded by wire structure and TD policy. It carries no prepared interaction capability or permission |
| Engine execution | Existing registration/data/access/preparation/call/result owners and original resource accounts; independent of Directory session or credentials |

These are enforcing capacities/work bounds, not measured additive RAM or a
transitive allocator/OS/rustls maximum. TLS/Hyper/kernel storage belongs to the
native request and deployment; application concurrency across sessions and
retained pages is funded/contained by the application. Unpaginated duplicate
checking retains only borrowed ID references in a set bounded by the admitted
page's result cap; paginated ordering needs only the previous ID. Allocation
failure follows the existing Host allocator policy. Parsing and crypto are
cooperative bounded calls, not preemptible hard-real-time work. Credential source/encoding scratch
is zeroized, native header/kernel copies have no process-wide erasure guarantee.
Diagnostics retain fixed categories and status codes, without remote text or URLs.

## Remaining scope

The remote client is Host-only. Without `http`, the crate provides error/result
values and no remote/no_std discovery execution claim. DNS resolution is an
application deployment step: every URL has an explicit socket pin, while TLS
checks its own URL identity. Root/credential rotation uses a new session; there
is no shared mutable credential manager. TLS 1.2, mTLS, proxies, revocation
fetches, redirects, bearer/OAuth negotiation, content compression and other
protocols are not implemented.

Array listing and TD-server retrieval are supported. Automatic Directory-TD or
ThingLink traversal, context fetching, search, collection envelopes, watches,
registration, storage, snapshots and resumable sessions are not implemented.
The TD layer's existing typed Basic/inspection semantics apply; this is not a
new complete strict-ingestion or JSON-LD processor. Selected semantics and all
native interaction support still require actual Engine preparation and current
instance permission. Unsupported extensions cannot grant execution authority.
The example's native operation is the existing NoSec cleartext-IP Property Read;
Discovery TLS/Basic is exercised in the composed test and against independent TLS peers. General
cross-vendor authenticated Directory-to-authenticated-Thing qualification remains
open. Independent architecture/public API acceptance is required before merge.
