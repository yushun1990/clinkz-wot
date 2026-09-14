#!/usr/bin/env python3
"""Independent Cargo cells: dependency-feature unification is the invariant."""
import argparse
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--offline", action="store_true")
args = parser.parse_args()
common = ["--locked", "--manifest-path", str(Path(__file__).with_name("Cargo.toml"))]
if args.offline:
    common.append("--offline")

def run(command, negative=False):
    r = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if negative:
        assert r.returncode and "error[E0432]" in r.stdout and "configured out" in r.stdout, r.stdout
    elif r.returncode:
        raise RuntimeError(" ".join(command) + "\n" + r.stdout)
    return r.stdout

def cell(package, features, target=None, no_default=False, negative=False):
    options = [*common, "-p", package]
    if features:
        options += ["--features", ",".join(features)]
    if no_default:
        options.append("--no-default-features")
    if target:
        options += ["--target", target]
    mode = "check" if target or package.endswith("surface") else "test"
    run(["cargo", mode, *options], negative)
    graph = run(["cargo", "tree", *options, "-e", "normal", "--prefix", "none", "-f", "{p}|{f}"])
    def resolved(name):
        rows = [line.split("|", 1)[1].removesuffix(" (*)") for line in graph.splitlines()
                if line.startswith(name + " v")]
        assert rows, graph
        return set(rows[0].split(",")) - {""}
    serde = resolved("serde_json")
    local = resolved("number-boundary-td-prototype")
    capability = "validated-thing" in features or "capability" in features
    assert ("validated-thing" in local) == capability, graph
    assert ("arbitrary_precision" in serde) == (capability or "ap" in features), graph
    assert ("preserve_order" in serde) == ("order" in features), graph
    if target or (no_default and "order" not in features):
        assert "std" not in serde and "std" not in local, graph
        assert "std" not in resolved("clinkz-wot-td"), graph
    print(f"PASS {package} {target or 'host'} defaults={not no_default} "
          f"features={','.join(features) or '-'} {'E0432' if negative else mode}; "
          f"serde_json={','.join(sorted(serde))}", flush=True)

for features in [[], ["order"], ["ap"], ["ap", "order"]]:
    for capability in [[], ["validated-thing"]]:
        cell("number-boundary-downstream", features + capability)
for features in [[], ["async"]]:
    for capability in [[], ["validated-thing"]]:
        cell("number-boundary-downstream", features + capability, no_default=True)
target = "thumbv7em-none-eabihf"
for features in [[], ["ap"], ["async"], ["async", "ap"]]:
    for capability in [[], ["validated-thing"]]:
        cell("number-boundary-downstream", features + capability, target, True)
# Import is absent under serde-only unification, present under sibling TD activation.
for features in [[], ["ap"], ["order"], ["ap", "order"]]:
    cell("number-boundary-surface", features, negative=True)
    cell("number-boundary-surface", features + ["capability"])
for features in [[], ["ap"]]:
    cell("number-boundary-surface", features, target, True, negative=True)
    cell("number-boundary-surface", features + ["capability"], target, True)
print("32 independent feature cells passed; no readmission or target-runtime claim.")
