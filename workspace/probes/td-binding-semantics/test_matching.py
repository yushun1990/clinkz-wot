"""Falsify capability matching; reuse #211's resolver, values and lowerers.

The first half is ordinary package-owned example contract/native code, NOT a
universal vocabulary registry. The criticality carrier is fixture-owned.
"""

from copy import deepcopy
from contextlib import closing
from dataclasses import dataclass, replace
from http.client import HTTPConnection
from http.server import BaseHTTPRequestHandler, HTTPServer
from itertools import permutations
from threading import Thread
import unittest
from urllib.parse import urlsplit

from engine import resolve
import http_lowering as http
import mqtt_lowering as mqtt
from interface import Atom, Credit, Node, Reference, Unsupported
from matching import Binding, Policy, Prepared, producer_assignments, select
from test_probe import td, form, SAREF_SENSOR
from test_profile_boundary import FOLLOWS, HTTP_BASIC, READ_RULE

# Fixed v1 execution contract. These identities belong to these example
# packages, not Engine Core. A real mapping can group many terms in one feature.
METHOD = http.HTTP + "methodName"
STATUS = http.HTTP + "statusCodeValue"
HEADERS = http.HTTP + "headers"
CONFIRM = "urn:probe:instruction:confirmation-v1"
MUST = "urn:probe:contract-v1:requires"
LABEL = "http://www.w3.org/2000/01/rdf-schema#label"
HTTP_FEATURES = frozenset({("form", METHOD), ("response", STATUS), ("form", READ_RULE)} |
    {(scope, CONFIRM) for scope in ("thing", "affordance", "form")})
MQTT_FEATURES = frozenset(("form", mqtt.MQTT + term)
    for term in ("controlPacket", "topic", "filter", "qos", "retain"))


def scopes(i):
    return (("thing", i.root_extensions), ("affordance", i.affordance_extensions),
            ("form", i.extensions), ("response", i.response_extensions)) + tuple(
                ("security", s.extensions) for s in i.security)


def http_requirements(i):
    if urlsplit(i.target).scheme not in {"http", "https"}:
        return set()
    return {(scope, term) for scope, node, terms in (
        ("form", i.extensions, (METHOD, HEADERS)),
        ("response", i.response_extensions, (STATUS,)))
        for term in terms if node.get(term, Credit()) is not None}


def mqtt_requirements(i):
    if urlsplit(i.target).scheme not in {"mqtt", "mqtts"}:
        return set()
    return {key for key in mqtt_features(i) if i.extensions.get(key[1], Credit()) is not None}


def mqtt_features(i):
    address = "filter" if i.operation == "readproperty" else "topic"
    return MQTT_FEATURES - {("form", mqtt.MQTT + ("topic" if address == "filter" else "filter"))}


def prepare(source):
    """Package contract fixes requirements without access to Binding inventory.

    Unknown extension values/classes stay in the original Interaction. The base
    contract permits ignoring them, unless a known mapping, explicit criticality
    carrier, or interpreted Profile establishes a requirement. No inert-term list.
    Unknown standard Profile claims are conservatively refused in this slice.
    """
    interactions = resolve(source)
    claim = interactions[0].root_extensions.get(FOLLOWS, Credit())
    advertised = frozenset()
    if claim is not None:
        if not claim.values or any(not isinstance(v, Atom) or type(v.value) is not str
                or v.datatype or v.language or v.direction for v in claim.values):
            raise Unsupported("invalid Profile advertisement")
        advertised = frozenset(v.value for v in claim.values)
        if advertised != {HTTP_BASIC}:
            raise Unsupported("unavailable Profile interpretation")
        if not any(urlsplit(i.target).scheme in {"http", "https"} and i.operation == "readproperty"
                   for i in interactions):
            raise Unsupported("advertised Profile lacks its HTTP read path")
    prepared = []
    for i in interactions:
        required = set()
        for scope, node in scopes(i):
            critical = node.get(MUST, Credit())
            if critical is not None:
                if not critical.values or any(not isinstance(v, Reference) for v in critical.values):
                    raise Unsupported("criticality requires resolved term references")
                for value in critical.values:
                    if node.get(value.iri, Credit()) is None:
                        raise Unsupported("critical instruction missing in its semantic scope")
                    required.add((scope, value.iri))
            if node.get(CONFIRM, Credit()) is not None:
                required.add((scope, CONFIRM))
        scheme = urlsplit(i.target).scheme
        # Positive operational rules for supported mappings, not a classification
        # of arbitrary extensions. These remain installed if a Binding is removed.
        required.update(http_requirements(i))
        required.update(mqtt_requirements(i))
        profile_paths = frozenset()
        if advertised and scheme in {"http", "https"} and i.operation != "readproperty":
            raise Unsupported("Profile interpreter outside HTTP read slice")
        if advertised and scheme in {"http", "https"} and i.operation == "readproperty":
            required.add(("form", READ_RULE))
            profile_paths = advertised
        prepared.append(Prepared(i, frozenset(required), profile_paths))
    return tuple(prepared)


