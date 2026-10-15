# 0085 Resource Ownership and Policy Question

Status: MIGRATED

## Investigation boundary

Reassess whether the protocol-neutral engine must budget concrete binding SDK
memory, rather than treating centralized quotas, their removal or a shared
manager as a premise. Reconstruct historical rationale, actual counter/Runtime
behavior, controlled evidence and the Host/embedded failure boundary. Reconsider
ADR-0022 and PR #178's prerequisite framing without implementing a binding,
qualifying an HTTP SDK, changing production source or merging that PR.

Inputs include original design/resource history, ADR-0006/0009/0011/0015/0022,
Foundation/Binding/Planning/Runtime authorities, topics 0037/0045/0047/0071/0076/
0083, Review 07, both Property Read gate manifests and completed Consumer Runtime
evidence. PR #178's exact diff/discriminators are scoped counterexamples, not
acceptance of their proposed remedy.

## Migration

The architectural conclusion, alternatives, evidence impact, unresolved risks
and staged activation boundary belong to
[ADR-0023](../docs/ADRs/0023-resource-ownership-and-policy-boundaries.org).
The HTTP target and roadmap project its dependency/SDK-selection disposition.
Detailed accounting/API/schema migration and independent readmission remain a
separate work boundary identified by that ADR. This topic stores neither an
execution checklist nor PR/CI continuation state.
