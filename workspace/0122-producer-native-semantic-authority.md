# Producer native and semantic authority (Review 10 F1)

Status: MIGRATED. Normative contracts are in [Binding SPI](../docs/spec/binding-spi.md#producer-action-candidates-and-native-participation),
module boundaries, Interaction Core, Runtime Safety and the affected work packages.

Review 10 identifies executable authority in HTTP Action native progress and
MQTT participation failure handling. Reconstructed source at `a84938c` confirms
that both runtime crates depend normally on Servient, including without prepare.
The existing finite Producer already supplies a narrower counterexample:
native candidates and original reply opportunities do not require a logical owner.

The selected correction uses an Action-specific Core candidate/borrowed-reply
contract. Servient's existing Action store retains non-forgeable handles,
continuations, authorization, original instance/result funding, terminal retention
and settlement. Native records retain only copyable invocation references for
correlation; Servient associates those records with the exact native participation
before encoding acceptance. Described controls/root inventories may use other
qualified native participants while retaining original route/principal checks;
local handle recovery and acknowledgement stay with the invoking participation.
HTTP retains matching, token epochs, listener/socket
ownership, native status inventory/order and reply encoding. The passive closed
participation contract belongs in Core and receives an issuer identity, never a
Servient or an application-dispatch capability. MQTT supplies native failure
facts; Servient reconciles them into logical revocation. Logical notification
publication metadata is funded by Servient separately from native publication
storage.

Alternatives rejected: moving only the existing step behind a dispatch trait
preserves the authority leak; moving logical handles into Core solely so the
Binding can keep progressing them preserves wrong ownership; a generic session,
new per-Thing mega-runtime or common scheduler is unnecessary for this concrete
path. A separate Action runtime table would duplicate the existing invocation
store. Extending that original store with participation association is smaller.

This corrects an implementation conflict with accepted v6 authority, rather than
selecting another architecture or superseding ADR-0025. Reusable definitions keep
immutable mapping/schema data only; live issuer, invocation association and native
resources remain outside them, preserving topic 0081's required boundary. No
cross-instance compiler reuse is claimed. Review 10 F2/F3, continuing Consumer
capacity, all-profile execution and production protocol qualification remain
separate limitations. MQTT changes are limited to the shared authority seam and
its existing execution callers, not its wire protocol.

Falsifiable boundary: real HTTP invoke/query/cancel, original controls after the
initiating exchange, rejection, revocation, late results and independent native/
logical/result disposal; actual MQTT failure evidence before and after logical
reconciliation; neither Binding has a normal dependency on Servient. Independent
acceptance reconstructs the final diff and executable evidence at the PR head.
