# Reusable Producer preparation and independent publication

Status: MIGRATED

## Reconstructed problem

At the PR #282 merge (`5c8fd09`), `PreparedThing` contains a complete selected
operation inventory with scope/Form/operation coordinates, provider indices and
owned native definitions. HTTP definitions include endpoints alongside response
instructions and schemas. Existing final checking avoids replacement schema
trees, but new compatible Things still lower the native response table and
compile owned validation data. Sharing whole prepared Things would also share
the wrong coordinates and fixed endpoints, without solving behavior, collision
or complete final-TD qualification.

Producer advertisement has a different boundary from Consumer selected
interactions. Every retained Form/effective operation and behavior must qualify
before one local publication commit. Consumer definition/association/capacity
types are therefore evidence of distinct lifetimes, not the Producer's required
representation. `ComposedProducerExposure` already isolates publication and
retirement over an original complete native membership; changing it to a shared
runtime would conflate owners that are currently correct.

## Alternatives and selected correction

| Alternative | Disposition |
| --- | --- |
| Share/clone the complete `PreparedThing` | Reject: coordinates, provider indices and bound native routes belong to a description; no implicit new-Thing qualification |
| Share only separately prepared schemas | Useful existing content reuse, insufficient to avoid native response instruction lowering |
| Compile every instance and intern equal output | Reject: identical bytes do not establish avoided compilation |
| Copy the Consumer association API or add a universal compiled program/cache | Reject: no demonstrated Producer need for those representations or cache policy |
| Extend a single-Property per-Thing facade into a shared runtime | Reject: publication membership, native progress and live authority remain independent |
| Share concrete immutable HTTP response/validation instructions; check and bind native route operands through the existing capability | Select: removes actual repeated lowering while retaining complete per-description qualification and native ownership |

The minimal proving path is Host cleartext NoSec HTTP Property Read with
structured value constraints and an additional response schema/native status.
The actual compiler performs all applicability checks for either fresh lowering
or supplied-definition binding. Borrowed common schema/response comparators and
native response-fact comparison check dependencies without a second stored
semantic projection. The source-free runtime retains concrete immutable
instructions, not a compiler, semantic arena or TD callback.

Native scheme/address/authority/path/query and Form are checked mapping operands;
Property name and actual registration identify each new recipient association.
Timeout, listener/correlation/parser ownership and collision inventory remain
native participation facts. Behavior/current authorization/publication and
unread result/cleanup remain separate instance/work owners. The immutable owner
retains original physical funding across all program and definition references.
No change to publication state transitions or new generic SPI facet is needed.

## Migration and evidence boundary

The selected semantics are owned by
[Planning](../docs/spec/planning.md#reusable-producer-preparation), the
[HTTP Binding contract](../docs/spec/binding-spi.md#reusable-http-producer-instructions)
and [Runtime Safety](../docs/spec/runtime-safety.md#reusable-producer-definition-ownership),
with API/lifecycle projections, WP-200/300/400/600 and PLAN updated.
[The evidence](../docs/evidence/producer-preparation-reuse.md) records the exact
scope and alternatives' falsification through real HTTP execution. Existing
specialized mappings remain supported. No package/milestone/gate closes, and no
independent acceptance or merge is initiated by this implementation.
