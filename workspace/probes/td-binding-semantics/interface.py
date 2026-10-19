"""Disposable semantic interface. No TD syntax or JSON-LD context API.

The Python representation is evidence, not a proposed production SPI.
"""

from dataclasses import dataclass


class Unsupported(ValueError):
    """This candidate cannot implement the supplied meaning."""


class Exhausted(ValueError):
    pass


@dataclass
class Credit:
    remaining: int = 4096

    def debit(self, amount=1):
        if amount > self.remaining:
            raise Exhausted("semantic query credit exhausted")
        self.remaining -= amount


@dataclass(frozen=True)
class Atom:
    value: object
    datatype: str | None = None
    language: str | None = None
    direction: str | None = None


@dataclass(frozen=True)
class Reference:
    iri: str


@dataclass(frozen=True)
class Ordered:
    values: tuple


@dataclass(frozen=True)
class Occurrence:
    spelling: str
    shape: str


@dataclass(frozen=True)
class Member:
    iri: str
    values: tuple
    occurrences: tuple[Occurrence, ...] = ()


@dataclass(frozen=True)
class Node:
    fields: tuple[Member, ...] = ()
    types: tuple[str, ...] = ()
    reference: str | None = None
    # Explicitly unresolved members retain opaque JSON bytes, never executable facts.
    unresolved: tuple[tuple[str, str], ...] = ()

    def members(self, credit):
        for member in self.fields:
            credit.debit()
            yield member

    def get(self, iri, credit):
        credit.debit()
        return next((m for m in self.members(credit) if m.iri == iri), None)

    def scalar(self, iri, expected_type, credit, default=None):
        member = self.get(iri, credit)
        if member is None:
            return default
        if len(member.values) != 1 or not isinstance(member.values[0], Atom):
            raise Unsupported(f"expected one literal: {iri}")
        atom = member.values[0]
        if type(atom.value) is not expected_type or atom.language or atom.direction:
            raise Unsupported(f"unsupported literal: {iri}")
        xsd = "http://www.w3.org/2001/XMLSchema#"
        if atom.datatype not in {None, xsd + {str: "string", int: "integer", bool: "boolean"}[expected_type]}:
            raise Unsupported(f"unsupported datatype: {iri}")
        return atom.value

    def require_known(self, allowed, credit, types=()):
        if self.unresolved or set(self.types) - set(types) or self.reference:
            raise Unsupported("unresolved members, node identity or unsupported types")
        for member in self.members(credit):
            if member.iri not in allowed:
                raise Unsupported(f"unknown extension: {member.iri}")


@dataclass(frozen=True)
class Security:
    name: str
    scheme: str
    parameters: tuple
    extensions: Node


@dataclass(frozen=True)
class ScopedAnnotations:
    scope: str
    types: tuple[str, ...]


@dataclass(frozen=True)
class Interaction:
    name: str
    form_index: int
    operation: str
    form_operations: tuple[str, ...]
    target: str
    media: str
    coding: str | None
    subprotocol: str | None
    security: tuple[Security, ...]  # Conjunction, not alternatives.
    security_origin: str
    scopes: tuple[str, ...]
    extensions: Node
    root_extensions: Node
    affordance_extensions: Node
    response_extensions: Node
    response_media: str
    schema: Node
    annotations: tuple[ScopedAnnotations, ...] = ()


def common_support(i, credit):
    """Example author support library; no protocol decisions."""
    i.root_extensions.require_known(set(), credit)
    i.affordance_extensions.require_known(set(), credit)
    if i.media != "application/json" or i.response_media != "application/json":
        raise Unsupported("example only implements JSON")
    if i.coding or i.subprotocol or i.scopes:
        raise Unsupported("coding, subprotocol or authorization scopes")
    if not i.security or any(s.scheme != "nosec" for s in i.security):
        raise Unsupported("example requires explicit nosec requirement")
    for security in i.security:
        security.extensions.require_known(set(), credit)
