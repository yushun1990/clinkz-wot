# ClinkZ-WoT v6.0 architecture authority

This tree has one architecture target: **a semantic Engine with separate
execution capabilities over Binding-owned protocol runtimes**. The source map
is [authority.toml](spec/authority.toml); [ADR-0025](ADRs/0025-semantic-engine-and-binding-runtimes.org)
records the decision and the dispositions of earlier decisions.

`AUTH-REV-001`: The registered v6.0 sources are the sole target authority in this
tree. The complete migration becomes mainline authority in one atomic merge of
the independently reviewed tree. Before that merge, it is the proposed successor
on the task branch; master retains its own authority. There is no per-document,
per-feature or runtime switch that activates fragments of v6.0. Review and merge
facts belong to Git/GitHub, not to a copied acceptance declaration here.

The prior baseline is immutable Git revision
`ed09f059e383b7a1b9d22902ee29aa218b4471a6` (the full revision is pinned in the
source map). The v5.1 activation and its 121 inherited requirement dispositions
remain historical provenance. They neither bound the successor identity set nor
authorize new implementation. The source map explicitly disposes every inherited
identity. Retained exact requirements keep their owners; changed meanings receive
new identities. The requirements CSV is metadata, including historical rows;
only the source map's active set is current authority.

`AUTH-EVIDENCE-001`: Architecture acceptance, implementation admission, tested
behavior, product qualification and release acceptance are distinct claims.
Historical tests, measurements, admissions and passed Property Read gates retain
their revision and scope. They MUST NOT satisfy successor contracts by inherited
status. Reusing an invariant test requires checking that its oracle still tests
the successor meaning. The existing design checker validates projections; it
cannot accept this architecture or qualify an implementation.

## Reading the authority

Architecture governance owns change control. ADRs own rationale and supersession.
The [backbone](architecture/README.md) owns component relationships and primary
flows. Registered domain specifications own exact behavioral requirements.
API/state/resource/work-package/performance records project those requirements;
code and tests establish availability. A conflict requires correcting its owners,
not choosing a convenient paragraph or adding a precedence exception.

Uppercase MUST, MUST NOT, SHOULD and MAY use RFC 2119/8174 meanings. Stable
requirement identities identify contracts, not paragraph numbers or task status.
New source work follows ADR-0013 and the revised work-package dependencies.
This migration admits no general runtime implementation and adds no protocol
count, broad completion, or universal generality gate. Disjoint qualified work,
including narrow HTTP HTTPS/security and Discovery client work, can proceed
under the applicable successor contracts and proportional review.

## Standards and scope

Use dated W3C [WoT Architecture 1.1](https://www.w3.org/TR/2023/REC-wot-architecture11-20231205/),
[TD 1.1](https://www.w3.org/TR/2023/REC-wot-thing-description11-20231205/) and
[Discovery](https://www.w3.org/TR/2023/REC-wot-discovery-20231205/)
Recommendations. The [Scripting API](https://www.w3.org/TR/2023/NOTE-wot-scripting-api-20231003/)
is a Group Note and semantic/API guidance, not a requirement to reproduce WebIDL.
The [Binding Templates retirement notice](https://www.w3.org/TR/2025/NOTE-wot-binding-templates-20251104/)
is a Group Note. TD 2.0 and the Binding Registry's draft material do not replace
TD 1.1. Each Binding names its actual mapping specification and version.
WoT does not prescribe our Rust SPI, resource accounts or publication phases.

The enduring [product goals](architecture/00-system-goals-and-context.md) retain
both roles, Property/Action/Event families, described collections, security,
Discovery and constrained execution. Host HTTP/HTTPS is the first production
direction. A Directory service, runtime Rust plugin loading, universal executor,
transport/session VM and transparent live-handle migration remain outside v1.
