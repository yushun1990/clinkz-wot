"""Disposable Engine-side resolver and selection experiment.

PyLD supplies JSON-LD processing. Its private hook records source occurrences
before expansion erases nulls/empty arrays. This is instrumentation, not a
candidate Rust implementation or a claim of bounded JSON-LD processing.
"""

from copy import deepcopy
from dataclasses import dataclass, replace
import json
from pathlib import Path
from urllib.parse import urljoin, urlsplit

from pyld import jsonld

from interface import (Atom, Interaction, Member, Node, Occurrence, Ordered,
                       Reference, ScopedAnnotations, Security, Unsupported)

TD_CONTEXT = "https://www.w3.org/2022/wot/td/v1.1"
TD = "https://www.w3.org/2019/wot/td#"
JS = "https://www.w3.org/2019/wot/json-schema#"
HCTL = "https://www.w3.org/2019/wot/hypermedia#"
MARK = "urn:clinkz:disposable-probe:source"
ROOT_KEYS = {"title", "id", "properties", "security", "securityDefinitions", "base"}
PROPERTY_KEYS = {"forms", "type", "readOnly", "writeOnly"}
FORM_KEYS = {"href", "op", "contentType", "contentCoding", "subprotocol", "security", "scopes", "response"}


class ResolutionError(ValueError):
    pass


@dataclass(frozen=True)
class AnnotationProfile:
    """Trusted application declarations: exact scope/type pairs, never inferred.

    This experiment supports domain types at Thing/affordance scope only. It
    cannot mark Form, response or security instructions inert, or allow an
    entire namespace. Values remain available in Interaction.annotations.
    """
    type_terms: tuple[tuple[str, str], ...] = ()

    def __post_init__(self):
        if any(scope not in {"thing", "affordance"} for scope, _ in self.type_terms):
            raise ResolutionError("annotation profile only classifies Thing/affordance types")

    def split(self, scope, node):
        metadata = tuple(t for t in node.types if (scope, t) in self.type_terms)
        remaining = tuple(t for t in node.types if (scope, t) not in self.type_terms)
        annotations = (ScopedAnnotations(scope, metadata),) if metadata else ()
        return replace(node, types=remaining), annotations


def array(value):
    return value if isinstance(value, list) else [value]


def shape(value):
    if value is None:
        return "null"
    if isinstance(value, list):
        return "array:" + str(len(value))
    return type(value).__name__


