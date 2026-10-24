# Architecture backbone — v6.0

The backbone is navigation and cross-domain structure. Detailed requirements have
one owner in [the authority map](../spec/authority.toml).

| Owner | Responsibility |
| --- | --- |
| [Goals/context](00-system-goals-and-context.md) | Enduring product outcomes and deployment scope |
| [Flows](10-primary-data-flows.md) | Semantic and physical data flows, Discovery client boundary |
| [Modules](20-module-boundaries.md) | Dependency and capability ownership |
| [Preparation](30-compiled-plan-lifecycle.md) | Description, reusable definitions, instances and preparation lifetime |
| [Bindings](40-protocol-binding-spi-and-deployment.md) | Native execution, composition and infrastructure |
| [Servient](50-servient-runtime-lifecycle.md) | Publication, dispatch, logical lifetime and retirement |

The retained filenames do not mandate compiled plan sets or route-scoped native
progress. Those former universal mechanisms are superseded by ADR-0025.
[Domain specifications](../spec/README.md) refine these boundaries. Availability
is established by source/tests, not by this target description. Current Property
Read representations are restricted implementations, not the general SPI.
