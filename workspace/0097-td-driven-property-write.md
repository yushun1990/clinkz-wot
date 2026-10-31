# TD-driven Property Write

Status: MIGRATED

## Finding and alternatives

The finite Consumer definition, request and seal admit only Read/Invoke. The
finite Producer already dispatches raw writes, but its captured service checks
only output; the HTTP definition and listener have one read-only mapping.
Admitting a write through those interfaces would skip required input checks or
reuse a read definition. Independent listeners cannot serve default Read/Write
operations on the same endpoint.

Duplicating write execution/ownership, treating writes as reads, and introducing
a universal HTTP-shaped exchange are rejected. Extend the existing finite
capabilities: operation-scoped Consumer definitions and a captured Producer
`FiniteValidator` with separate input/output roles, keyed by the original
candidate. One bounded HTTP participant may retain multiple native mappings;
native matching and reply framing stay in HTTP. Replace the output-only finite
validation API directly; the restricted legacy read adapter retains its scope.

## Selected boundary

Whole-description Producer qualification must cover every effective Form and
operation and actual behavior. Property declarations and explicit operation
applicability are enforced before effects. Required assertions outside the
existing type-only schema/NoSec subset refuse preparation, with no partial
Producer advertisement. Consumer alternatives may remain unsupported.

Write input validation precedes behavior; fresh application authorization and
configuration checks follow validation callbacks. Empty terminal application
success is distinct from native delivery and effect certainty. A handler error,
local cancellation, replacement, failed or ambiguous exchange cannot prove that
the remote write did not occur and never authorizes automatic replay.

HTTP uses operation defaults GET/PUT and a strictly empty 204 Write success.
The strict empty remainder is checked through peer EOF on the existing closing
connection. Hyper's header-terminator completion alone cannot establish it:
the bounded raw head stays private until EOF, then enters Hyper under the same
callback byte allowance. Waiting retains the original cancellable call/socket;
native read partitioning cannot change the completion or resource contract.
An HTTP 202, read-shaped 200, malformed framing, unexpected body or unsupported
response mapping grants no application success. This is a concrete supported
mapping, not a Core status convention or general interoperability claim.

## Affected authority and falsification

EXEC-DATA-001, EXEC-OUTCOME-001, PREP-SELECT-001/PREP-COVER-001, Binding SPI,
runtime safety, API ownership, WP-200/300/400/600 and PLAN capability coverage.
Prove real Engine-to-Engine Read/Write on one Property after preparation sources
drop; negative schema/security/access/applicability cases; cancellation,
replacement, late results, bounded unread ownership and final physical/account
release. Reaffirm existing finite read, Action and notification behavior.

## Disposition

The selected direction is implemented through the existing finite Consumer and
composed Producer owners. EXEC-DATA-001/EXEC-OUTCOME-001, Planning, Binding SPI,
runtime safety, API ownership, work packages and capability coverage now project
the same scoped contract. HTTP owns its declared native subset and integration
evidence in its README/tests. The independently linked Consumer fixture also
executes a Write using its own native completion convention; Core receives no
HTTP status interpretation. Existing raw and restricted Read adapters are migrated
directly to the finite validation API while retaining their qualification scope.
No package, architecture gate, milestone or production claim is closed.
