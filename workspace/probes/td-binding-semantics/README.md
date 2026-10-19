# Disposable TD-to-Binding semantic probe

Evidence for [0056's semantic-boundary investigation](../../0056-target-spi-external-validation-and-cross-protocol-neutrality.md#semantic-boundary-investigation).
This directory is outside the Cargo workspace and all production dependency and
gate paths. It can be removed after the investigation. It is not a replacement
SPI implementation, TD validator, network runtime or embedded qualification.

From this directory, with Python 3.12:

```sh
python3 -m venv /tmp/clinkz-semantic-probe
/tmp/clinkz-semantic-probe/bin/pip install -r requirements.txt
/tmp/clinkz-semantic-probe/bin/python -m unittest -v
```

Only dependency installation uses the network. Tests use the checked-in TD
context and explicitly supplied test contexts; the document loader refuses all
other URLs. No credentials or external services are used. The analysis used
Python 3.12.14 and the pinned dependencies. Reproduction revisions and executed
checks belong in the investigation PR.

| File | Responsibility |
| --- | --- |
| `interface.py` | Public resolved facts, semantic values, scoped annotations, source occurrences, explicit unsupported result and bounded member queries. |
| `engine.py` | One experimental shared resolver, explicit annotation-role profile and generic candidate/selection/coverage logic. |
| `http_lowering.py` | Independently readable HTTP method/response lowering. Imports only the public interface and the standard library. |
| `mqtt_lowering.py` | Independently readable MQTT packet/topic/filter lowering, with the same imports and interface. |
| `test_probe.py` | Positive examples and intentional counterexamples, including one passing test that **proves information loss**. |

The HTTP and MQTT lowerers are 40 and 49 physical lines respectively,
including imports, comments and blank lines.
Both were written in this investigation; they are separate authoring examples,
not independent human implementations or external acceptance. They implement
small mappings, not conformant complete protocol bindings. The MQTT namespace
`http://www.example.org/mqtt-binding#` is explicitly fixture-owned. Packet,
topic/filter and retained-read conventions are drawn from the W3C MQTT binding
draft; the fixture does not claim that namespace or its QoS default is a W3C
standard. Broker URIs with topic paths and wildcard reads are deliberately
outside this author's supported mapping.

The 23 tests exercise:

- Prefix, alias, full-IRI and supplied remote-context equivalence at lowering.
- WoT operation defaults, HTTP per-operation defaults and source cardinality,
  MQTT publish/subscribe distinctions and retained-read refusal.
- Property/type scoped contexts, resolved IRI values, lists, typed and
  language-tagged literals, JSON literals and alias collisions.
- Missing/null/empty/singleton distinctions; unknown and dropped terms;
  unsupported datatypes, reverse edges and unavailable contexts.
- Security override versus inheritance, response mapping, supported Consumer
  alternatives, strict selection, ambiguous/uncovered Producer coordinates,
  query exhaustion and fatal failures that cannot become candidate skips.
- Declared domain classes on Thing/typed-property scopes compose with both
  mappings and remain preserved; the empty-profile counterexample, unknown
  mandatory instructions, wrong scope and same-namespace unknowns remain refused.
- Property, operation, strict Form and strict Binding constraints are applied
  before lowering, including two read properties with Form index zero and an
  excluded fatal compiler. Fatal failures from eligible lowerers still propagate.
- The explicit `[null,"A"]` / `["A",null]` counterexample: the tested expanded
  projection plus occurrence shape loses the distinction. Ordinary JSON-LD
  arrays are unordered, so this disproves arbitrary source preservation, not
  semantic equivalence for that data model. A declared JSON literal keeps
  positional syntax. A passing suite is not a universal sufficiency result.

`resolve(..., annotation_profile=AnnotationProfile(...))` accepts trusted exact
scope/type declarations for Thing/affordance domain annotations. It retains
them in `Interaction.annotations`; it cannot exempt Form/response/security
instructions, arbitrary properties or a whole namespace. There is no automatic
inertness inference. The tests use the review's example IRI and the published
SAREF TemperatureSensor class, with aliased names and numeric property schemas.
The investigation compares alternative policies and records the remaining
Owner trade-off. Neither Binding imports or maintains that policy.

`select(interactions, property_name, operation, bindings, *, strict_form=None,
strict_binding=None)` filters all those coordinates before calling any lowerer.
The required property name disambiguates Form indices. Producer coverage still
checks every advertised coordinate. Full-input resolution precedes selection;
these tests do not establish isolation from unrelated TD validation/context errors.

`Trace` instruments PyLD 3.1.0's private expansion hook to collect occurrences
under the actual active context. Bindings never see that hook or a context.
This avoids writing an approximate JSON-LD processor just to obtain evidence;
it also makes this code unsuitable as a production resolver. PyLD processes the
full document and can allocate before a query budget applies. Query credit and
the input byte cap prove neither bounded expansion nor recoverable allocation.
Python URL resolution here is only exercised for HTTP relative references and
already-absolute MQTT broker URIs; it is not generic RFC3986/6570 evidence.

The adapter handles the fixed lexical TD core of the fixtures. It omits full
validation, Actions/Events/collections, additional responses, schema linking,
URI templates, credential provision, codecs, resource ownership, install/close
and all network I/O. Unimplemented shared fields generally survive as unknown
members and are conservatively refused. Some critical core remappings are
checked, but arbitrary core aliases/redefinitions are not qualified. Do not
feed arbitrary TDs into this probe expecting admission safety. The investigation
identifies the stronger contract and remaining evidence needed before adoption.

`td-context.jsonld` is the unmodified 39,128-byte W3C context retrieved from
<https://www.w3.org/2022/wot/td/v1.1> on 2026-10-19. SHA-256:
`9298ddacce1d023e584dcd4b0e639baa228fb9fa8743dcad6d9c9c29db6ca069`.
It is included solely to make resolution reproducible offline. Copyright W3C;
see the [W3C Software and Document License](https://www.w3.org/copyright/software-license-2023/).
PyLD is an external dependency under its own license; it is not vendored here.
