#!/usr/bin/env python3
"""Run the secure application example against independent Python/OpenSSL peers.

From any directory: python3 discovery/tests/support/secure_peers.py
Requires cargo, Python's standard library and openssl. Uses ephemeral loopback
ports, temporary certificates and fixture-only credentials; no external service.
This is deliberately separate from the Rust transcript peers and Cargo tests.
"""

import base64
import contextlib
import http.server
import json
import os
from pathlib import Path
import ssl
import subprocess
import tempfile
import threading
import time

REPO = Path(__file__).resolve().parents[3]
DIRECTORY_AUTH = ("directory-reader", "directory-only")
THING_AUTH = ("sensor-reader", "thing-only")


def certificate(directory, name):
    key, pem, der = (directory / f"{name}.{suffix}" for suffix in ("key", "pem", "der"))
    subprocess.run(
        ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-noenc",
         "-keyout", str(key), "-out", str(pem), "-days", "1",
         "-subj", f"/CN={name}", "-addext", "subjectAltName=IP:127.0.0.1",
         "-addext", "basicConstraints=critical,CA:FALSE",
         "-addext", "keyUsage=critical,digitalSignature,keyEncipherment",
         "-addext", "extendedKeyUsage=serverAuth"],
        check=True, capture_output=True,
    )
    subprocess.run(
        ["openssl", "x509", "-in", str(pem), "-outform", "DER", "-out", str(der)],
        check=True, capture_output=True,
    )
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.minimum_version = context.maximum_version = ssl.TLSVersion.TLSv1_3
    context.set_alpn_protocols(["http/1.1"])
    context.load_cert_chain(pem, key)
    return context, der


@contextlib.contextmanager
def peer(context, credentials, path, media):
    class Server(http.server.ThreadingHTTPServer):
        daemon_threads = False
        requests = 0
        active = 0
        lock = threading.Lock()
        document = None

    class Handler(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def setup(self):
            super().setup()
            self.connection.settimeout(5)
            with self.server.lock:
                self.server.active += 1

        def finish(self):
            try:
                super().finish()
            finally:
                with self.server.lock:
                    self.server.active -= 1

        def log_message(self, *_):
            pass

        def do_GET(self):
            expected = "Basic " + base64.b64encode(":".join(credentials).encode()).decode()
            with self.server.lock:
                self.server.requests += 1
            # Authentication and the peer's exact-path authorization are separate.
            if self.headers.get_all("Authorization") != [expected]:
                self.send_response(401)
                self.send_header("WWW-Authenticate", 'Basic realm="fixture"')
                body = b""
            elif self.path != path:
                self.send_response(403)
                body = b""
            else:
                self.send_response(200)
                body = json.dumps(self.server.document).encode()
            self.send_header("Content-Type", media)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    server = Server(("127.0.0.1", 0), Handler)
    server.socket = context.wrap_socket(server.socket, server_side=True)
    thread = threading.Thread(target=server.serve_forever)
    thread.start()
    try:
        yield server
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
        assert not thread.is_alive()
        assert server.active == 0


def released(*peers):
    end = time.monotonic() + 3
    while any(peer.active for peer in peers) and time.monotonic() < end:
        time.sleep(0.01)
    assert all(peer.active == 0 for peer in peers), "client TLS socket leaked"


def main():
    subprocess.run(
        ["cargo", "build", "--locked", "--offline", "-p", "clinkz-wot-discovery",
         "--example", "secure_discover_read"], cwd=REPO, check=True,
    )
    with tempfile.TemporaryDirectory(prefix="clinkz-secure-peers-") as tmp:
        directory_context, directory_root = certificate(Path(tmp), "directory")
        thing_context, thing_root = certificate(Path(tmp), "thing")
        with peer(directory_context, DIRECTORY_AUTH, "/things?limit=16", "application/ld+json") as directory, \
             peer(thing_context, THING_AUTH, "/level", "application/json") as thing:
            target = f"https://127.0.0.1:{thing.server_port}/level"
            td = {
                "@context": "https://www.w3.org/2022/wot/td/v1.1",
                "id": "urn:external:secure", "title": "external secure sensor",
                "security": "basic", "securityDefinitions": {"basic": {"scheme": "basic"}},
                "properties": {"level": {
                    "type": "object", "readOnly": True, "required": ["temperature", "label"],
                    "properties": {
                        "temperature": {"type": "integer", "minimum": 0, "maximum": 50},
                        "label": {"type": "string", "minLength": 1, "maxLength": 8},
                    },
                    "forms": [{"href": target}, {"href": target}],
                }},
            }
            directory.document = [td]
            thing.document = {"temperature": 23, "label": "office"}
            env = dict(os.environ)
            env.update(CLINKZ_DIRECTORY_USER=DIRECTORY_AUTH[0], CLINKZ_DIRECTORY_PASSWORD=DIRECTORY_AUTH[1],
                       CLINKZ_THING_USER=THING_AUTH[0], CLINKZ_THING_PASSWORD=THING_AUTH[1])
            args = ["cargo", "run", "--quiet", "--locked", "--offline", "-p", "clinkz-wot-discovery",
                    "--example", "secure_discover_read", "--",
                    f"https://127.0.0.1:{directory.server_port}/things", f"127.0.0.1:{directory.server_port}",
                    "urn:external:secure", "level", "1", target, str(directory_root), str(thing_root)]

            def run(success):
                result = subprocess.run(args, cwd=REPO, env=env, capture_output=True, text=True, timeout=30)
                assert (result.returncode == 0) == success, result.stderr
                released(directory, thing)
                return result

            result = run(True)
            assert json.loads(result.stdout) == thing.document
            assert directory.requests == thing.requests == 1
            # A usable earlier Form cannot replace the explicitly selected one.
            td["properties"]["level"]["forms"][1]["href"] = target + "/denied"
            run(False)
            assert directory.requests == 2 and thing.requests == 1
            td["properties"]["level"]["forms"][1]["href"] = target
            args[-1] = str(directory_root)
            run(False)
            assert directory.requests == 3 and thing.requests == 1
            args[-1] = str(thing_root)
            thing.document["temperature"] = 51
            run(False)
            assert directory.requests == 4 and thing.requests == 2
            print("PASS: independent TLS/Basic peers, exact Form refusal, separate roots, DataSchema refusal, final sockets released")


if __name__ == "__main__":
    main()