def native_view(i, form_terms=(), response_terms=()):
    """Adapt to the old conservative lowerers AFTER requirements are fixed.

    Original semantic nodes are retained in Prepared/Selection. This projection
    is only for reusing #211's native code, not destructive semantic preparation.
    Scheme/parameter/coding/subprotocol facts remain intact; security extension
    requirements, like other extensions, have already been established.
    """
    def projected(node, terms):
        return Node(tuple(m for m in node.fields if m.iri in terms))
    return replace(i, root_extensions=Node(), affordance_extensions=Node(),
        extensions=projected(i.extensions, form_terms),
        response_extensions=projected(i.response_extensions, response_terms),
        security=tuple(replace(s, extensions=Node()) for s in i.security))


@dataclass(frozen=True)
class HttpDefinition:
    native: http.HttpProgram
    accept: str | None = None
    confirmation: str | None = None


@dataclass(frozen=True)
class HttpRoute:
    method: str
    endpoint: str
    response_status: int
    response_media: str
    confirmation: str | None


def http_definition(native, role, accept=None, confirmation=None):
    if role == "consumer":
        return HttpDefinition(native, accept, confirmation)
    return HttpRoute(native.method, native.endpoint, native.expected_status or 200,
                     "application/json", confirmation)


def default_http(item, role):
    if item.required:
        raise Unsupported("default HTTP has no extra semantics")
    return http_definition(http.lower(native_view(item.interaction)), role)


def specific_http(item, role):
    if not item.required <= HTTP_FEATURES:
        raise Unsupported("unsupported HTTP semantics")
    i = item.interaction
    native = http.lower(native_view(i, {METHOD}, {STATUS}))
    confirmation = None
    for scope, node in scopes(i):
        if (scope, CONFIRM) in item.required:
            if node.scalar(CONFIRM, str, Credit()) != "before-success":
                raise Unsupported("confirmation parameter")
            confirmation = "before-success"
    accept = None
    if ("form", READ_RULE) in item.required:
        if native.method != "GET" or native.expected_status not in {None, 200}:
            raise Unsupported("Profile/native instruction conflict")
        native = replace(native, expected_status=200)
        accept = "application/json"
    return http_definition(native, role, accept, confirmation)


@dataclass(frozen=True)
class MqttWriteRoute:
    broker: str
    incoming_publish_topic: str
    qos: int
    expected_retain: bool


def native_mqtt(item, role):
    if not item.required <= mqtt_features(item.interaction):
        raise Unsupported("unsupported MQTT semantics")
    native = mqtt.lower(native_view(item.interaction, {key[1] for key in MQTT_FEATURES}))
    if role == "producer":
        if item.interaction.operation == "readproperty":
            raise Unsupported("retained Consumer read has no per-request Producer handler route")
        return MqttWriteRoute(native.broker, native.address, native.qos, native.retain)
    return native


ROLES = frozenset({"consumer", "producer"})
OPS = frozenset({"readproperty", "writeproperty"})
DEFAULT = Binding("default", frozenset({"http", "https"}), ROLES, OPS, frozenset(), default_http)
SPECIFIC = Binding("specific", DEFAULT.schemes, ROLES, OPS, HTTP_FEATURES, specific_http)
MQTT = Binding("mqtt", frozenset({"mqtt", "mqtts"}), ROLES, OPS, MQTT_FEATURES, native_mqtt)
BINDINGS = (DEFAULT, SPECIFIC, MQTT)


