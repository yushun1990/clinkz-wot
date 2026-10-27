# Joined Producer publication over native finite execution

Status: MIGRATED

## Source reconstruction and alternatives

Basis: master `cdeeaa7` after #235/#236. Accepted owners remain PREP-COVER-001
in Planning, SAFE-PUBLISH-001/SAFE-AUTH-001 in Runtime Safety and the native
responsibilities in Binding SPI. No conflict in those contracts was found and
no architecture revision is changed by this implementation.

TD's `ValidatedProducerCursor` enumerates every scope/Form and effective
operation, including unsupported siblings. Its `with_property_semantics` lends
the TD-owned resolved extension/schema/security tree. Planning's existing
`ProducerPropertyReadAdmission` checks complete single-Property shape; its Covered
marker does not establish native/service support. `ProducerBindingPreparation`
captures the old `ProducerExecutor` and handler into `PreparedProducer`, consumed
by the independent `ProducerInstallationRuntime`. That stack has its own
instance, request/response ABI, dispatch and funded native owner. Passing it into
the finite runtime would duplicate acceptance and behavior ownership.

`RawProducerServient` already owns logical currentness/behavior independently of
the Binding. `RawFiniteRuntime` retains the actual native configuration, original
input/reply/result, deadlines and settlement. Its raw activation has no final TD
inventory, publication or aggregate account claim. These missing obligations
belong at the preparation/execution join, not in native matching.

The executable choices were compared before selecting a participant:

| Path | Disposition |
| --- | --- |
| Legacy HTTP installer wrapped by raw finite execution | Reject: two logical/dispatch owners and incompatible request/permit/reply ABIs. Its historical tests remain independent regressions. |
| TCP/datagram Bindings in finite integration tests | Retain for real I/O and shared-resource/currentness regressions. They do not yet supply complete TD protocol, codec/schema/security preparation. Making one a qualified protocol would add unrelated mapping semantics. |
| Raw Action or Consumer notification kernels | Different controls and lifetimes; no real Producer mapping/preparation participant in these kernels. They cannot stand in for publication. |
| Direct `HttpFiniteBinding` | Select for this finite slice: real socket/parser/reply owner, bounded local abort, actual JSON/type validator and source-independent operands already exist. No general SPI is inferred from HTTP. |
| Universal exposure certificate/cache/session/matcher | Reject: one completely supported mapping requires none of these. |

## Decision and migration

Use a frozen actual preparation provider in the optional Servient facade. Reuse
whole-description TD interpretation and the complete shape accumulator; add an
open finite preparation facet that constructs the real Binding and checks the
same provider again after closed installation. Keep runtime types/Core facets
free of TD/Planning. The HTTP implementation uses its pure mapping compiler,
without calling the legacy executor/installer.

The exact immutable retained JSON is the final description in this slice.
Explicit endpoints are checked against native `local_addr`; port-0 rewriting and
multi-participant publication remain unsupported. Every retained declaration
passes the entire process twice, before setup and before publication. No draft
coverage marker alone can open the runtime.

Stage the description, actual behavior, authority and storage before native
effects. The source-free registry uses one shared logical permission cell so a
lost publication guard revokes that exact incarnation. After final checks, the
commit only installs preallocated references/flags and opens the finite runtime.
Keep displaced owners outside the commit; old accepted work retains its actual
instance. Native setup, endpoint probes, codec callbacks, allocation and behavior
destruction occur outside that commit.

Detailed implemented scope, funding, resource-policy limits and executable
boundaries are migrated to WP-200, WP-400, WP-600, the API ownership projection,
source and the `joined_producer` target. No roadmap, gate status, legacy evidence
or milestone acceptance is changed. Exact PR/CI/review state remains in GitHub.
