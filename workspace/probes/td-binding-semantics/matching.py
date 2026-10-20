"""Disposable Engine matcher over the existing resolved interface.

Requirements are supplied by fixed execution-contract code BEFORE registrations
are considered. No protocol, vocabulary interpretation, or supports() callback
lives here. Compilation success promises ALL assigned native requirements;
there is no second coverage ledger. This is evidence, not a production SPI.
"""

from dataclasses import dataclass
from urllib.parse import urlsplit

from engine import Selection
from interface import Interaction, Unsupported


@dataclass(frozen=True)
class Prepared:
    interaction: Interaction
    required: frozenset[tuple[str, str]]  # (scope, meaning under the fixed contract)
    profile_paths: frozenset[str] = frozenset()


@dataclass(frozen=True)
class Binding:
    name: str
    schemes: frozenset[str]
    roles: frozenset[str]
    operations: frozenset[str]
    features: frozenset[tuple[str, str]] | None  # None: ask the pure compiler.
    compile: object
    media: frozenset[str] = frozenset({"application/json"})
    security: frozenset[str] = frozenset({"nosec"})


@dataclass(frozen=True)
class Policy:
    # These describe available shared providers/policy, never credentials.
    media: frozenset[str] = frozenset({"application/json"})
    security: frozenset[str] = frozenset({"nosec"})
    required_profiles: frozenset[str] = frozenset()
    allowed_schemes: frozenset[str] | None = None
    schemes: tuple[str, ...] = ()
    bindings: tuple[str, ...] = ()


def candidates(prepared, affordance, operation, bindings, *, role="consumer",
               policy=Policy(), strict_form=None, strict_binding=None):
    if len({b.name for b in bindings}) != len(bindings):
        raise ValueError("duplicate Binding identity")
    supported, diagnostics = [], []
    for item in prepared:
        i = item.interaction
        if i.name != affordance or i.operation != operation:
            continue
        if strict_form is not None and i.form_index != strict_form:
            continue
        scheme = urlsplit(i.target).scheme
        for b in bindings:
            if strict_binding is not None and b.name != strict_binding:
                continue
            try:
                if policy.allowed_schemes is not None and scheme not in policy.allowed_schemes:
                    raise Unsupported("application scheme constraint")
                if scheme not in b.schemes or role not in b.roles or operation not in b.operations:
                    raise Unsupported("scheme/role/operation")
                if role == "consumer" and not policy.required_profiles <= item.profile_paths:
                    raise Unsupported("required Profile has no execution path at this coordinate")
                if not {i.media, i.response_media} <= b.media & policy.media:
                    raise Unsupported("native/shared media capability")
                schemes = {s.scheme for s in i.security}
                if not schemes or not schemes <= b.security & policy.security:
                    raise Unsupported("native/shared security capability")
                if b.features is not None and not item.required <= b.features:
                    raise Unsupported("missing required semantics: " + str(sorted(item.required - b.features)))
                # The descriptor is an index, not proof of parameter applicability.
                # Only Unsupported is a skip; fatal/resource errors propagate.
                program = b.compile(item, role)
                supported.append(Selection(b.name, i, program))
            except Unsupported as error:
                diagnostics.append((i.name, i.form_index, b.name, str(error)))

    def preference(value, ordered):
        return ordered.index(value) if value in ordered else len(ordered)

    supported.sort(key=lambda s: (
        preference(urlsplit(s.interaction.target).scheme, policy.schemes),
        preference(s.binding, policy.bindings), s.interaction.form_index, s.binding))
    return supported, diagnostics


def select(prepared, affordance, operation, bindings, **constraints):
    supported, diagnostics = candidates(prepared, affordance, operation, bindings, **constraints)
    if not supported:
        raise Unsupported("no eligible candidate: " + str(diagnostics))
    return supported[0], diagnostics


def producer_assignments(prepared, bindings, handlers, *, policy=Policy()):
    """All advertised coordinates, one chosen owner each; NOT install/publication.

    Additional non-Profile Forms are allowed. Required Profile path restriction
    is a Consumer choice; scoped Profile obligations still govern each Form.
    Native route overlap and actual provider/runtime readiness remain unproved.
    """
    available_paths = frozenset().union(*(item.profile_paths for item in prepared))
    if not policy.required_profiles <= available_paths:
        raise Unsupported("required Profile has no Producer path")
    assignments = []
    for item in prepared:
        i = item.interaction
        if (i.name, i.operation) not in handlers:
            raise Unsupported("uncovered Producer handler")
        chosen, _ = select((item,), i.name, i.operation, bindings, role="producer", policy=policy)
        assignments.append(chosen)
    return tuple(assignments)
