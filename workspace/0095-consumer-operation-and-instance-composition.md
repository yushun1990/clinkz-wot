# Consumer operation and instance composition

Status: MIGRATED

Source basis: merged PR #242, `5623f67`. The open compiler is usable, but
`open_consumer` selects one definition per name, Core definitions have no
operation, `OutboundRequest` always reads, and the finite owner authorizes and
validates a Property value. These are structural restrictions, not missing
Binding implementations. An Action and Property with the same name collide.

Finite and notification roots independently manufacture currentness. Neither
can participate in one logical Consumer incarnation. Native registrations and
prepared content must remain independent of that authority. The notification
executor is source-free code stored in the preparation-dependent Servient crate.

Selected: retain the existing owned compiler and actual registrations, select
definitions by affordance kind/name/operation, and prove finite input/output
roles using Property Read and explicitly synchronous Action invocation. Introduce
one move-only logical Consumer authority and passive retained references shared
by separate finite/notification owners. Authority drop/revocation fences both;
closing a facet or individual operation affects only its own work. Relocate the
notification executor into the existing execution-only crate. Preserve original
call/result/queue/response/cleanup owners, paid callbacks and physical accounts.

Rejected: an SSE-first extension would exercise another Property path without
falsifying the finite assumptions. Repeating registries per family preserves the
name collision and fragmented currentness. A universal runtime/session/RPC would
erase the distinct invocation and notification lifecycles without solving a
demonstrated native need. Replacing generic compiler erasure is unnecessary.
Implementing all Actions, Event preparation or general schema/security would
obscure the smaller falsifiable composition boundary.

TypeSchema is deliberately type-only and refuses other assertions; its expressive
limits can grow incrementally. A validator of InteractionOutput is incorrectly
direction-specific: validate payloads with captured input/output roles instead.
The NoSec destination policy is a valid bounded subset, but execution also needs
the actual logical instance and selected operation in authorization. General
credential/security composition and distinct input/output media remain open.

Executable boundary: same-named Property and synchronous Action prepare and run
after source/compiler destruction; wrong input/shape/unsupported continuation
refuses; one authority fences finite late/unread results and raw notifications
including callback-time revocation; sibling facet close does not revoke it.
Test Bindings provide actual I/O without a new production protocol. The roadmap
defers HTTP SSE in favor of protocol-neutral operation/instance composition.

The selected contracts are migrated to the existing Core/Planning/Binding SPI,
Runtime Safety and module owners; WP-200/WP-400 record executable scope and
WP-600 retains the deferred observation obligations. Existing package/gate
statuses and numeric resource schemas are unchanged. Independent acceptance
belongs to the PR review, not this investigation.
