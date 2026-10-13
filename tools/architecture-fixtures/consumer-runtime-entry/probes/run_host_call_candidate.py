#!/usr/bin/env python3
"""Reproduce the admission alternatives in a disposable historical snapshot."""
import argparse
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--baseline", action="store_true")
parser.add_argument("--alternative", choices=["inline", "preallocated"], default="preallocated")
parser.add_argument("--toolchain", default="stable")
parser.add_argument("--source-revision", default="988acf3",
                    help="pre-implementation source boundary (default: admitted master)")
args = parser.parse_args()
root = Path(__file__).resolve().parents[4]
probe = Path(__file__).resolve().parent
revision = subprocess.check_output(["git", "rev-parse", args.source_revision + "^{commit}"], cwd=root, text=True).strip()
print("source_revision=" + revision, flush=True)
print(subprocess.check_output(["rustc", f"+{args.toolchain}", "--version"], text=True).strip(), flush=True)

with tempfile.TemporaryDirectory(prefix="clinkz-host-call-") as directory:
    snapshot = Path(directory)
    archive = subprocess.check_output(["git", "archive", revision], cwd=root)
    with tarfile.open(fileobj=io.BytesIO(archive)) as files:
        files.extractall(snapshot, filter="data")
    if not args.baseline:
        patch = probe / ("host_call_candidate.patch" if args.alternative == "inline" else "host_call_preallocated.patch")
        subprocess.run(["git", "apply", "--check", str(patch)], cwd=snapshot, check=True)
        subprocess.run(["git", "apply", str(patch)], cwd=snapshot, check=True)
    tests = snapshot / "core/tests/consumer_property_read_binding.rs"
    with tests.open("a") as output:
        appendix = (probe / "host_call_candidate.rs").read_text()
        construct = "Ok(HostBindingCallBox::new(call))" if args.baseline else "HostBindingCallBox::try_new(call)"
        output.write("\n" + appendix.replace("HOST_CALL_CONSTRUCT", construct))
    env = dict(os.environ)
    env["CLINKZ_HOST_CALL_CANDIDATE"] = "baseline" if args.baseline else args.alternative
    # Reuse only compilation artifacts; source and mutations stay disposable.
    env["CARGO_TARGET_DIR"] = str(root / "target/host-call-candidate")
    command = ["cargo", f"+{args.toolchain}"]
    subprocess.run(command + ["test", "--locked", "--offline", "-p", "clinkz-wot-core",
                             "--test", "consumer_property_read_binding", "--test",
                             "consumer_compiler_support", "--", "--nocapture"],
                   cwd=snapshot, env=env, check=True)
    if not args.baseline:
        for features in [[], ["--features", "async"]]:
            subprocess.run(command + ["check", "--locked", "--offline", "-p",
                                     "clinkz-wot-core", "--no-default-features"] + features,
                           cwd=snapshot, env=env, check=True)
