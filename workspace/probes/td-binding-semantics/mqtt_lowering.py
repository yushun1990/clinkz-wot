"""An MQTT author: broker URI, packets, topic/filter, QoS and retained read."""

from dataclasses import dataclass
from urllib.parse import urlsplit

from interface import Credit, Unsupported, common_support

MQTT = "http://www.example.org/mqtt-binding#"


@dataclass(frozen=True)
class MqttProgram:
    broker: str
    packet: str
    address: str
    qos: int
    retain: bool
    completion: str


def lower(i, credit=None):
    credit = credit if credit is not None else Credit()
    uri = urlsplit(i.target)
    if uri.scheme not in {"mqtt", "mqtts"}:
        raise Unsupported("MQTT scheme")
    common_support(i, credit)
    i.extensions.require_known(
        {MQTT + k for k in ("controlPacket", "topic", "filter", "qos", "retain")}, credit
    )
    i.response_extensions.require_known(set(), credit)
    defaults = {"readproperty": "subscribe", "writeproperty": "publish"}
    if i.operation not in defaults:
        raise Unsupported("MQTT example operation")
    packet = i.extensions.scalar(MQTT + "controlPacket", str, credit, defaults[i.operation])
    if packet != defaults[i.operation]:
        raise Unsupported("MQTT example packet/operation mapping")
    retain = i.extensions.scalar(MQTT + "retain", bool, credit, False)
    qos = i.extensions.scalar(MQTT + "qos", int, credit, 0)
    address = i.extensions.scalar(MQTT + ("filter" if packet == "subscribe" else "topic"), str, credit)
    if (not uri.hostname or uri.username or uri.password or uri.path not in {"", "/"}
            or uri.query or uri.fragment or not address or qos not in {0, 1, 2}):
        raise Unsupported("MQTT example broker/address/QoS")
    # Deliberately narrow: no wildcard response attribution in this probe.
    if "+" in address or "#" in address:
        raise Unsupported("MQTT example requires an exact topic")
    if i.operation == "readproperty" and not retain:
        raise Unsupported("read requires retained-value mapping")
    return MqttProgram(i.target, packet, address, qos, retain,
                       "first-retained-message" if packet == "subscribe" else "publish-accepted")