def mixed():
    return td([{"href": "https://device.test/p"}, {"href": "mqtt://broker.test",
        "mqv:filter": "sensors/p", "mqv:retain": True}])


def mixed_write():
    s = td([{"href": "https://device.test/p"}, {"href": "mqtt://broker.test", "mqv:topic": "commands/p"}])
    s["properties"]["p"].pop("readOnly")
    s["properties"]["p"]["writeOnly"] = True
    return s


def choose(source, bindings=BINDINGS, **constraints):
    return select(prepare(source), "p", "readproperty", bindings, **constraints)[0]


class MatchingProbe(unittest.TestCase):
    def test_http_native_execution_preserves_metadata_and_obeys_explicit_instructions(self):
        requests = []
        class Capture(BaseHTTPRequestHandler):
            def respond(self):
                requests.append((self.command, self.path, self.headers.get("Accept")))
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b"42")
            do_GET = respond
            do_POST = respond
            def log_message(self, *args):
                pass
        with HTTPServer(("127.0.0.1", 0), Capture) as server:
            worker = Thread(target=server.serve_forever, daemon=True)
            worker.start()
            try:
                s = td([{"href": f"http://127.0.0.1:{server.server_port}/p"}])
                def execute(source):
                    definition = choose(source).program
                    uri = urlsplit(definition.native.endpoint)
                    with closing(HTTPConnection(uri.hostname, uri.port, timeout=2)) as client:
                        client.request(definition.native.method, uri.path,
                            headers={"Accept": definition.accept} if definition.accept else {})
                        response = client.getresponse()
                        self.assertEqual(response.read(), b"42")
                        if definition.native.expected_status is not None:
                            self.assertEqual(response.status, definition.native.expected_status)
                execute(s)
                s["@type"] = SAREF_SENSOR
                s[LABEL] = "temperature"
                form(s)["urn:vendor:unrelated"] = "description"
                execute(s)
                self.assertEqual(requests[0], requests[1])
                form(s)[METHOD] = "POST"
                execute(s)
                self.assertEqual(requests[-1], ("POST", "/p", None))
                del form(s)[METHOD]
                s["profile"] = HTTP_BASIC
                execute(s)
                self.assertEqual(requests[-1], ("GET", "/p", "application/json"))
            finally:
                server.shutdown()
                worker.join(timeout=2)

    def test_metadata_preserved_at_all_tested_scopes_without_native_changes(self):
        for source in (td(), mixed()):
            baseline = {b: choose(source, strict_binding=b).program
                        for b in (("default", "specific") if source == td() else ("default", "specific", "mqtt"))}
            source["@type"] = SAREF_SENSOR
            source["properties"]["p"]["@type"] = SAREF_SENSOR
            for target in (source, source["properties"]["p"], source["securityDefinitions"]["none"],
                           *source["properties"]["p"]["forms"]):
                target[LABEL] = "temperature"
                target["urn:vendor:unfamiliar"] = {"@value": {"label": [None, "x"]}, "@type": "@json"}
            for name, program in baseline.items():
                chosen = choose(source, strict_binding=name)
                self.assertEqual(chosen.program, program)
                self.assertIn(SAREF_SENSOR, chosen.interaction.root_extensions.types)
                self.assertIsNotNone(chosen.interaction.extensions.get(LABEL, Credit()))
            with self.assertRaises(Unsupported):  # A/B: original conservative model.
                http.lower(resolve(source)[0])

    def test_context_namespaces_are_not_requirements_and_aliases_are_equivalent(self):
        source = td()
        original = choose(source).program
        source["@context"].append({"irrelevant": "urn:unused:", "h": http.HTTP})
        self.assertEqual(prepare(source)[0].required, frozenset())
        self.assertEqual(choose(source).program, original)
        results = []
        for context, term in (({"h": http.HTTP}, "h:methodName"),
                              ({"verb": METHOD}, "verb"), ({}, METHOD)):
            s = td(context=context)
            form(s)[term] = "POST"
            item = prepare(s)[0]
            self.assertEqual(item.required, {("form", METHOD)})
            results.append(choose(s).program)
        self.assertTrue(all(p == results[0] for p in results))
        self.assertEqual(results[0].native.method, "POST")
        self.assertNotEqual(results[0], original)

    def test_type_scoped_context_is_applied_before_any_ignoring(self):
        s = td(context={"Tagged": {"@id": "urn:vendor:Tagged", "@context": {"verb": METHOD}}})
        form(s).update({"@type": "Tagged", "verb": "POST"})
        self.assertEqual(choose(s).program.native.method, "POST")
        self.assertEqual(prepare(s)[0].required, {("form", METHOD)})
        s = td()
        s[METHOD] = "POST"  # Same expanded IRI, different semantic scope.
        self.assertEqual(prepare(s)[0].required, frozenset())
        self.assertEqual(choose(s).program.native.method, "GET")

    def test_explicit_method_response_and_mqtt_instruction_change_definition(self):
        s = td()
        form(s).update({METHOD: "POST", "response": {STATUS: 201}})
        selected = choose(s)
        self.assertEqual(selected.binding, "specific")
        self.assertEqual((selected.program.native.method, selected.program.native.expected_status), ("POST", 201))
        s = mixed()
        first = choose(s, strict_binding="mqtt").program
        s["properties"]["p"]["forms"][1]["mqv:filter"] = "sensors/other"
        self.assertNotEqual(choose(s, strict_binding="mqtt").program, first)

    def test_mqtt_requirements_follow_the_effective_operation_not_every_present_term(self):
        s = td([{"href": "mqtt://broker.test", "mqv:filter": "sensors/p",
                 "mqv:topic": "commands/p", "mqv:retain": True}])
        s["properties"]["p"].pop("readOnly")
        items = prepare(s)
        self.assertIn(("form", mqtt.MQTT + "filter"), items[0].required)
        self.assertNotIn(("form", mqtt.MQTT + "topic"), items[0].required)
        self.assertIn(("form", mqtt.MQTT + "topic"), items[1].required)
        self.assertNotIn(("form", mqtt.MQTT + "filter"), items[1].required)
        read, _ = select(items, "p", "readproperty", BINDINGS)
        write, _ = select(items, "p", "writeproperty", BINDINGS)
        self.assertEqual((read.program.address, write.program.address), ("sensors/p", "commands/p"))

    def test_unknown_advisory_and_explicitly_mandatory_are_different(self):
        for scope in ("thing", "affordance", "form", "response", "security"):
            s = td(context={"must": {"@id": MUST, "@type": "@id"}})
            form(s)["response"] = {}
            target = {"thing": s, "affordance": s["properties"]["p"],
                      "form": form(s), "response": form(s)["response"],
                      "security": s["securityDefinitions"]["none"]}[scope]
            unknown = "urn:unfamiliar:confirmation"
            target[unknown] = "before-success"
            self.assertEqual(choose(s).binding, "default")
            target["must"] = unknown
            self.assertIn((scope, unknown), prepare(s)[0].required)
            with self.assertRaisesRegex(Unsupported, "required semantics"):
                choose(s)

    def test_known_unsupported_http_instruction_is_required_without_a_capable_binding(self):
        s = td()
        form(s)[HEADERS] = [{http.HTTP + "fieldName": "X-Required", http.HTTP + "fieldValue": "one"}]
        self.assertIn(("form", HEADERS), prepare(s)[0].required)
        for bindings in (BINDINGS, (DEFAULT,)):
            with self.assertRaisesRegex(Unsupported, "required semantics"):
                choose(s, bindings)
        del form(s)[HEADERS]
        form(s)[http.HTTP + "unfamiliarDisplayHint"] = "green"
        self.assertEqual(choose(s).binding, "default")  # Not a namespace-wide rule.

    def test_new_scheme_and_explicit_semantics_need_no_engine_case(self):
        meaning = "urn:third-party:nonce-v1"
        feature = frozenset({("form", meaning)})
        s = td([{"href": "newwire://device.test/p", "nonce": "one", "must": meaning}],
               {"nonce": meaning, "must": {"@id": MUST, "@type": "@id"}})
        def compile_new(item, role):
            if item.required != feature:
                raise Unsupported("newwire contract")
            return (role, item.interaction.target, item.interaction.extensions.scalar(meaning, str, Credit()))
        third_party = Binding("third-party", frozenset({"newwire"}), ROLES, OPS, feature, compile_new)
        self.assertEqual(choose(s, (third_party,)).program, ("consumer", "newwire://device.test/p", "one"))
        form(s)["nonce"] = "two"
        self.assertEqual(choose(s, (third_party,)).program[-1], "two")

    def test_criticality_carrier_cannot_hide_absent_or_malformed_instructions(self):
        for declaration in (None, [], "urn:missing:instruction", {"not-an-iri": True}):
            s = td(context={"must": {"@id": MUST, "@type": "@id"}})
            form(s)["must"] = declaration
            with self.assertRaisesRegex(Unsupported, "critical"):
                prepare(s)

    def test_required_semantics_survive_binding_removal_preferences_and_strict_choice(self):
        for scope in ("thing", "affordance", "form"):
            s = mixed() if scope in {"thing", "affordance"} else td()
            target = {"thing": s, "affordance": s["properties"]["p"], "form": form(s)}[scope]
            target[CONFIRM] = "before-success"
            prepared = prepare(s)
            before = prepared[0].required
            for bindings in permutations(BINDINGS):
                chosen, _ = select(prepared, "p", "readproperty", bindings,
                    policy=Policy(bindings=("default", "specific")))
                self.assertEqual(chosen.binding, "specific")
                self.assertEqual(chosen.program.confirmation, "before-success")
                self.assertIs(prepared[0].required, before)
            for bindings, constraints in (((DEFAULT, MQTT), {}),
                    (BINDINGS, {"strict_binding": "default"})):
                with self.assertRaises(Unsupported):
                    select(prepared, "p", "readproperty", bindings, **constraints)

    def test_fewer_recognized_terms_can_satisfy_more_required_semantics(self):
        s = td()
        metadata = {"urn:vendor:field" + str(n) for n in range(6)}
        form(s).update({key: "description" for key in metadata})
        seen = set()
        def descriptive_http(item, role):
            for term in metadata:
                if item.interaction.extensions.scalar(term, str, Credit()) is not None:
                    seen.add(term)  # Actually read/understood, with no native effect.
            return default_http(item, role)
        descriptive = replace(DEFAULT, compile=descriptive_http)
        self.assertEqual(choose(s, (descriptive,)).program, choose(td()).program)
        self.assertEqual(seen, metadata)
        form(s)[CONFIRM] = "before-success"
        # Actual recognition is diagnostic-only, never an eligibility substitute.
        recognized = {"default": seen, "specific": {CONFIRM}}
        actual = {m.iri for m in prepare(s)[0].interaction.extensions.fields}
        self.assertGreater(len(actual & recognized["default"]), len(actual & recognized["specific"]))
        self.assertEqual(choose(s, (descriptive, SPECIFIC), policy=Policy(bindings=("default",))).binding, "specific")

    def test_deriving_requirements_from_registered_capabilities_silently_weakens_mapping(self):
        s = td()
        form(s)[METHOD] = "POST"
        item = prepare(s)[0]
        with self.assertRaises(Unsupported):
            select((item,), "p", "readproperty", (DEFAULT,))
        # Deliberate anti-model: infer requirements only from offered features.
        offered = frozenset().union(*(b.features for b in (DEFAULT,)))
        weakened = replace(item, required=item.required & offered)
        chosen, _ = select((weakened,), "p", "readproperty", (DEFAULT,))
        self.assertEqual(chosen.program.native.method, "GET")  # Violates explicit POST.
        self.assertEqual(item.required, {("form", METHOD)})

    def test_profile_requirements_are_scoped_but_active_profile_cannot_fallback(self):
        s = mixed()
        s["profile"] = HTTP_BASIC
        items = prepare(s)
        self.assertIn(("form", READ_RULE), items[0].required)
        self.assertNotIn(("form", READ_RULE), items[1].required)
        policy = Policy(required_profiles=frozenset({HTTP_BASIC}), schemes=("mqtt",))
        for bindings in permutations(BINDINGS):
            chosen, _ = select(items, "p", "readproperty", bindings, policy=policy)
            self.assertEqual(chosen.binding, "specific")
            self.assertEqual((chosen.program.accept, chosen.program.native.expected_status), ("application/json", 200))
        with self.assertRaises(Unsupported):
            select(items, "p", "readproperty", (DEFAULT, MQTT), policy=policy)
        with self.assertRaises(Unsupported):
            choose(s, policy=policy, strict_form=1)
        # Without the application's required path, an additional MQTT path is legal.
        self.assertEqual(choose(s, policy=Policy(schemes=("mqtt",))).binding, "mqtt")
        form(s)[METHOD] = "POST"
        with self.assertRaisesRegex(Unsupported, "conflict"):
            choose(s, policy=policy)

    def test_profile_activation_and_missing_interpretation_are_not_binding_decisions(self):
        policy = Policy(required_profiles=frozenset({HTTP_BASIC}))
        with self.assertRaisesRegex(Unsupported, "Profile"):
            choose(mixed(), policy=policy)
        for claim in ("urn:unknown:profile", [HTTP_BASIC, "urn:unknown:profile"]):
            s = mixed()
            s["profile"] = claim
            with self.assertRaisesRegex(Unsupported, "interpretation"):
                prepare(s)

    def test_static_capabilities_are_only_an_index_and_fatal_failures_are_not_skips(self):
        calls = []
        def once(item, role):
            calls.append(item.required)
            return specific_http(item, role)
        special = replace(SPECIFIC, features=None, compile=once)
        s = td()
        form(s)[CONFIRM] = "after-success"  # Advertised feature, unsupported value.
        with self.assertRaisesRegex(Unsupported, "confirmation parameter"):
            choose(s, (DEFAULT, special))
        self.assertEqual(len(calls), 1)
        form(s)[CONFIRM] = "before-success"
        self.assertEqual(choose(s, (DEFAULT, special)).program.confirmation, "before-success")
        self.assertEqual(len(calls), 2)  # No semantic index or separate supports pass needed.
        def fatal(item, role):
            raise RuntimeError("compiler/resource failure")
        broken = replace(DEFAULT, compile=fatal)
        with self.assertRaisesRegex(RuntimeError, "compiler/resource"):
            choose(td(), (broken, SPECIFIC))
        self.assertEqual(choose(td(), (broken, SPECIFIC), strict_binding="specific").binding, "specific")

    def test_security_media_operation_and_constraints_precede_compilation(self):
        def forbidden(item, role):
            self.fail("excluded candidate compiled")
        sentinel = replace(DEFAULT, compile=forbidden)
        for policy in (Policy(media=frozenset()), Policy(security=frozenset()),
                       Policy(allowed_schemes=frozenset())):
            with self.assertRaises(Unsupported):
                choose(td(), (sentinel,), policy=policy)
        s = td()
        s["securityDefinitions"]["auth"] = {"scheme": "basic"}
        s["security"] = ["auth"]
        with self.assertRaisesRegex(Unsupported, "security capability"):
            choose(s, (sentinel,))
        form(s)["security"] = ["none"]  # A real WoT Form override, not a fallback waiver.
        self.assertEqual(choose(s).binding, "default")
        s = td()
        form(s)["response"] = {"contentType": "application/cbor"}
        with self.assertRaisesRegex(Unsupported, "media capability"):
            choose(s, (sentinel,))
        for constraints in ({"strict_form": 9}, {"strict_binding": "missing"}):
            with self.assertRaises(Unsupported):
                choose(td(), (sentinel,), **constraints)
        with self.assertRaises(Unsupported):
            choose(td(), (replace(sentinel, operations=frozenset({"writeproperty"})),))

    def test_deterministic_preferences_only_order_eligible_candidates(self):
        for bindings in permutations(BINDINGS):
            self.assertEqual(choose(mixed(), bindings).binding, "default")
            self.assertEqual(choose(mixed(), bindings, policy=Policy(schemes=("mqtt",))).binding, "mqtt")
            self.assertEqual(choose(mixed(), bindings, policy=Policy(bindings=("specific",))).binding, "specific")
            self.assertEqual(choose(mixed(), bindings, policy=Policy(
                allowed_schemes=frozenset({"mqtt"}), bindings=("default",))).binding, "mqtt")
        s = td()
        s["properties"]["other"] = deepcopy(s["properties"]["p"])
        s["properties"]["other"]["forms"][0]["href"] = "https://device.test/other"
        chosen, _ = select(prepare(s), "other", "readproperty", BINDINGS, strict_form=0)
        self.assertEqual(chosen.interaction.target, "https://device.test/other")

    def test_producer_requires_every_coordinate_not_one_good_consumer_path(self):
        s = mixed()
        items = prepare(s)
        self.assertEqual(choose(s, (DEFAULT,)).binding, "default")
        with self.assertRaises(Unsupported):
            producer_assignments(items, (DEFAULT,), {("p", "readproperty")})
        with self.assertRaisesRegex(Unsupported, "no per-request Producer"):
            producer_assignments(items, BINDINGS, {("p", "readproperty")})
        items = prepare(mixed_write())
        for bindings in permutations(BINDINGS):
            owners = producer_assignments(items, bindings, {("p", "writeproperty")})
            self.assertEqual(tuple(o.binding for o in owners), ("default", "mqtt"))
            self.assertEqual(owners[0].program.method, "PUT")
            self.assertEqual(owners[1].program.incoming_publish_topic, "commands/p")
        with self.assertRaisesRegex(Unsupported, "handler"):
            producer_assignments(items, BINDINGS, set())
        with self.assertRaises(Unsupported):
            producer_assignments(items, (replace(DEFAULT, roles=frozenset({"consumer"})), MQTT),
                                 {("p", "writeproperty")})
        s["properties"]["p"].pop("readOnly")
        with self.assertRaises(Unsupported):
            producer_assignments(prepare(s), BINDINGS, {("p", "readproperty")})

    def test_producer_profile_coverage_allows_extra_forms_without_waiving_requirements(self):
        s = td()
        s["properties"]["command"] = mixed_write()["properties"]["p"]
        s["properties"]["command"]["forms"].pop(0)  # Extra MQTT write, not another Profile path.
        s["profile"] = HTTP_BASIC
        policy = Policy(required_profiles=frozenset({HTTP_BASIC}))
        handlers = {("p", "readproperty"), ("command", "writeproperty")}
        owners = producer_assignments(prepare(s), BINDINGS, handlers, policy=policy)
        self.assertEqual(tuple(o.binding for o in owners), ("specific", "mqtt"))
        self.assertEqual((owners[0].program.response_status, owners[0].program.response_media),
                         (200, "application/json"))
        with self.assertRaises(Unsupported):
            producer_assignments(prepare(s), (DEFAULT, MQTT), handlers, policy=policy)
        with self.assertRaises(Unsupported):
            producer_assignments(prepare(s), BINDINGS, handlers,
                                 policy=replace(policy, media=frozenset()))
        with self.assertRaisesRegex(Unsupported, "Profile"):
            producer_assignments(prepare(td()), BINDINGS, {("p", "readproperty")}, policy=policy)

    def test_matching_alone_does_not_detect_producer_native_route_collision(self):
        s = td()
        s["properties"]["other"] = deepcopy(s["properties"]["p"])
        owners = producer_assignments(prepare(s), BINDINGS, {("p", "readproperty"), ("other", "readproperty")})
        # Deliberate falsifier: individually eligible assignments would install
        # the same GET endpoint for two handlers. No publication is performed.
        self.assertEqual(len(owners), 2)
        self.assertEqual(owners[0].program, owners[1].program)
        self.assertNotEqual(owners[0].interaction.name, owners[1].interaction.name)

    def test_unknown_without_a_criticality_contract_is_fundamentally_ambiguous(self):
        s = td()
        form(s)["urn:unknown:requiresNonce"] = True
        chosen = choose(s)
        # Same input could be metadata or an undisclosed mandatory handshake.
        # Matching cannot establish which author meaning was intended.
        self.assertIsNone(chosen.program.confirmation)
        self.assertEqual(chosen.program.native.method, "GET")


if __name__ == "__main__":
    unittest.main(verbosity=2)
