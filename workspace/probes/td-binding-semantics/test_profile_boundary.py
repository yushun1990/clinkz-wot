"""A standard Thing.profile carrier, not a profile registry or production API.

Only the HTTP Basic draft's read request/response slice is interpreted here.
This does not implement or certify the complete profile or its Common Constraints.
The trusted dependency is supplied before selection, never obtained from a TD URL
or chosen by a candidate. All helpers are disposable, test-only responsibility
examples; the existing resolver, interface and native lowerers are unchanged.
"""

from dataclasses import dataclass, replace
import unittest
from urllib.parse import urlsplit

from engine import TD, select
import http_lowering as http
import mqtt_lowering as mqtt
from interface import Atom, Credit, Unsupported
from test_probe import td, form
from test_role_contract import prepare

FOLLOWS = TD + "followsProfile"
HTTP_BASIC = "https://www.w3.org/2022/wot/profile/http-basic/v1"
# Opaque to the Engine-side coverage check, interpreted by profile/native code.
READ_RULE = "urn:probe:requirement:http-basic-read-request-response:v1"
RDFS_LABEL = "http://www.w3.org/2000/01/rdf-schema#label"


@dataclass(frozen=True)
class Prepared:
    advertised: tuple[str, ...]
    interactions: tuple
    required: frozenset[str]


def prepare_profiles(interactions, trusted_interpreter=None):
    member = interactions[0].root_extensions.get(FOLLOWS, Credit())
    advertised = ()
    if member is not None:
        # The pinned TD context expands the key, but does not @id-coerce values.
        if not member.values or any(not isinstance(v, Atom) or type(v.value) is not str
                or v.datatype or v.language or v.direction or not urlsplit(v.value).scheme
                for v in member.values):
            raise Unsupported("profile URI values outside this test slice")
        advertised = tuple(v.value for v in member.values)
    required = frozenset()
    if advertised:
        if trusted_interpreter is None:
            raise Unsupported("profile interpretation unavailable before selection")
        required = trusted_interpreter(advertised, interactions)
    # Separate the standard advertisement from remaining extensions only AFTER
    # interpreting it. Retain the claims alongside immutable active requirements.
    views = tuple(replace(i, root_extensions=replace(i.root_extensions,
        fields=tuple(m for m in i.root_extensions.fields if m.iri != FOLLOWS)))
        for i in interactions)
    return Prepared(advertised, views, required)


def http_basic_read_slice(advertised, interactions):
    """Local maintained code for one known version/scope, outside Engine Core.

    Fixed source: WD-wot-profile-20251104, section 6.2.1.1. No conformance claim.
    """
    if set(advertised) != {HTTP_BASIC}:
        raise Unsupported("uninterpreted advertised profile; no partial activation")
    if any(i.operation != "readproperty" or urlsplit(i.target).scheme not in {"http", "https"}
           for i in interactions):
        raise Unsupported("profile interpreter only demonstrates HTTP read slice")
    return frozenset({READ_RULE})


def ordinary_http(i, required):
    return http.lower(i), frozenset()  # Implements no extra profile obligation.


@dataclass(frozen=True)
class ProfileReadProgram:
    request: http.HttpProgram
    accept: str


def profile_http(i, required):
    program = http.lower(i)
    if not required:
        return program, frozenset()
    if required != {READ_RULE} or program.method != "GET" or program.expected_status not in {None, 200}:
        raise Unsupported("unsupported profile requirement or contradictory native instruction")
    return ProfileReadProgram(replace(program, expected_status=200), "application/json"), required


def select_prepared(prepared, bindings, **constraints):
    def checked(lower):
        def compile_candidate(i):
            program, covered = lower(i, prepared.required)
            if not prepared.required <= covered:
                raise Unsupported("undischarged active profile requirements")
            return program
        return compile_candidate
    # Each candidate sees exactly the same prior requirement set in this slice.
    return select(prepared.interactions, "p", "readproperty",
                  tuple((name, checked(lower)) for name, lower in bindings), **constraints)


