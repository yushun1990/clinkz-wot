# Remote Discovery client

Status: MIGRATED

The v6 authority is DISC-CLIENT-001 (primary flows), WOT-SEM/EXT-001,
SAFE-AUTH/CANCEL/CLEANUP-001 and RES-OWNER/WORK-001. The 2023 W3C Discovery
Recommendation §§6.1, 7.2 and 7.3.2.1.5 defines direct introduction, single-TD
GET and Directory listing/pagination. It does not make Directory metadata an
authorization grant or require retrieval to be a Property Read.

The inspected facade cannot retrieve remote descriptions. LocalDiscoverer
rejects URL directories and TD fetches, while ProcessState::next discards all
but the first full TD in a batch. Its publication, lease, storage, filtering and
watch APIs encode an in-process service, not a remote client contract. No
production caller outside Discovery uses those APIs. Keeping them would create
two competing discovery models and preserve the wrong default composition.

Replace that implementation with a small explicit Host HTTP client, retaining
portable error/result meaning. Directly configured endpoint, pinned destination,
transport trust and optional credential audience precede any network work.
Single TD retrieval and array listing use HTTP GET; a bounded session owns
continuations. Relative links resolve against the Directory API base, with the
same origin and exact collection path, unchanged limit, progressing offset and
stable canonical collection/etag. Reject unsupported representations rather
than infer a new protocol. Do not follow ThingLinks or TD Forms implicitly.

Use Hyper/rustls for framing/TLS, fluent-uri for RFC 3986 resolution, and existing
bounded TD admission for common validation. Native HTTP retrieval has different
media, status, pagination and trust contracts from the existing HTTP Property
Read adapter; sharing that adapter would silently change its operation meaning.
Neither a new universal transport SPI nor a Directory server is warranted.

The application selects an admitted immutable description, then supplies actual
Engine registration, instance, destination policy, credentials and funding.
Admission does not assert selected interaction support. Connection futures have
no detached driver; cancellation, timeout and abandonment drop the actual socket.
Interrupted page attempts terminate without replay. Complete pages already
returned remain application-owned; an incomplete page returns no partial TDs.

Falsifiable evidence: real HTTP/TLS Directory and TD-server peers, multi-item
and partial final pages, hostile links, malformed/oversized descriptions,
unsupported media, credentials/trust refusal, interruption and final release;
then explicit application selection and real Engine Consumer execution against
a separate peer. Independent acceptance remains a separate fresh review.

The selected contract is migrated to [primary flows](../docs/architecture/10-primary-data-flows.md#host-discovery-client-contract)
and [WP-500](../docs/work-packages/WP-500-discovery.md);
[scoped evidence](../docs/evidence/remote-discovery.md) owns implementation characterization.
