"""An HTTP author: owns method/status interpretation and URI restrictions."""

from dataclasses import dataclass
from urllib.parse import urlsplit

from interface import Credit, Unsupported, common_support

HTTP = "http://www.w3.org/2011/http#"


@dataclass(frozen=True)
class HttpProgram:
    method: str
    endpoint: str
    expected_status: int | None


def lower(i, credit=None):
    credit = credit if credit is not None else Credit()
    uri = urlsplit(i.target)
    if uri.scheme not in {"http", "https"}:
        raise Unsupported("HTTP scheme")
    common_support(i, credit)
    i.extensions.require_known({HTTP + "methodName"}, credit, types={HTTP + "Request"})
    i.response_extensions.require_known({HTTP + "statusCodeValue"}, credit)
    defaults = {"readproperty": "GET", "writeproperty": "PUT", "invokeaction": "POST"}
    if i.operation not in defaults:
        raise Unsupported("HTTP example operation")
    explicit = i.extensions.get(HTTP + "methodName", credit)
    if explicit is not None and len(i.form_operations) != 1:
        raise Unsupported("explicit HTTP method with multiple effective operations")
    method = i.extensions.scalar(HTTP + "methodName", str, credit, defaults[i.operation])
    if method not in {"GET", "PUT", "POST"}:
        raise Unsupported("HTTP example method")
    if not uri.hostname or uri.username or uri.password or uri.fragment or "{" in i.target:
        raise Unsupported("HTTP example target")
    status = i.response_extensions.scalar(HTTP + "statusCodeValue", int, credit)
    if status is not None and not 100 <= status <= 599:
        raise Unsupported("HTTP status")
    return HttpProgram(method, i.target, status)
