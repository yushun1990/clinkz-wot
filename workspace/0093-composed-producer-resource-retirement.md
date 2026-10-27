# Composed Producer resource and retirement ownership

Status: MIGRATED

## Finding and alternatives

At the #238 merge (`ec648cd`), `retire_composed` checks only the supplied
participants. It cannot detect a missing original participant, a replacement
object, or an accepted Action still retained at the logical owner. The composed
handoff bounds one definition set but reserves no shared Host capacity.

Keeping caller-supplied slices alone cannot enforce complete retirement. A
universal runtime/session abstraction would incorrectly merge native progress,
finite exchanges and continuing Actions. Extending the single-Property facade
would retain the wrong compiler/operation cardinality.

## Selected implementation boundary

Keep the source-free logical owner and concrete Binding-owned progress. Give
each participation a move-only opaque identity; stage bounded original identity
storage before native effects, and require the identical complete set at
publication and retirement. Check retained Actions as well as native retirement.

An optional owned composition exposure takes pure, caller-provisioned prepared
definitions, behavior and unstarted native participation owners. It reserves
their controlled backing, result/cleanup allowances and final-qualification
scratch from the actual shared Host bank before accepting that handoff. Its
fixed member storage permits typed native progress but no member replacement.
The original logical instance shares the same reservation owner, so accepted
work and unread results retain the original charge through replacement. The
publication guard retains no native shutdown capability. The owned exposure's
Drop revokes permission and disposes its actual native leases using their
existing finite local-abort contracts; it reports no successful drain.

Native SDK/OS/shared infrastructure and the externally provisioned raw logical
root/Action-store baseline keep their declared owners. No generic native-RAM,
hardware, schema/security, production Action or package/milestone qualification
is inferred. Pure caller construction precedes the funded ownership handoff;
unsupported or already-started managed inputs are returned intact on refusal.

## Falsifiable completion boundary

Real HTTP and the existing linked test-only UDP participant must exercise
complete-set retirement, shared native sibling isolation, partial setup failure,
replacement/late results, saturation, lost exposure/root cleanup, original-bank
retention and final socket release. Qualification scratch must end independently
of retained lifetime work. Independent review remains required for acceptance.

Detailed ownership and implemented scope are projected into the existing
[Binding contract](../docs/spec/binding-spi.md),
[Runtime contract](../docs/spec/runtime-safety.md),
[WP-200](../docs/work-packages/WP-200-planning.md),
[WP-400](../docs/work-packages/WP-400-servient.md#owned-composed-host-producer),
API ownership projection and the `composed_thing` real-I/O regressions. This
migration records the implementation boundary, not independent acceptance or
package/milestone completion.