class Trace(jsonld.JsonLdProcessor):
    def __init__(self):
        super().__init__()
        self.nodes = {}
        self.origins = {}
        self.type_origins = {}

    def _expand_object(self, active_ctx, active_property, expanded_active_property,
                       element, expanded_parent, options, inside_list=False,
                       type_key=None, type_scoped_ctx=None):
        facts, unresolved, type_origins = {}, [], []
        for key, value in element.items():
            if key in {"@context", MARK}:
                continue
            iri = self._expand_iri(active_ctx, key, vocab=True)
            if iri == MARK:
                raise ResolutionError("source collides with probe instrumentation")
            if iri is None or (not iri.startswith("@") and ":" not in iri):
                unresolved.append((key, json.dumps(value, sort_keys=True)))
            else:
                facts.setdefault(iri, []).append(Occurrence(key, shape(value)))
                if iri == "@type":
                    type_origins.extend((key, self._expand_iri(type_scoped_ctx, term, vocab=True,
                                                             base=options.get("base", "")))
                                        for term in array(value))
        self.origins[id(expanded_parent)] = (facts, tuple(unresolved))
        self.type_origins[id(expanded_parent)] = type_origins
        result = super()._expand_object(active_ctx, active_property, expanded_active_property,
                                        element, expanded_parent, options,
                                        inside_list, type_key, type_scoped_ctx)
        if MARK in element:
            marker = element[MARK]
            self.nodes[marker] = (expanded_parent, facts, tuple(unresolved))
        return result

    def value(self, value):
        if "@value" in value:
            payload = value["@value"]
            if value.get("@type") == "@json":
                # JSON literal keys/order/arrays are opaque payload, not semantic terms.
                payload = json.dumps(payload, sort_keys=True, separators=(",", ":"))
            return Atom(payload, value.get("@type"), value.get("@language"), value.get("@direction"))
        if "@list" in value:
            return Ordered(tuple(self.value(v) for v in value["@list"]))
        if set(value) == {"@id"}:
            return Reference(value["@id"])
        facts, unresolved = self.origins.get(id(value), ({}, ()))
        return self.node(value, facts, unresolved)

    def node(self, expanded, facts, unresolved, exclude=()):
        unsupported = set(expanded) & {"@graph", "@reverse", "@included", "@index"}
        if unsupported:
            raise ResolutionError("unsupported semantic feature: " + str(sorted(unsupported)))
        fields = []
        keys = (set(expanded) | set(facts)) - set(exclude) - {MARK, "@id", "@type"}
        for iri in sorted(keys):
            if iri.startswith("@"):
                raise ResolutionError("unsupported semantic keyword: " + iri)
            fields.append(Member(iri, tuple(self.value(v) for v in expanded.get(iri, [])),
                                 tuple(facts.get(iri, ()))))
        return Node(tuple(fields), () if "@type" in exclude else tuple(expanded.get("@type", [])),
                    None if "@id" in exclude else expanded.get("@id"), unresolved)

    def scope(self, marker, core_keys):
        expanded, facts, unresolved = self.nodes[marker]
        exclude = {iri for iri, occurrences in facts.items()
                   if any(o.spelling in core_keys for o in occurrences)}
        # Non-type core/extension collisions cannot be silently separated.
        # Class values have per-spelling origins from the expansion hook.
        for iri in exclude:
            if iri != "@type" and any(o.spelling not in core_keys for o in facts[iri]):
                raise ResolutionError("alias collides with a shared TD field")
        # The official TD context inserts these index-map labels. They are
        # already carried as typed source names, not extension instructions.
        exclude |= {"name", "hasInstanceConfiguration"}
        node = self.node(expanded, facts, unresolved, exclude)
        if "@type" in exclude:
            # A DataSchema's lexical `type` and semantic `@type` both expand
            # here. Preserve extension classes without reclassifying the schema.
            node = replace(node, types=tuple(t for spelling, t in self.type_origins[id(expanded)]
                                            if spelling not in core_keys and t in expanded.get("@type", ())))
        return node


