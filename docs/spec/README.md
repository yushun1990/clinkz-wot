# Domain specifications — v6.0

[authority.toml](authority.toml) is the single active source/requirement map.

| Specification | Exact responsibility |
| --- | --- |
| [TD](td.md) | Shared WoT interpretation, extension preservation/context support |
| [Planning](planning.md) | Selection/support, pure construction, deferred preparation, reuse and Producer coverage |
| [Interaction Core](interaction-core.md) | Identity, behavior, payload/security, Actions, notifications, collections and outcomes |
| [Binding SPI](binding-spi.md) | Open composition, native progress/matching, delivery and physical retirement |
| [Runtime safety](runtime-safety.md) | Permission, publication, cancellation, cleanup, concurrency and profiles |
| [Foundation](foundation.md) | Owner-scoped resources, linear work and environment limits |
| [Time amendment](../amendments/WP-100-time-domain-v1.md) | Retained exact clock/source-time semantics |

The [old authority map](v5-authority-reset.toml) is a historical tombstone.
[Consumer resource schema 2](consumer-resource-ownership.md), remaining amendments,
Property Read admission/gate/evidence records and numeric resource profiles have
only the explicit historical/restricted scope registered in the successor map.
They cannot override the general contracts above. No inactive v4.9 clause is
reactivated by a reference. Old physical API inventories are superseded, not
second active specifications.

The API matrix records one defining package/path per retained implementation and
marks restricted or removed shapes explicitly; it does not invent future Rust
names. General responsibility ownership is specified above. Exact trait layout,
erasure, schema storage, scheduling and static synchronization remain engineering
choices within these contracts. Review must challenge an actual conflict before
introducing another common abstraction.