class ProfileBoundaryProbe(unittest.TestCase):
    def test_standard_advertisement_survives_but_old_lowerer_refuses(self):
        s = td()
        s["profile"] = HTTP_BASIC
        views = prepare(s)
        member = views[0].root_extensions.get(FOLLOWS, Credit())
        self.assertEqual(member.values, (Atom(HTTP_BASIC),))
        self.assertEqual(member.occurrences[0].spelling, "profile")
        with self.assertRaisesRegex(Unsupported, "followsProfile"):
            http.lower(views[0])
        # Merely stripping the claim would lose the active response obligation.
        stripped = replace(views[0], root_extensions=replace(views[0].root_extensions, fields=()))
        self.assertIsNone(http.lower(stripped).expected_status)

    def test_trusted_interpretation_precedes_binding_support_and_selection(self):
        for advertisement in (HTTP_BASIC, [HTTP_BASIC]):
            s = td()
            s["profile"] = advertisement
            prepared = prepare_profiles(prepare(s), http_basic_read_slice)
            self.assertEqual(prepared.advertised, (HTTP_BASIC,))
            self.assertEqual(prepared.required, frozenset({READ_RULE}))
            seen = []
            def plain(i, required):
                seen.append(required)
                return ordinary_http(i, required)
            def aware(i, required):
                seen.append(required)
                return profile_http(i, required)
            for bindings in ((('plain', plain), ('aware', aware)),
                             (('aware', aware), ('plain', plain))):
                chosen, diagnostics = select_prepared(prepared, bindings)
                self.assertEqual(chosen.binding, "aware")
                self.assertEqual(chosen.program.request.expected_status, 200)
                self.assertEqual(chosen.program.accept, "application/json")
                self.assertTrue(any("undischarged" in d[-1] for d in diagnostics))
                self.assertTrue(all(r is prepared.required for r in seen))
            with self.assertRaisesRegex(Unsupported, "undischarged"):
                select_prepared(prepared, (("plain", plain),))
            with self.assertRaises(Unsupported):
                select_prepared(prepared, (("plain", plain), ("aware", aware)), strict_binding="plain")

    def test_unavailable_and_unknown_profiles_fail_before_any_native_lowering(self):
        def unavailable_native(i, required):
            self.fail("native compilation must not run before profile interpretation")
        for advertisement, interpreter in ((HTTP_BASIC, None),
                ("urn:unknown:profile:v1", http_basic_read_slice),
                ([HTTP_BASIC, "urn:unknown:profile:v1"], http_basic_read_slice)):
            s = td()
            s["profile"] = advertisement
            with self.assertRaisesRegex(Unsupported, "profile"):
                prepared = prepare_profiles(prepare(s), interpreter)
                select_prepared(prepared, (("native", unavailable_native),))
        # Installation alone does not activate a profile that is not advertised.
        prepared = prepare_profiles(prepare(td()), http_basic_read_slice)
        self.assertEqual(prepared.required, frozenset())
        chosen, _ = select_prepared(prepared, (("plain", ordinary_http),))
        self.assertIsNone(chosen.program.expected_status)

    def test_profile_does_not_override_native_or_security_requirements(self):
        for mutate in (lambda s: form(s).update({"htv:methodName": "POST"}),
                       lambda s: form(s).update({"subprotocol": "unknown-dialect-v1"}),
                       lambda s: form(s).update({"urn:probe:unknownRequiredStep": True}),
                       lambda s: s.update(security=["basic"], securityDefinitions={"basic": {"scheme": "basic"}})):
            s = td()
            s["profile"] = HTTP_BASIC
            mutate(s)
            prepared = prepare_profiles(prepare(s), http_basic_read_slice)
            with self.assertRaises(Unsupported):
                select_prepared(prepared, (("plain", ordinary_http), ("aware", profile_http)))

    def test_unknown_non_type_annotations_remain_preserved_but_unsupported(self):
        for lower, source_form in ((http.lower, {"href": "https://device.test/p"}),
                (mqtt.lower, {"href": "mqtt://broker.test", "mqv:filter": "sensors/p", "mqv:retain": True})):
            for scope in ("thing", "affordance"):
                s = td([source_form])
                target = s if scope == "thing" else s["properties"]["p"]
                target[RDFS_LABEL] = "Room A temperature"
                i = prepare(s)[0]
                node = i.root_extensions if scope == "thing" else i.affordance_extensions
                self.assertEqual(node.get(RDFS_LABEL, Credit()).values, (Atom("Room A temperature"),))
                with self.assertRaisesRegex(Unsupported, "unknown extension"):
                    lower(i)
        # Activating a known profile does not bless unrelated annotation members.
        s = td()
        s["profile"] = HTTP_BASIC
        s["properties"]["p"][RDFS_LABEL] = "Room A temperature"
        prepared = prepare_profiles(prepare(s), http_basic_read_slice)
        with self.assertRaisesRegex(Unsupported, "unknown extension"):
            select_prepared(prepared, (("aware", profile_http),))


if __name__ == "__main__":
    unittest.main(verbosity=2)
