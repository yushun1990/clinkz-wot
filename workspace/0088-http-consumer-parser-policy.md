# 0088 HTTP Consumer parser policy and SDK maintenance

Status: MIGRATED

Investigation question: does the independently demonstrated finite stock-Hyper
envelope satisfy the actual safety requirement without owning a private HTTP
SDK solely to preserve local trailer/extension numbers?

[PR #197 review 5562587383](https://github.com/yushun1990/clinkz-wot/pull/197#pullrequestreview-5562587383)
supplies the independent challenge and alternatives. The investigation separates
Engine/SPI backing, identity, cancellation and ownership invariants from concrete
parser policy under ADR-0023. A private fork can implement the old policy; that
fact alone does not justify selecting it. Deferring chunked support preserves a
safe partial boundary but cannot qualify ordinary HTTP/1.1 framing.

The selected replacement, rejected alternatives, exact constraints, identity and
impact obligations are projected into the existing
[WP-600 admission amendment](../docs/work-packages/WP-600-http-property-read-target.md#http-parser-policy-amendment).
The [standalone upstream probes](../docs/evidence/WP-600-stock-hyper-policy-probe.md)
and [tranche evidence](../docs/evidence/WP-600-http-consumer-property-read.toml)
own reproducibility and observations. The work-package index owns admission.

Migration records the proposal in its proper owners; it grants no independent
acceptance, changed-source implementation admission or production qualification.
PR #197 remains historical technical evidence. No separate implementation
tranche, gate, state file or global resource-policy change is proposed.