def resolve(source, contexts=None, max_bytes=32768, *, annotation_profile=AnnotationProfile()):
    """Narrow property TD adapter; rejects unimplemented shared fields.

    No network loader. Input/output caps do NOT bound PyLD's peak memory/time.
    """
    if len(json.dumps(source).encode()) > max_bytes:
        raise ResolutionError("input byte cap")
    source = deepcopy(source)
    def check_marker(value):
        if isinstance(value, dict):
            if MARK in value:
                raise ResolutionError("source collides with probe instrumentation")
            for child in value.values():
                check_marker(child)
        elif isinstance(value, list):
            for child in value:
                check_marker(child)
    check_marker(source)
    contexts = dict(contexts or {})
    contexts[TD_CONTEXT] = json.loads(Path(__file__).with_name("td-context.jsonld").read_text())

    def loader(url, options=None):
        if url not in contexts:
            raise ResolutionError("context not supplied: " + url)
        return {"contextUrl": None, "documentUrl": url, "document": deepcopy(contexts[url])}

    original = deepcopy(source)
    source[MARK] = "root"
    for name, prop in source["properties"].items():
        prop[MARK] = "affordance:" + name
        for index, form in enumerate(prop["forms"]):
            form[MARK] = f"form:{name}:{index}"
            if "response" in form:
                form["response"][MARK] = f"response:{name}:{index}"
    for name, security in source["securityDefinitions"].items():
        security[MARK] = "security:" + name
    trace = Trace()
    trace.expand(source, {"documentLoader": loader, "processingMode": "json-ld-1.1"})
    # This probe supports the standard lexical TD core, not core aliases or
    # remapping. Check critical fields against their context-resolved identity.
    expected = {"href": HCTL + "hasTarget", "op": HCTL + "hasOperationType",
                "contentType": HCTL + "forContentType", "forms": TD + "hasForm",
                "readOnly": JS + "readOnly", "writeOnly": JS + "writeOnly",
                "security": TD + "hasSecurityConfiguration"}
    for _, facts, _ in trace.nodes.values():
        for iri, occurrences in facts.items():
            for occurrence in occurrences:
                if occurrence.spelling in expected and iri != expected[occurrence.spelling]:
                    raise ResolutionError("remapped shared TD field: " + occurrence.spelling)
    root = trace.scope("root", ROOT_KEYS)
    # Root @id is source identity, not an extension or execution authority.
    root = Node(root.fields, root.types, None, root.unresolved)
    root, root_annotations = annotation_profile.split("thing", root)
    interactions = []
    for name, prop in original["properties"].items():
        aff = trace.scope("affordance:" + name, PROPERTY_KEYS)
        aff, aff_annotations = annotation_profile.split("affordance", aff)
        expanded_prop, facts, unresolved = trace.nodes["affordance:" + name]
        schema = trace.node(expanded_prop, facts, unresolved, {TD + "hasForm", "name"})
        for index, form in enumerate(prop["forms"]):
            default_ops = ["writeproperty"] if prop.get("writeOnly", False) else (
                ["readproperty"] if prop.get("readOnly", False) else ["readproperty", "writeproperty"])
            operations = tuple(array(form["op"])) if "op" in form else tuple(default_ops)
            if not operations or set(operations) - {"readproperty", "writeproperty"}:
                raise ResolutionError("property operation outside probe scope")
            if prop.get("readOnly") and prop.get("writeOnly"):
                raise ResolutionError("contradictory property access")
            if "{" in form["href"]:
                raise ResolutionError("URI templates outside probe scope")
            target = urljoin(original.get("base", ""), form["href"])
            if not urlsplit(target).scheme:
                raise ResolutionError("absolute endpoint required")
            requirements = []
            for sec in array(form.get("security", original["security"])):
                definition = original["securityDefinitions"][sec]
                params = {k: v for k, v in definition.items() if k in {"in", "name"}}
                if definition["scheme"] == "basic":
                    params.setdefault("in", "header")
                requirements.append(Security(sec, definition["scheme"], tuple(sorted(params.items())),
                    trace.scope("security:" + sec, {"scheme", "in", "name"})))
            response = trace.scope(f"response:{name}:{index}", {"contentType"}) if "response" in form else Node()
            for operation in operations:
                interactions.append(Interaction(
                    name, index, operation, operations, target,
                    form.get("contentType", "application/json"), form.get("contentCoding"),
                    form.get("subprotocol"), tuple(requirements),
                    "form" if "security" in form else "thing", tuple(array(form.get("scopes", []))),
                    trace.scope(f"form:{name}:{index}", FORM_KEYS), root, aff, response,
                    form.get("response", {}).get("contentType", form.get("contentType", "application/json")),
                    schema, root_annotations + aff_annotations))
    return tuple(interactions)


@dataclass(frozen=True)
class Selection:
    binding: str
    interaction: Interaction
    program: object


def candidates(interactions, affordance, operation, bindings, *, strict_form=None, strict_binding=None):
    """Engine coordinates; Bindings decide native applicability, without effects."""
    supported, diagnostics = [], []
    for i in interactions:
        if i.name != affordance or i.operation != operation:
            continue
        if strict_form is not None and i.form_index != strict_form:
            continue
        for name, lower in bindings:
            if strict_binding is not None and name != strict_binding:
                continue
            try:
                supported.append(Selection(name, i, lower(i)))
            except Unsupported as error:
                diagnostics.append((i.name, i.form_index, name, str(error)))
    return supported, diagnostics


def select(interactions, affordance, operation, bindings, *, strict_form=None, strict_binding=None):
    supported, diagnostics = candidates(interactions, affordance, operation, bindings,
                                        strict_form=strict_form, strict_binding=strict_binding)
    if not supported:
        raise Unsupported("no supported candidate for requested constraints: " + str(diagnostics))
    return supported[0], diagnostics


def producer_coverage(interactions, bindings):
    """Every advertised coordinate needs exactly one owner in this experiment."""
    for i in interactions:
        supported, _ = candidates((i,), i.name, i.operation, bindings)
        if len(supported) != 1:
            raise Unsupported("uncovered or ambiguous Producer coordinate")
