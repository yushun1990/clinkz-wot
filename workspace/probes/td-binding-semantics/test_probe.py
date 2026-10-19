"""Falsification examples, all using the same public semantic interface."""

import unittest

from pyld import jsonld

from engine import (TD_CONTEXT, AnnotationProfile, ResolutionError, candidates, producer_coverage,
                    resolve, select)
import http_lowering as http
import mqtt_lowering as mqtt
from interface import Atom, Credit, Exhausted, Node, Ordered, Reference, ScopedAnnotations, Unsupported

EX = "https://example.test/vocab#"
SAREF_SENSOR = "https://saref.etsi.org/core/TemperatureSensor"
BINDINGS = (("http", http.lower), ("mqtt", mqtt.lower))


def td(forms=None, context=None):
    return {
        "@context": [TD_CONTEXT, context or {"mqv": mqtt.MQTT}],
        "id": "urn:probe:source",
        "title": "semantic boundary probe",
        "security": ["none"],
        "securityDefinitions": {"none": {"scheme": "nosec"}},
        "properties": {"p": {"type": "number", "readOnly": True, "forms": forms or [
            {"href": "https://device.test/p"}]}}
    }


def form(source):
    return source["properties"]["p"]["forms"][0]


class Probe(unittest.TestCase):
    def test_http_prefix_alias_full_iri_and_pinned_remote_equivalence(self):
        variants = [({"h": http.HTTP}, "h:methodName"),
                    ({"verb": http.HTTP + "methodName"}, "verb"),
                    ({}, http.HTTP + "methodName")]
        programs = []
        for context, term in variants:
            s = td(context=context)
            form(s)[term] = "POST"
            programs.append(http.lower(resolve(s)[0]))
        remote = td()
        remote["@context"].append("https://contexts.test/http")
        form(remote)["verb"] = "POST"
        programs.append(http.lower(resolve(remote, {"https://contexts.test/http": {
            "@context": {"verb": http.HTTP + "methodName"}}})[0]))
        self.assertTrue(all(p == programs[0] for p in programs))
        self.assertEqual(programs[0].method, "POST")
        self.assertNotEqual(programs[0], http.lower(resolve(td())[0]))

    def test_http_defaults_and_multi_operation_source_cardinality(self):
        s = td()
        del s["properties"]["p"]["readOnly"]
        views = resolve(s)
        self.assertEqual([http.lower(i).method for i in views], ["GET", "PUT"])
        form(s)["htv:methodName"] = "POST"
        for i in resolve(s):
            with self.assertRaisesRegex(Unsupported, "multiple"):
                http.lower(i)

    def test_write_only_default_does_not_invent_read(self):
        s = td()
        del s["properties"]["p"]["readOnly"]
        s["properties"]["p"]["writeOnly"] = True
        self.assertEqual([i.operation for i in resolve(s)], ["writeproperty"])
        self.assertEqual(http.lower(resolve(s)[0]).method, "PUT")

    def test_mqtt_prefix_alias_equivalence_and_protocol_defaults(self):
        outputs = []
        for prefix in ["mqv", "bus"]:
            s = td([{"href": "mqtt://broker.test:1883", prefix + ":filter": "sensors/p",
                     prefix + ":retain": True, prefix + ":qos": 1}], {prefix: mqtt.MQTT})
            outputs.append(mqtt.lower(resolve(s)[0]))
        self.assertEqual(outputs[0], outputs[1])
        self.assertEqual(outputs[0].packet, "subscribe")
        self.assertEqual(outputs[0].completion, "first-retained-message")
        form(s)[prefix + ":filter"] = "sensors/other"
        self.assertNotEqual(outputs[0], mqtt.lower(resolve(s)[0]))
        s = td([{"href": "mqtt://broker.test:1883", "mqv:topic": "sensors/p"}])
        s["properties"]["p"] = {"writeOnly": True, "forms": s["properties"]["p"]["forms"]}
        self.assertEqual(mqtt.lower(resolve(s)[0]).packet, "publish")

    def test_mqtt_nonretained_read_and_wildcard_are_explicit_limits(self):
        s = td([{"href": "mqtt://broker.test", "mqv:filter": "sensors/p"}])
        with self.assertRaisesRegex(Unsupported, "retained"):
            mqtt.lower(resolve(s)[0])
        form(s)["mqv:retain"] = True
        form(s)["mqv:filter"] = "sensors/#"
        with self.assertRaisesRegex(Unsupported, "exact topic"):
            mqtt.lower(resolve(s)[0])

    def test_property_and_type_scoped_contexts(self):
        s = td(context={"packet": {"@id": EX + "packet", "@context": {
            "verb": http.HTTP + "methodName"}}})
        form(s)["packet"] = {"verb": "POST"}
        n = resolve(s)[0].extensions.get(EX + "packet", Credit()).values[0]
        self.assertIsInstance(n, Node)
        self.assertEqual(n.scalar(http.HTTP + "methodName", str, Credit()), "POST")
        # A nested packet is preserved, but this HTTP author does not implement it.
        with self.assertRaisesRegex(Unsupported, "unknown extension"):
            http.lower(resolve(s)[0])
        s = td(context={"Request": {"@id": http.HTTP + "Request", "@context": {
            "verb": http.HTTP + "methodName"}}})
        form(s).update({"@type": "Request", "verb": "POST"})
        self.assertEqual(http.lower(resolve(s)[0]).method, "POST")

    def test_scoped_value_iris_lists_language_and_json_literals(self):
        s = td(context={"targetRef": {"@id": EX + "ref", "@type": "@id"},
                        "steps": {"@id": EX + "steps", "@container": "@list"},
                        "payload": {"@id": EX + "payload", "@type": "@json"},
                        "label": EX + "label"})
        form(s).update({"targetRef": "https://device.test/ref", "steps": ["b", "a"],
                        "payload": {"unmappedKey": [None, 1]},
                        "label": {"@value": "hello", "@language": "en"}})
        ext = resolve(s)[0].extensions
        self.assertEqual(ext.get(EX + "ref", Credit()).values, (Reference("https://device.test/ref"),))
        self.assertEqual(ext.get(EX + "steps", Credit()).values, (Ordered((Atom("b"), Atom("a"))),))
        self.assertEqual(ext.get(EX + "label", Credit()).values, (Atom("hello", language="en"),))
        self.assertEqual(ext.get(EX + "payload", Credit()).values,
                         (Atom('{"unmappedKey":[null,1]}', datatype="@json"),))

    def test_absent_null_empty_array_and_singleton_shapes_survive(self):
        observations = []
        for value in ["ABSENT", None, [], "GET", ["GET"]]:
            s = td()
            if value != "ABSENT":
                form(s)["htv:methodName"] = value
            observations.append(resolve(s)[0].extensions.get(http.HTTP + "methodName", Credit()))
        self.assertIsNone(observations[0])
        self.assertEqual(observations[1].values, observations[2].values)
        self.assertNotEqual(observations[1].occurrences, observations[2].occurrences)
        self.assertEqual(observations[3].values, observations[4].values)
        self.assertNotEqual(observations[3].occurrences, observations[4].occurrences)
        for value in [None, []]:
            s = td()
            form(s)["htv:methodName"] = value
            with self.assertRaises(Unsupported):
                http.lower(resolve(s)[0])

    def test_alias_collision_cannot_overwrite_an_instruction(self):
        s = td(context={"verb": http.HTTP + "methodName"})
        form(s).update({"htv:methodName": "GET", "verb": "POST"})
        ext = resolve(s)[0].extensions.get(http.HTTP + "methodName", Credit())
        self.assertEqual(len(ext.values), 2)
        self.assertEqual(len(ext.occurrences), 2)
        with self.assertRaisesRegex(Unsupported, "one literal"):
            http.lower(resolve(s)[0])

    def test_unknown_undefined_and_explicitly_ignored_terms_are_visible(self):
        for term in ["unknownInstruction", "oops:methodName", EX + "requiredFeature"]:
            s = td()
            form(s)[term] = "POST"
            with self.assertRaisesRegex(Unsupported, "unknown extension"):
                http.lower(resolve(s)[0])
        s = td(context={"discarded": None})
        form(s)["discarded"] = {"still": "preserved as opaque evidence"}
        i = resolve(s)[0]
        self.assertEqual(i.extensions.unresolved[0][0], "discarded")
        with self.assertRaisesRegex(Unsupported, "unresolved"):
            http.lower(i)

    def test_remote_context_denial_and_unsupported_jsonld_feature(self):
        s = td()
        s["@context"].append("https://contexts.test/unavailable")
        with self.assertRaises(jsonld.JsonLdError):
            resolve(s)
        s = td()
        form(s)["@reverse"] = {EX + "requires": {"@id": "urn:dependency"}}
        with self.assertRaisesRegex(ResolutionError, "unsupported semantic feature"):
            resolve(s)

    def test_security_override_and_response_mapping_preserved(self):
        s = td()
        s["securityDefinitions"]["auth"] = {"scheme": "basic"}
        form(s)["security"] = ["auth"]
        i = resolve(s)[0]
        self.assertEqual(i.security_origin, "form")
        self.assertEqual(i.security[0].parameters, (("in", "header"),))
        with self.assertRaisesRegex(Unsupported, "nosec"):
            http.lower(i)
        form(s)["security"] = []
        self.assertEqual(resolve(s)[0].security, ())
        with self.assertRaises(Unsupported):
            http.lower(resolve(s)[0])
        del form(s)["security"]
        form(s)["response"] = {"contentType": "application/json", "htv:statusCodeValue": 201}
        self.assertEqual(http.lower(resolve(s)[0]).expected_status, 201)

    def test_consumer_selection_supported_alternatives_and_strict_selection(self):
        s = td([{"href": "coap://unsupported.test/p"}, {"href": "https://device.test/p"},
                {"href": "mqtt://broker.test", "mqv:filter": "sensors/p", "mqv:retain": True}])
        views = resolve(s)
        supported, diagnostics = candidates(views, "p", "readproperty", BINDINGS)
        self.assertEqual([(c.interaction.form_index, c.binding) for c in supported], [(1, "http"), (2, "mqtt")])
        chosen, _ = select(views, "p", "readproperty", BINDINGS)
        self.assertEqual(chosen.binding, "http")
        self.assertTrue(diagnostics)
        with self.assertRaises(Unsupported):
            select(views, "p", "readproperty", BINDINGS, strict_form=0)
        self.assertEqual(select(views, "p", "readproperty", BINDINGS, strict_form=2)[0].binding, "mqtt")
        with self.assertRaisesRegex(Unsupported, "uncovered"):
            producer_coverage(views, BINDINGS)
        producer_coverage(views[1:], BINDINGS)
        with self.assertRaisesRegex(Unsupported, "ambiguous"):
            producer_coverage(views[1:], BINDINGS + (("other-http", http.lower),))

    def test_unknown_first_form_does_not_hide_a_supported_consumer_form(self):
        s = td([{"href": "https://device.test/p", EX + "required": True},
                {"href": "https://device.test/alternative"}])
        self.assertEqual(select(resolve(s), "p", "readproperty", BINDINGS)[0].interaction.form_index, 1)

    def test_strict_form_and_binding_constraints_precede_lowering(self):
        s = td([{"href": "https://device.test/bad"}, {"href": "https://device.test/good"}])
        calls = []
        def unrelated_binding(i):
            raise MemoryError("unrequested Binding")
        def lower(i):
            calls.append((i.name, i.form_index))
            if i.form_index == 0:
                raise MemoryError("unrelated earlier Form")
            return http.lower(i)
        bindings = (("unrelated", unrelated_binding), ("http", lower))
        selected, diagnostics = select(resolve(s), "p", "readproperty", bindings,
                                       strict_form=1, strict_binding="http")
        self.assertEqual(selected.program.endpoint, "https://device.test/good")
        self.assertEqual(calls, [("p", 1)])
        self.assertEqual(diagnostics, [])
        for constraints in ({"strict_form": 9}, {"strict_binding": "missing"}):
            with self.assertRaises(Unsupported):
                select(resolve(s), "p", "readproperty", bindings, **constraints)
        self.assertEqual(calls, [("p", 1)])
        with self.assertRaisesRegex(MemoryError, "earlier Form"):
            select(resolve(s), "p", "readproperty", bindings, strict_form=0, strict_binding="http")

    def test_affordance_and_operation_constraints_precede_lowering(self):
        s = td()
        s["properties"]["q"] = {"type": "number", "readOnly": True,
                                    "forms": [{"href": "https://device.test/q"}]}
        calls = []
        def lower(i):
            calls.append((i.name, i.form_index))
            if i.name == "p":
                raise MemoryError("unrelated property")
            return http.lower(i)
        views, bindings = resolve(s), (("http", lower),)
        selected, _ = select(views, "q", "readproperty", bindings, strict_form=0)
        self.assertEqual((selected.interaction.name, selected.interaction.form_index), ("q", 0))
        self.assertEqual(selected.program.endpoint, "https://device.test/q")
        self.assertEqual(calls, [("q", 0)])
        for name, operation in (("missing", "readproperty"), ("q", "writeproperty")):
            with self.assertRaises(Unsupported):
                select(views, name, operation, bindings)
        self.assertEqual(calls, [("q", 0)])
        with self.assertRaisesRegex(MemoryError, "unrelated property"):
            select(views, "p", "readproperty", bindings, strict_form=0)

    def test_declared_domain_annotations_compose_with_both_protocols(self):
        # Include the review's example IRI and a published SAREF domain class.
        for iri in ("https://schema.org/TemperatureSensor", SAREF_SENSOR):
            profile = AnnotationProfile((("thing", iri), ("affordance", iri)))
            for lower, source_form in ((http.lower, {"href": "https://device.test/p"}),
                    (mqtt.lower, {"href": "mqtt://broker.test", "mqv:filter": "sensors/p", "mqv:retain": True})):
                with self.subTest(iri=iri, lower=lower.__module__):
                    s = td([source_form])
                    baseline = lower(resolve(s)[0])
                    # A type-scoped context remaps its own alias for nested
                    # values; the class identity uses the prior type context.
                    s["@context"].append({"Sensor": {"@id": iri, "@context": {
                        "Sensor": EX + "DifferentInsideScope"}}})
                    s["@type"] = iri
                    s["properties"]["p"]["@type"] = "Sensor"
                    with self.assertRaises(Unsupported):
                        lower(resolve(s)[0])  # Empty-profile counterexample survives.
                    i = resolve(s, annotation_profile=profile)[0]
                    self.assertEqual(lower(i), baseline)
                    self.assertEqual(i.annotations, (ScopedAnnotations("thing", (iri,)),
                                                     ScopedAnnotations("affordance", (iri,))))
                    self.assertIn(iri, i.schema.types)
                    self.assertIn("https://www.w3.org/2019/wot/json-schema#NumberSchema", i.schema.types)

    def test_annotation_profile_cannot_hide_required_unknown_instructions(self):
        profile = AnnotationProfile((("thing", SAREF_SENSOR), ("affordance", SAREF_SENSOR)))
        for scope in ("thing", "affordance", "form"):
            s = td()
            s["@type"] = SAREF_SENSOR
            # The fixture defines this as a mandatory native confirmation rule.
            target = {"thing": s, "affordance": s["properties"]["p"], "form": form(s)}[scope]
            target[EX + "requireConfirmation"] = "before-success"
            with self.assertRaisesRegex(Unsupported, "unknown extension"):
                http.lower(resolve(s, annotation_profile=profile)[0])
        with self.assertRaisesRegex(ResolutionError, "Thing/affordance"):
            AnnotationProfile((("form", EX + "requireConfirmation"),))
        s = td()
        form(s)["@type"] = SAREF_SENSOR
        with self.assertRaises(Unsupported):
            http.lower(resolve(s, annotation_profile=profile)[0])

    def test_annotation_profile_is_exact_and_scope_specific(self):
        s = td()
        s["@type"] = SAREF_SENSOR
        with self.assertRaises(Unsupported):
            http.lower(resolve(s, annotation_profile=AnnotationProfile((("affordance", SAREF_SENSOR),)))[0])
        s["@type"] = "https://saref.etsi.org/core/UnknownNativeBehavior"
        with self.assertRaises(Unsupported):
            http.lower(resolve(s, annotation_profile=AnnotationProfile((("thing", SAREF_SENSOR),)))[0])

    def test_query_exhaustion_and_nonsemantic_failure_do_not_become_fallback(self):
        with self.assertRaises(Exhausted):
            http.lower(resolve(td())[0], Credit(0))
        def broken(_):
            raise MemoryError("construction failure is not Unsupported")
        with self.assertRaises(MemoryError):
            select(resolve(td()), "p", "readproperty", (("broken", broken),) + BINDINGS)
        with self.assertRaisesRegex(ResolutionError, "byte cap"):
            resolve(td(), max_bytes=1)

    def test_core_remapping_and_templates_refused(self):
        s = td()
        form(s)["@context"] = {"href": EX + "not-a-target"}
        with self.assertRaisesRegex(ResolutionError, "remapped"):
            resolve(s)
        s = td([{"href": "https://device.test/{id}"}])
        with self.assertRaisesRegex(ResolutionError, "templates"):
            resolve(s)

    def test_datatype_and_scope_information_is_not_automatic_understanding(self):
        s = td()
        form(s)["htv:methodName"] = {"@value": "GET", "@type": EX + "CustomInstruction"}
        self.assertEqual(resolve(s)[0].extensions.get(http.HTTP + "methodName", Credit()).values,
                         (Atom("GET", datatype=EX + "CustomInstruction"),))
        with self.assertRaisesRegex(Unsupported, "datatype"):
            http.lower(resolve(s)[0])
        s = td()
        s[EX + "requirement"] = "root"
        s["properties"]["p"][EX + "requirement"] = "affordance"
        i = resolve(s)[0]
        self.assertEqual(i.root_extensions.scalar(EX + "requirement", str, Credit()), "root")
        self.assertEqual(i.affordance_extensions.scalar(EX + "requirement", str, Credit()), "affordance")
        with self.assertRaisesRegex(Unsupported, "unknown extension"):
            http.lower(i)

    def test_counterexample_presence_does_not_preserve_array_null_positions(self):
        a, b = td(), td()
        form(a)[EX + "sequence"] = [None, "A"]
        form(b)[EX + "sequence"] = ["A", None]
        # Source loss, not a counterexample to JSON-LD's unordered semantics.
        self.assertEqual(resolve(a)[0].extensions, resolve(b)[0].extensions)
        for s in (a, b):
            with self.assertRaises(Unsupported):
                http.lower(resolve(s)[0])
        # A declared JSON literal keeps positional syntax without context parsing.
        for s in (a, b):
            s["@context"].append({"sequence": {"@id": EX + "sequence", "@type": "@json"}})
            form(s)["sequence"] = form(s).pop(EX + "sequence")
        self.assertNotEqual(resolve(a)[0].extensions, resolve(b)[0].extensions)


if __name__ == "__main__":
    unittest.main(verbosity=2)
