#!/usr/bin/env python3
"""Independent Cargo cells: dependency-feature unification is the invariant."""
import argparse
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--offline", action="store_true")
args = parser.parse_args()
common = ["--locked", "--manifest-path", str(Path(__file__).with_name("matrix") / "Cargo.toml")]
if args.offline:
    common.append("--offline")

def run(command, negative=False):
    r = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if negative:
        assert r.returncode and "error[E0432]" in r.stdout, r.stdout
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
    mode = "check"
    run(["cargo", mode, *options], negative)
    graph = run(["cargo", "tree", *options, "-e", "normal", "--prefix", "none", "-f", "{p}|{f}"])
    def resolved(name):
        rows = [line.split("|", 1)[1].removesuffix(" (*)") for line in graph.splitlines()
                if line.startswith(name + " v")]
        assert rows, graph
        return set(rows[0].split(",")) - {""}
    serde = resolved("serde_json")
    local = resolved("consumer-borrowed-readmission-probe")
    capability = "validated-thing" in features or "capability" in features
    assert ("validated-thing" in local) == capability, graph
    assert ("arbitrary_precision" in serde) == (capability or "ap" in features), graph
    assert ("preserve_order" in serde) == ("order" in features), graph
    if target or (no_default and "order" not in features):
        assert "std" not in serde and "std" not in local, graph
        assert "std" not in resolved("validated-thing-schema-kernel-probe"), graph
    print(f"PASS {package} {target or 'host'} defaults={not no_default} "
          f"features={','.join(features) or '-'} {'E0432' if negative else mode}; "
          f"serde_json={','.join(sorted(serde))}", flush=True)

for features in [[], ["order"], ["ap"], ["ap", "order"]]:
    for capability in [[], ["validated-thing"]]:
        cell("readmission-downstream", features + capability)
for features in [[], ["async"]]:
    for capability in [[], ["validated-thing"]]:
        cell("readmission-downstream", features + capability, no_default=True)
target = "thumbv7em-none-eabihf"
for features in [[], ["ap"], ["async"], ["async", "ap"]]:
    for capability in [[], ["validated-thing"]]:
        cell("readmission-downstream", features + capability, target, True)
# Import is absent under serde-only unification, present under sibling TD activation.
for features in [[], ["ap"], ["order"], ["ap", "order"]]:
    cell("readmission-surface", features, negative=True)
    cell("readmission-surface", features + ["capability"])
for features in [[], ["ap"]]:
    cell("readmission-surface", features, target, True, negative=True)
    cell("readmission-surface", features + ["capability"], target, True)
print("32 exact-boundary feature cells passed.")
negative = Path(__file__).with_name("matrix") / "negative" / "src" / "lib.rs"
header = "#![no_std]\nuse candidate::*;\nuse clinkz_wot_foundation::{AdmissionLedger,WorkBudget};\n"
cases = {
 "raw-aggregate": ("E0308", "fn reject(t:&thing::Thing,r:&planning::registration::Registration,c:&planning::Calls) {let _=planning::Build::new(t,r,c,1000000,100000,None);}"),
 "forged-proof": ("E0423", "fn reject() {let _=ValidatedThing(todo!());}"),
 "forged-config": ("E0451", "fn reject() {let _=ValidatedThingAdmissionConfig{policy:todo!()};}"),
 "ready-loan-move": ("E0505", "fn reject(mut r:ValidatedPropertyReadCursor<'_>,b:&mut WorkBudget) {let loan=r.step(b,false);let moved=r;drop(loan);drop(moved);}"),
 "ready-loan-acknowledge": ("E0499", "fn reject(mut r:ValidatedPropertyReadCursor<'_>,b:&mut WorkBudget) {let loan=r.step(b,false);r.acknowledge();drop(loan);}"),
 "caller-destruction": ("E0505", "fn reject(t:thing::Thing,c:&ValidatedThingAdmissionConfig,l:AdmissionLedger) {let cursor=ValidatedThingCursor::from_thing(&t,c,l);drop(t);drop(cursor);}"),
 "registration-destruction": ("E0505", "fn reject(p:ValidatedThing<'_>,r:planning::registration::Registration,c:&planning::Calls) {let build=planning::Build::new(p,&r,c,1000000,100000,None);drop(r);drop(build);}"),
 "clone-cursor": ("E0599", "fn reject(v:ValidatedThingCursor<'_>) {let _=v.clone();}"),
 "clone-proof": ("E0599", "fn reject(v:ValidatedThing<'_>) {let _=v.clone();}"),
 "clone-semantic": ("E0599", "fn reject(v:ValidatedPropertyReadCursor<'_>) {let _=v.clone();}"),
 "clone-ledger": ("E0599", "fn reject(v:AdmissionLedger) {let _=v.clone();}"),
 "clone-budget": ("E0599", "fn reject(v:WorkBudget) {let _=v.clone();}"),
 "obsolete-snapshot": ("E0432", "use candidate::ValidatedThingView;"),
}
# The aggregate negative must derive its proof from the same local caller.
cases["aggregate-input-destruction"]=("E0505", "fn reject(t:thing::Thing,cfg:&ValidatedThingAdmissionConfig,l:AdmissionLedger,r:&planning::registration::Registration,c:&planning::Calls,b:&mut WorkBudget) {let cursor=ValidatedThingCursor::from_thing(&t,cfg,l);let ValidatedThingProgress::Complete(p)=cursor.step(b,false) else {return};let build=planning::Build::new(p,r,c,1000000,100000,None);drop(t);drop(build);}")
try:
 for name,(code,source) in cases.items():
  negative.write_text(header+source+"\n")
  command=["cargo","check",*common,"-p","readmission-negative"]
  r=subprocess.run(command,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  assert r.returncode and f"error[{code}]" in r.stdout,name+"\n"+r.stdout
  print(f"PASS compile-negative {name} {code}",flush=True)
finally:
 negative.write_text("#![no_std]\n")
print(f"{len(cases)} exact expected-diagnostic negative fixtures passed.")
