# TD-driven Thing-root WriteMultipleProperties

Status: MIGRATED

TD 1.1's root operation writes caller-selected writable Properties in one
interaction. The member values have independent Property schemas. ReadOnly
members are ineligible; writeOnly members remain writable. Neither an all-write
schema nor a selected-read request is the invocation contract: derive the set
from the validated input map, retain it independently, and validate the entire
input before group authorization and any effect. Empty selection is permitted
by this bounded subset and still makes one native interaction.

Reusing finite ownership is appropriate; combining the two existing collection
implementations without changing result semantics is not. The current empty
success/error rule loses legitimate native member evidence. A concrete contrast
is [RFC 4918 section 9.2](https://www.rfc-editor.org/rfc/rfc4918#section-9.2): PROPPATCH has an atomic application contract and a 207
response with per-property statuses, including failed dependencies. This is not
our implemented mapping, but disproves empty success as a universal write SPI.
A nontransactional group can instead complete one member, reject another and
leave a third uncertain. One aggregate error cannot preserve that distinction.

Retain an optional bounded PropertyWriteReport in the existing output metadata:
original-schema indices, completed/not-applied/unknown outcomes, optional opaque
16-bit native member statuses, and explicit atomicity evidence. No native codes
are interpreted by Core. Reports must match the original invocation set;
contradictory atomic outcomes refuse validation. Incomplete reports remain native
evidence, not whole-group application success. First local cause, native report,
effect classification and physical delivery remain separate through cancellation,
replacement, late validation and unread settlement. Inline report storage uses
the existing size-based funding and adds no variable destruction obligation.
The bounded carrier does not claim arbitrary protocol diagnostic preservation.

Select HTTP PUT/204 with a JSON object, following TD 1.1's HTTP method default and
the 2025-11-04 WoT Profiles Working Draft's selected-write mapping. The draft is
mapping guidance, not a TD Recommendation or a profile-conformance claim. HTTP
204 cannot encode a member report or establish atomicity; this Binding refuses
such Producer reports rather than discarding them. The native report contrast is
tested through an independently Cargo-linked datagram fixture, not advertised as
a production protocol. No WebDAV support is claimed.

Rejected: implicit N-WriteProperty lowering (changes group semantics); narrowing
the shared SPI to HTTP's empty reply (loses evidence); a universal transaction,
rollback, receipt payload, session or collection runtime (no demonstrated need).
Reuse existing captured service and finite lifetime/resource owners, with one
distinct root behavior and no automatic replay or readback.

Affected owners: Core collection/data/result contracts, TD semantic loans,
Planning, Binding SPI, runtime safety, API ownership, WP-200/300/400/600, HTTP
mapping and capability coverage. Exact executable evidence belongs to tests and
the PR; this investigation grants no independent acceptance.

The migrated contracts live in the existing Core, Planning, Binding SPI and
runtime-safety specifications; the work packages and API projection register
the implemented slice. No separate ADR or universal abstraction is required for
this bounded correction to the existing collection result contract.
