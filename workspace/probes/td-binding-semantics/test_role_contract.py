"""Compare role contracts, without implementing a registry or manifest system.

The old AnnotationProfile and its 23 cases remain evidence. Here the resolver's
experimental policy hook compares a fixed common rule, not an application IRI
list. This file is not a proposed production API or runtime implementation.
"""

from dataclasses import dataclass, replace
import unittest

from engine import resolve, select
import http_lowering as http
import mqtt_lowering as mqtt
from interface import ScopedAnnotations, Unsupported
from test_probe import td, form, EX, SAREF_SENSOR


class DescriptiveParentTypes:
    """Candidate shared contract: extension class assertions add no obligations.

    This defines admitted execution meaning; it does not infer that unknown
    vocabulary is harmless. Extra members and non-parent scopes stay unchanged.
    """

    def split(self, scope, node):
        if scope not in {"thing", "affordance"}:
            raise ValueError("outside this experimental contract")
        annotations = (ScopedAnnotations(scope, node.types),) if node.types else ()
        return replace(node, types=()), annotations


def prepare(source):
    return resolve(source, annotation_profile=DescriptiveParentTypes())


# A fixture-owned versioned dialect, implemented outside the common resolver.
# The selector activates the rule; the ambient class alone cannot activate it.
DIALECT = "probe-confirmed-read-v1"
CONFIRMED = "urn:probe:context:ConfirmedDevice"


@dataclass(frozen=True)
class ConfirmedProgram:
    http: http.HttpProgram
    require_confirmation: bool


def confirmed_lower(i):
    if i.subprotocol != DIALECT or i.operation != "readproperty":
        raise Unsupported("confirmed dialect not selected")
    if not any(CONFIRMED in a.types for a in i.annotations):
        raise Unsupported("missing dialect's required parent context")
    # Reuse the HTTP implementation after interpreting the selected dialect.
    # No existing Form/security/response requirement is removed.
    program = http.lower(replace(i, subprotocol=None))
    return ConfirmedProgram(program, require_confirmation=True)


class RoleContractProbe(unittest.TestCase):
    def test_shared_contract_accepts_domain_classes_without_an_inventory(self):
        for iri in (SAREF_SENSOR, "urn:completely-unknown-vocabulary:DomainClass"):
            for lower, source_form in ((http.lower, {"href": "https://device.test/p"}),
                    (mqtt.lower, {"href": "mqtt://broker.test", "mqv:filter": "sensors/p", "mqv:retain": True})):
                with self.subTest(iri=iri, lower=lower.__module__):
                    s = td([source_form])
                    expected = lower(prepare(s)[0])
                    s["@type"] = iri
                    s["properties"]["p"]["@type"] = iri
                    i = prepare(s)[0]
                    self.assertEqual(lower(i), expected)
                    self.assertEqual(i.annotations, (ScopedAnnotations("thing", (iri,)),
                                                     ScopedAnnotations("affordance", (iri,))))
                    self.assertIn(iri, i.schema.types)
                    self.assertIn("https://www.w3.org/2019/wot/json-schema#NumberSchema", i.schema.types)

    def test_common_rule_does_not_exempt_unknown_instructions(self):
        for scope in ("thing", "affordance", "form"):
            s = td()
            s["@type"] = SAREF_SENSOR
            target = {"thing": s, "affordance": s["properties"]["p"], "form": form(s)}[scope]
            target[EX + "requireConfirmation"] = "before-success"
            with self.assertRaisesRegex(Unsupported, "unknown extension"):
                http.lower(prepare(s)[0])
        s = td()
        form(s)["@type"] = SAREF_SENSOR
        with self.assertRaises(Unsupported):
            http.lower(prepare(s)[0])

    def test_explicit_dialect_requires_parent_context_before_selection(self):
        bindings = (("http", http.lower), ("confirmed", confirmed_lower))
        for scope in ("thing", "affordance"):
            s = td()
            target = s if scope == "thing" else s["properties"]["p"]
            target["@type"] = CONFIRMED
            form(s)["subprotocol"] = DIALECT
            selected, diagnostics = select(prepare(s), "p", "readproperty", bindings)
            self.assertEqual(selected.binding, "confirmed")
            self.assertTrue(selected.program.require_confirmation)
            self.assertEqual(selected.program.http.method, "GET")
            self.assertTrue(diagnostics)  # Ordinary HTTP refuses the dialect.
            with self.assertRaises(Unsupported):
                select(prepare(s), "p", "readproperty", bindings, strict_binding="http")
            del target["@type"]
            with self.assertRaises(Unsupported):
                select(prepare(s), "p", "readproperty", bindings)
            target["@type"] = CONFIRMED
            form(s)[EX + "unknownRequiredStep"] = True
            with self.assertRaises(Unsupported):
                select(prepare(s), "p", "readproperty", bindings)
        s = td()
        form(s)["subprotocol"] = "unknown-dialect-v1"
        with self.assertRaises(Unsupported):
            select(prepare(s), "p", "readproperty", bindings)

    def test_hidden_class_obligation_is_an_exclusion_not_automatic_discovery(self):
        s = td()
        s["@type"] = CONFIRMED
        selected, _ = select(prepare(s), "p", "readproperty",
                             (("http", http.lower), ("confirmed", confirmed_lower)))
        # Intentional limitation: a legacy vocabulary might demand confirmation
        # solely from this class. The descriptive contract cannot discover that.
        self.assertEqual(selected.binding, "http")
        self.assertIsInstance(selected.program, http.HttpProgram)
        # Descriptive classes cannot weaken an explicit WoT security requirement.
        s["securityDefinitions"]["auth"] = {"scheme": "basic"}
        s["security"] = ["auth"]
        with self.assertRaisesRegex(Unsupported, "nosec"):
            http.lower(prepare(s)[0])


if __name__ == "__main__":
    unittest.main(verbosity=2)
