# Data meaning across interaction lifetimes

Status: MIGRATED

The product objective is protocol-independent Property, Action, Event and
described collection execution. TD 1.1 assigns the same DataSchema vocabulary
to their distinct data roles; synchronicity does not select a smaller schema
language. Source inspection after PR #268 found `prepare_type_schema` hardcoded
in continuing Action selection and HTTP lowering, despite the shared owned
PreparedSchema and captured JSON validator already supporting structured values.
Those restrictions are incomplete preparation, not inherent WoT or HTTP limits.

Common responsibilities are TD interpretation, source-independent immutable
constraint meaning, complete actual service qualification, bounded validation
before effects/claimed delivery, fresh authority and original resource ownership.
Property access eligibility, Action input/output, Event's four data roles and
collection membership/selection/occurrence identity remain operation semantics.
Acceptance is not Action completion; local cancellation is not remote rollback;
notification stop and Producer accepted work have independent owners. Sharing
data meaning does not unify these lifecycles or grant execution permission.

Selected increment: migrate continuing Action input/output in both HTTP roles to
the existing structured contract. Reuse a borrowed schema census between finite
and continuing preparation, fund variable backing/destruction before building,
and construct Consumer meaning once across actual invoke/control mappings.
HTTP native qualification does not reconstruct Consumer constraints. Producer
definition clones and captured validators share immutable schema backing;
selected final qualification compares borrowed assertions without rebuilding it.
Complete final-TD coverage and actual native/control/codec support remain required.

Alternatives: simply replacing type-only calls leaves formerly zero-cost backing
and deep clones unfunded, and repeats semantic compilation inside native probes.
A universal prepared interaction/compiler/state machine conflates data meaning
with independent lifetimes. A second Action schema evaluator duplicates Core's
protocol-neutral semantics. None is needed: the existing borrowed validation
contract supports these roles; correct its preparation and ownership use.

Concrete HTTP endpoint, POST/GET/DELETE, id-template, status/Location, NoSec and
JSON limits remain participant qualifications. MQTT's declared type-only profile,
QoS/native attribution and unsupported control/response combinations remain its
qualifications. The generic notification scalar preparation and Action/Event
status/collection preparation still have narrower schema paths: these are
incomplete Engine migrations, not lifetime restrictions or permanent protocol
limits. This increment does not advertise structured MQTT, collection status,
new security/media, constrained runtime or cross-instance Action definition reuse.

Affected authority: EXEC-DATA-001 and EXEC-ACTION-001 (Core), PREP-COVER-001,
PREP-ISOLATION-001 and PREP-REUSE-001 (Planning), SPI captured actual services,
SAFE-AUTH-001/SAFE-CANCEL-001/SAFE-CLEANUP-001, and original resource/work ownership.
Existing scalar Action and downstream I/O evidence remains applicable to its
scope. New execution evidence must cover nested required input/output, exact
integer/string constraints, refusal by unsupported actual services/assertions,
short work/capacity, source retirement, cancellation/replacement, late validation
and final release. No milestone or independent acceptance claim follows.

Migration: Core's alloc-only `data_schema::ValueSchemas` replaces the
Consumer-specific input/output carrier; Event roles and collection carriers stay
distinct. Core, Planning, SPI and runtime-safety contracts, API ownership,
WP-200/400/600 and the structured-data evidence now record the selected scope.
Existing scalar Action/downstream and finite execution retain their original
evidence obligations. The new HTTP tests exercise the stated data, qualification,
late-result and original-funding boundaries.
