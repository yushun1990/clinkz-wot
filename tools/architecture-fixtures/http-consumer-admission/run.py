#!/usr/bin/env python3
"""Compare a pinned public Hyper graph with a disposable closed-interest repair."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

fixture = Path(__file__).resolve().parent
root = fixture.parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--variant", choices=["upstream", "closed-interest", "all", "installed"], default="all")
parser.add_argument("--installed-source", type=Path,
                    default=root / "protocol-bindings/dependencies/hyper-1.12.0")
args = parser.parse_args()


def run(command, *, cwd=fixture, env=None):
    subprocess.run(command, cwd=cwd, env=env, check=True)


def identities(lock):
    return {(p["name"], p["version"]): (p.get("source"), p.get("checksum"))
            for p in tomllib.loads(lock.read_text())["package"] if p["name"] != "hyper"}


environment = dict(os.environ)
environment.pop("HTTP_DISPOSAL_PATCH", None)
environment["CARGO_INCREMENTAL"] = "0"
environment["CARGO_TARGET_DIR"] = str(root / "target/http-consumer-admission")
test = ["cargo", "test", "--locked", "--offline", "--test", "connection", "--",
        "--test-threads=1", "--nocapture"]
run(["rustc", "--version"])
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], cwd=fixture))
package = next(p for p in metadata["packages"] if p["name"] == "hyper" and p["version"] == "1.12.0")
assert package["source"].startswith("registry+")
source = Path(package["manifest_path"]).parent
lock_package = next(p for p in tomllib.loads((fixture / "Cargo.lock").read_text())["package"]
                    if p["name"] == "hyper")
archive = source.parents[2] / "cache" / source.parent.name / "hyper-1.12.0.crate"
assert hashlib.sha256(archive.read_bytes()).hexdigest() == lock_package["checksum"]
with tarfile.open(archive) as files:
    for member in files.getmembers():
        if member.isfile():
            relative = member.name.split("/", 1)[1]
            assert (source / relative).read_bytes() == files.extractfile(member).read(), relative
print("verified Hyper 1.12.0 archive checksum and unmodified registry files", flush=True)
if args.variant in ("upstream", "all"):
    print("=== exact unmodified upstream; positive assertions reproduce the counterexample ===", flush=True)
    run(test, env=environment)
if args.variant == "upstream":
    raise SystemExit(0)

with tempfile.TemporaryDirectory(prefix="clinkz-http-disposal-") as directory:
    snapshot = Path(directory)
    with tarfile.open(archive) as files:
        files.extractall(snapshot, filter="data")
    (snapshot / "hyper-1.12.0").rename(snapshot / "hyper")
    shutil.copytree(fixture / "tests", snapshot / "tests")
    shutil.copyfile(fixture / "Cargo.lock", snapshot / "Cargo.lock")
    run(["git", "apply", "--check", str(fixture / "closed-interest.patch")], cwd=snapshot / "hyper")
    run(["git", "apply", str(fixture / "closed-interest.patch")], cwd=snapshot / "hyper")
    selected = snapshot / "hyper"
    if args.variant == "installed":
        selected = args.installed_source.resolve()
        if not (selected / "Cargo.toml").is_file():
            parser.error("installed source does not exist; this is a post-admission completion check")
        expected = {p.relative_to(snapshot / "hyper") for p in (snapshot / "hyper").rglob("*") if p.is_file()}
        actual = {p.relative_to(selected) for p in selected.rglob("*") if p.is_file() and "target" not in p.relative_to(selected).parts}
        assert actual == expected, "installed dependency file set differs from admitted source"
        for name in expected:
            assert (selected / name).read_bytes() == (snapshot / "hyper" / name).read_bytes(), name
    (snapshot / "Cargo.toml").write_text((fixture / "Cargo.toml").read_text() +
                                       '\n[patch.crates-io]\nhyper = { path = ' + json.dumps(str(selected)) + ' }\n')
    # The only resolution change permitted is the explicitly patched Hyper source.
    run(["cargo", "generate-lockfile", "--offline"], cwd=snapshot)
    assert identities(snapshot / "Cargo.lock") == identities(fixture / "Cargo.lock")
    resolved = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], cwd=snapshot))
    actual_hyper = [p for p in resolved["packages"] if p["name"] == "hyper"]
    assert len(actual_hyper) == 1
    assert Path(actual_hyper[0]["manifest_path"]) == selected / "Cargo.toml"
    environment["HTTP_DISPOSAL_PATCH"] = "1"
    label = "installed source" if args.variant == "installed" else "disposable closed-interest repair"
    print(f"=== {label}, identical dependency versions/features; source={selected} ===", flush=True)
    run(test, cwd=snapshot, env=environment)
