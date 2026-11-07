# Multi-instance HTTP occupancy versus retained result ownership

Status: MIGRATED

## Reproduction and boundary

The executable workload at `0f0efc4` runs independently authorized instances over
shared exact definitions and independently targeted mappings. Its baseline suite
retains a completed result and tries an independent sibling on the same native
registration. In all nine runs the sibling refuses while the separate peer
reports zero active exchanges. The connection/Hyper driver has retired; only the
result still occupies the registration. Keeping a result indefinitely therefore
prevents unrelated logical instances from using an otherwise idle native lane.

This is the concrete private `concurrency1-until-release` policy, not an Engine
Thing limit. `Call::retire` drops the future/target but `Call::drop` alone clears
the busy flag. The native owner is retained inside `ConsumerSettlement` to protect
original result, service and account lifetimes. That requirement does not imply
retaining exclusive connection occupancy after the connection is gone.

## Alternatives and selected experiment

- Preserve the current policy and require applications to copy/drop results.
  This couples unrelated authority to result consumption and adds copies; the
  existing borrowed settlement API deliberately protects original backing.
- Provision one registration per Thing or increase native concurrency. Neither
  follows physical ownership, and both hide the demonstrated lifetime coupling.
- Release only the original exchange occupancy **after** dropping its driver,
  connection and target. Keep result/service/definition/funding owners unchanged.
  A move-only optional occupancy reference makes release exactly once, so later
  destruction of an old result cannot clear a successor's busy flag.

The last option is the bounded correction to test. It preserves one accepted
unretired native exchange per registration, does not pool connections, changes
no public API and creates no shared Engine scheduler or cache. Version the
concrete private policy; keep artifact format compatibility because its payload
interpretation is unchanged. Applicable authority remains PREP-REUSE-001,
BIND-MEM-001, SAFE-AUTH-001, RES-OWNER-001 and RES-HANDOFF-001 in their existing
registered owners. No Engine ownership contract or resource limit is relaxed.

Acceptance requires real I/O with a retained normal/error result; a sibling can
start after exchange retirement; another call still refuses while the sibling
owns its exchange; old normal/cancelled result destruction cannot release that
new occupancy; unpolled abandonment retains occupancy until physical retirement;
and all original service/result accounts release only at final disposal. Existing
read/write, security, cancellation and ownership regressions remain applicable.

The workload separately exposes expensive overprovisioned inline slots, account
storage and repeated qualification. Those are measurement conclusions, not a
justification for a universal scheduler, normalized program or new quota model.

## Migration

The selected optional occupancy lease is implemented in the concrete HTTP Call;
private policy v16 invalidates the old configuration without changing artifact
payload interpretation. Binding SPI owns the exact native occupancy policy;
runtime safety distinguishes its lease from retained Consumer result funding,
Engine call slots and Producer settlement acknowledgement. The HTTP README
reflects those separate lifetimes. Real HTTP regressions cover retained
normal/error results, local
cancellation, unpolled abandonment, successor exclusion and final original-bank
release. Existing Write/root-query tests now distinguish live native occupancy
from retained Engine slots and continue to assert input recovery, authority,
effect evidence and refusal while an actual exchange is occupied. Independent
acceptance remains required for the changed native lifetime policy; migration
does not close a package, gate or milestone.
