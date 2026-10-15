# HTTP Consumer local disposal feasibility

Status: MIGRATED

## Question and evidence boundary

Preparing ADR-0022's first Host HTTP Consumer candidate exposed a concrete
dependency question: can one low-level Hyper connection be owned by the call
and destroyed on the existing Runtime's allocation-free last-root path, at
every suspended request/response phase?

Reconstruction used master `47f209575b830b3a86046f712a3deddfc80edf0e`, the
accepted Consumer/Producer authorities and exact predecessor evidence, actual
TD lending/compiler capture and Host Runtime Drop implementations, PR #177's
independent review constraints, and the checksum-pinned Hyper client source.

The [public-SDK discriminator](../tools/architecture-fixtures/http-consumer-admission/README.md)
demonstrates the unmodified dependency failure and a small repair in temporary
source. It also distinguishes direct unspawned TCP progress from the unproved
public engine integration. No production source or registry cache is patched.

## Migrated disposition

The exact scope, alternatives, accepted-authority impact and precheck/completion
split are owned by the registered
[WP-600 HTTP local-disposal admission candidate](../docs/work-packages/WP-600-http-local-disposal-admission.md).
It selects dependency repair before adapter admission, preserving the existing
disposal guarantee. Migration records the bounded investigation's disposition;
independent admission and production completion remain separate.

The first HTTP target remains one complete Consumer registration/generation/call,
two selected properties and repeated reads. Its paid metadata/support and
physical input/work/accounting obligations remain with that target. This topic
does not select a generic compiler certification framework, worker, runtime split,
process isolation, new gate or broader HTTP/embedded capability.
