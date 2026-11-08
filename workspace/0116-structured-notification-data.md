# Structured Notification data meaning

Status: MIGRATED

At merged master 1d5d414, TD exposes all Notification SchemaViews and Core already
evaluates structured constraints independently of operation lifetime. Notification
preparation still requests primitive assertions. This is incomplete integration,
not evidence for a second evaluator or a universal interaction runtime.

Common responsibilities are schema interpretation, source-independent constraint
ownership, exact actual codec qualification, prepaid bounded validation and fresh
authority before effects or delivery. Property observation uses its readable
value role without an implicit Read or initial sample. Events preserve independent
subscription, data, dataResponse and cancellation roles, including absence versus
an unconstrained value. Property collections retain eligible named members and
their map representation. Event collections retain occurrence identity and all
roles per Event; they imply no universal aggregate control encoding or reply right.

Selected correction: one shared Planning role selector and census, borrowing the
existing Core evaluator through actual codecs. Native Consumer compilation borrows
the prepared roles. Runtime validation retains the original funded definition
rather than copying variable schema trees. MQTT Producer definitions and captured
services share immutable `notification::NotificationSchemas` role backing;
the old Consumer-specific carrier and private MQTT schema projection are removed; selected final qualification compares the
borrowed final TD without reconstructing trees. Aggregate Event schema storage,
rollback/disposal work and JSON callback scratch/work require explicit bounds.

The real vertical capability is structured leaf Property/Event JSON publication
and subscription through the existing MQTT 3.1.1/Mosquitto path. This single native
value path establishes Producer and Consumer validation without expanding wire
protocols. MQTT collection mappings remain explicitly type-only; shared structured
meaning does not qualify those mappings. Subscription/cancellation data and any
Event dataResponse still refuse MQTT preparation. Native ACKs cannot establish
application response semantics. Security remains the independently qualified
loopback NoSec subset with fresh captured destination/application policy.

Rejected alternatives: changing type-only calls alone preserves unfunded deep
clones and obsolete scratch/work bounds; codec-local constraint interpretation
duplicates Core; a finite request wrapper loses continuing ownership and response
opportunities; widening every collection/control protocol is unnecessary to prove
the boundary. Existing notification owners already preserve independent cursors,
native sharing, loss, cancellation and cleanup and should remain in place.

Affected authority: WOT-SEM-001; EXEC-DATA-001/EXEC-NOTIFY-001; PREP-COVER-001,
PREP-ISOLATION-001/PREP-REUSE-001; actual SPI service qualification; original
resource/work ownership and SAFE-AUTH-001/SAFE-CANCEL-001/SAFE-CLEANUP-001.
Existing scalar/native lifetime evidence retains its scope. New evidence must
exercise structured producer-to-consumer data, an independent invalid wire peer,
all four external Event roles, complete final advertisement, short work/capacity,
source retirement, original retained data and final resource release. No gate or
milestone acceptance is implied.

Migration: the Core, TD, Planning, SPI and runtime-safety owners, API ownership
projection, WP-200/300/400/600, concrete MQTT mapping and structured-data evidence
now describe this boundary. Real broker and public downstream fixtures implement
the scoped evidence; existing logical/native lifecycle regressions remain binding.
